//! GitHub-backed publisher.
//!
//! Submission creates a branch, commits the package source under
//! `<package>/<version>/`, and opens a pull request for review. Credentials are
//! read from the environment (`VPM_TOKEN` or `GITHUB_TOKEN`) and are never
//! written to the manifest, lockfile, or package source.

use super::{PublishPlan, Publisher};
use base64::Engine as _;
use reqwest::blocking::Client;

pub struct GitHubPublisher {
    client: Client,
    api: String,
    token: String,
}

impl GitHubPublisher {
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let token = std::env::var("VPM_TOKEN")
            .or_else(|_| std::env::var("GITHUB_TOKEN"))
            .map_err(|_| "publishing requires a provider token in VPM_TOKEN or GITHUB_TOKEN")?;
        let client = Client::builder()
            .user_agent(concat!("vpm/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            client,
            api: "https://api.github.com".into(),
            token,
        })
    }

    fn get(&self, url: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = self
            .client
            .get(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()?;
        Self::value(response)
    }

    fn post(
        &self,
        url: &str,
        body: &serde_json::Value,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.token)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .json(body)
            .send()?;
        Self::value(response)
    }

    fn value(
        response: reqwest::blocking::Response,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(format!(
                "provider returned HTTP {}: {}",
                status.as_u16(),
                body.trim()
            )
            .into());
        }
        Ok(response.json()?)
    }
}

impl Publisher for GitHubPublisher {
    fn submit(&self, plan: &PublishPlan) -> Result<(), Box<dyn std::error::Error>> {
        let repo = format!("{}/{}", plan.owner, plan.repository);
        let base = format!("{}/repos/{repo}", self.api);

        let default_branch = self.get(&base)?["default_branch"]
            .as_str()
            .ok_or("provider response has no default branch")?
            .to_owned();
        let head_ref = self.get(&format!("{base}/git/ref/heads/{default_branch}"))?;
        let parent = head_ref["object"]["sha"]
            .as_str()
            .ok_or("provider response has no head commit")?
            .to_owned();
        let base_tree = self.get(&format!("{base}/git/commits/{parent}"))?["tree"]["sha"]
            .as_str()
            .ok_or("provider response has no base tree")?
            .to_owned();

        let mut tree = Vec::with_capacity(plan.files.len());
        for (relative, data) in &plan.files {
            let blob = self.post(
                &format!("{base}/git/blobs"),
                &serde_json::json!({
                    "content": base64::engine::general_purpose::STANDARD.encode(data),
                    "encoding": "base64",
                }),
            )?;
            tree.push(serde_json::json!({
                "path": format!("{}/{}/{}", plan.package, plan.version, relative),
                "mode": "100644",
                "type": "blob",
                "sha": blob["sha"],
            }));
        }
        let new_tree = self.post(
            &format!("{base}/git/trees"),
            &serde_json::json!({ "base_tree": base_tree, "tree": tree }),
        )?;
        let message = format!("Publish {} {}", plan.package, plan.version);
        let new_commit = self.post(
            &format!("{base}/git/commits"),
            &serde_json::json!({
                "message": message,
                "tree": new_tree["sha"],
                "parents": [parent],
            }),
        )?;
        let branch = format!("vpm/publish/{}-{}", plan.package, plan.version);
        self.post(
            &format!("{base}/git/refs"),
            &serde_json::json!({
                "ref": format!("refs/heads/{branch}"),
                "sha": new_commit["sha"],
            }),
        )?;
        self.post(
            &format!("{base}/pulls"),
            &serde_json::json!({
                "title": message,
                "head": branch,
                "base": default_branch,
                "body": "Automated VPM publish submission. Native artifacts are built and uploaded by trusted CI.",
            }),
        )?;
        Ok(())
    }
}
