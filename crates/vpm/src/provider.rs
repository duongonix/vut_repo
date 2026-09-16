use crate::{PackageSource, ProviderKind};
use base64::Engine as _;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use reqwest::blocking::Client;
use semver::Version;
use serde::Deserialize;
use std::{collections::BTreeMap, fmt};

#[derive(Clone, Debug)]
pub struct PackageSnapshot {
    pub files: BTreeMap<String, Vec<u8>>,
    pub revision: String,
}

#[derive(Debug)]
pub enum ProviderError {
    NotFound(String),
    PermissionDenied,
    RateLimited,
    Network(String),
    InvalidResponse(String),
    Integrity(String),
}
impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(v) => write!(f, "not found: {v}"),
            Self::PermissionDenied => f.write_str("provider permission denied"),
            Self::RateLimited => f.write_str("provider rate limit exceeded"),
            Self::Network(v) => write!(f, "network failure: {v}"),
            Self::InvalidResponse(v) => write!(f, "invalid provider response: {v}"),
            Self::Integrity(v) => write!(f, "package integrity failure: {v}"),
        }
    }
}
impl std::error::Error for ProviderError {}

pub trait PackageProvider {
    fn list_versions(&self, source: &PackageSource) -> Result<Vec<String>, ProviderError>;
    fn fetch(
        &self,
        source: &PackageSource,
        version: &Version,
    ) -> Result<PackageSnapshot, ProviderError>;
}

fn client() -> Result<Client, ProviderError> {
    Client::builder()
        .user_agent(concat!("vpm/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| ProviderError::Network(e.to_string()))
}
fn checked(
    response: reqwest::blocking::Response,
) -> Result<reqwest::blocking::Response, ProviderError> {
    match response.status().as_u16() {
        200..=299 => Ok(response),
        401 | 403
            if response
                .headers()
                .get("x-ratelimit-remaining")
                .is_some_and(|v| v == "0") =>
        {
            Err(ProviderError::RateLimited)
        }
        401 | 403 => Err(ProviderError::PermissionDenied),
        404 => Err(ProviderError::NotFound("remote package path".into())),
        status => Err(ProviderError::Network(format!(
            "provider returned HTTP {status}"
        ))),
    }
}
fn hosted(
    source: &PackageSource,
    expected: ProviderKind,
) -> Result<(&str, &str, String), ProviderError> {
    match source {
        PackageSource::Registry { name } if expected == ProviderKind::GitHub => {
            Ok(("duongonix", "vpm", name.clone()))
        }
        PackageSource::Hosted {
            provider,
            owner,
            repository,
            path,
        } if *provider == expected => Ok((owner, repository, path.join("/"))),
        _ => Err(ProviderError::InvalidResponse(
            "source/provider mismatch".into(),
        )),
    }
}

pub struct GitHubProvider {
    client: Client,
    api: String,
}
impl GitHubProvider {
    pub fn new() -> Result<Self, ProviderError> {
        Ok(Self {
            client: client()?,
            api: "https://api.github.com".into(),
        })
    }
    #[cfg(test)]
    fn at(api: String) -> Result<Self, ProviderError> {
        Ok(Self {
            client: client()?,
            api,
        })
    }
}
#[derive(Deserialize)]
struct GhContent {
    name: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
}
#[derive(Deserialize)]
struct GhTree {
    tree: Vec<GhTreeItem>,
}
#[derive(Deserialize)]
struct GhTreeItem {
    path: String,
    #[serde(rename = "type")]
    kind: String,
    sha: String,
}
#[derive(Deserialize)]
struct GhBlob {
    content: String,
    encoding: String,
}
impl PackageProvider for GitHubProvider {
    fn list_versions(&self, source: &PackageSource) -> Result<Vec<String>, ProviderError> {
        let (owner, repo, path) = hosted(source, ProviderKind::GitHub)?;
        let url = format!("{}/repos/{owner}/{repo}/contents/{path}", self.api);
        let values: Vec<GhContent> = checked(
            self.client
                .get(url)
                .send()
                .map_err(|e| ProviderError::Network(e.to_string()))?,
        )?
        .json()
        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        Ok(values
            .into_iter()
            .filter(|v| v.kind == "dir")
            .map(|v| v.name)
            .collect())
    }
    fn fetch(
        &self,
        source: &PackageSource,
        version: &Version,
    ) -> Result<PackageSnapshot, ProviderError> {
        let (owner, repo, path) = hosted(source, ProviderKind::GitHub)?;
        let version_name = format!("v{version}");
        let content_url = format!("{}/repos/{owner}/{repo}/contents/{path}", self.api);
        let roots: Vec<GhContent> = checked(
            self.client
                .get(content_url)
                .send()
                .map_err(|e| ProviderError::Network(e.to_string()))?,
        )?
        .json()
        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        let revision = roots
            .into_iter()
            .find(|entry| entry.kind == "dir" && entry.name == version_name)
            .map(|entry| entry.sha)
            .ok_or_else(|| ProviderError::NotFound(format!("{path}/{version_name}")))?;
        let tree_url = format!(
            "{}/repos/{owner}/{repo}/git/trees/{revision}?recursive=1",
            self.api
        );
        let tree: GhTree = checked(
            self.client
                .get(tree_url)
                .send()
                .map_err(|e| ProviderError::Network(e.to_string()))?,
        )?
        .json()
        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        let mut files = BTreeMap::new();
        for item in tree.tree.into_iter().filter(|v| v.kind == "blob") {
            validate_remote_path(&item.path)?;
            let url = format!("{}/repos/{owner}/{repo}/git/blobs/{}", self.api, item.sha);
            let blob: GhBlob = checked(
                self.client
                    .get(url)
                    .send()
                    .map_err(|e| ProviderError::Network(e.to_string()))?,
            )?
            .json()
            .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
            if blob.encoding != "base64" {
                return Err(ProviderError::InvalidResponse(
                    "GitHub blob is not base64".into(),
                ));
            }
            let compact: String = blob
                .content
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(compact)
                .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
            files.insert(item.path, bytes);
        }
        Ok(PackageSnapshot { files, revision })
    }
}

pub struct GitLabProvider {
    client: Client,
    api: String,
}
impl GitLabProvider {
    pub fn new() -> Result<Self, ProviderError> {
        Ok(Self {
            client: client()?,
            api: "https://gitlab.com/api/v4".into(),
        })
    }
    #[cfg(test)]
    fn at(api: String) -> Result<Self, ProviderError> {
        Ok(Self {
            client: client()?,
            api,
        })
    }
}
#[derive(Deserialize)]
struct GlTree {
    name: String,
    path: String,
    #[serde(rename = "type")]
    kind: String,
}
#[derive(Deserialize)]
struct GlCommit {
    id: String,
}
impl PackageProvider for GitLabProvider {
    fn list_versions(&self, source: &PackageSource) -> Result<Vec<String>, ProviderError> {
        let (owner, repo, path) = hosted(source, ProviderKind::GitLab)?;
        let project = encode(&format!("{owner}/{repo}"));
        let url = format!(
            "{}/projects/{project}/repository/tree?path={}&per_page=100",
            self.api,
            encode(&path)
        );
        let values: Vec<GlTree> = checked(
            self.client
                .get(url)
                .send()
                .map_err(|e| ProviderError::Network(e.to_string()))?,
        )?
        .json()
        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        Ok(values
            .into_iter()
            .filter(|v| v.kind == "tree")
            .map(|v| v.name)
            .collect())
    }
    fn fetch(
        &self,
        source: &PackageSource,
        version: &Version,
    ) -> Result<PackageSnapshot, ProviderError> {
        let (owner, repo, path) = hosted(source, ProviderKind::GitLab)?;
        let project = encode(&format!("{owner}/{repo}"));
        let root = format!("{path}/v{version}");
        let commit_url = format!("{}/projects/{project}/repository/commits/HEAD", self.api);
        let revision: GlCommit = checked(
            self.client
                .get(commit_url)
                .send()
                .map_err(|e| ProviderError::Network(e.to_string()))?,
        )?
        .json()
        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        let url = format!(
            "{}/projects/{project}/repository/tree?path={}&recursive=true&per_page=100&ref={}",
            self.api,
            encode(&root),
            encode(&revision.id)
        );
        let entries: Vec<GlTree> = checked(
            self.client
                .get(url)
                .send()
                .map_err(|e| ProviderError::Network(e.to_string()))?,
        )?
        .json()
        .map_err(|e| ProviderError::InvalidResponse(e.to_string()))?;
        if entries.is_empty() {
            return Err(ProviderError::NotFound(root.clone()));
        }
        let mut files = BTreeMap::new();
        for entry in entries.into_iter().filter(|v| v.kind == "blob") {
            let relative = entry
                .path
                .strip_prefix(&format!("{root}/"))
                .ok_or_else(|| ProviderError::Integrity("path escaped package root".into()))?;
            validate_remote_path(relative)?;
            let url = format!(
                "{}/projects/{project}/repository/files/{}/raw?ref={}",
                self.api,
                encode(&entry.path),
                encode(&revision.id)
            );
            let bytes = checked(
                self.client
                    .get(url)
                    .send()
                    .map_err(|e| ProviderError::Network(e.to_string()))?,
            )?
            .bytes()
            .map_err(|e| ProviderError::Network(e.to_string()))?
            .to_vec();
            files.insert(relative.into(), bytes);
        }
        Ok(PackageSnapshot {
            files,
            revision: revision.id,
        })
    }
}
fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}
fn validate_remote_path(path: &str) -> Result<(), ProviderError> {
    if path.is_empty()
        || path
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == ".." || p.contains('\\'))
    {
        Err(ProviderError::Integrity(format!(
            "unsafe remote path `{path}`"
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read as _, Write as _};
    #[test]
    fn github_version_listing_is_mockable_and_filters_files() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let count = stream.read(&mut request).unwrap();
            assert!(
                String::from_utf8_lossy(&request[..count])
                    .contains("/repos/duongonix/vpm/contents/math")
            );
            let body = r#"[{"name":"v0.9.0","type":"dir","sha":"a"},{"name":"README.md","type":"file","sha":"b"},{"name":"v0.10.0","type":"dir","sha":"c"}]"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        let provider = GitHubProvider::at(format!("http://{address}")).unwrap();
        let versions = provider
            .list_versions(&PackageSource::parse("math").unwrap())
            .unwrap();
        assert_eq!(versions, ["v0.9.0", "v0.10.0"]);
        server.join().unwrap();
    }
    #[test]
    fn gitlab_version_listing_uses_explicit_provider() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 2048];
            let count = stream.read(&mut request).unwrap();
            assert!(
                String::from_utf8_lossy(&request[..count])
                    .contains("/projects/group%2Frepo/repository/tree")
            );
            let body = r#"[{"name":"v1.0.0","path":"math/v1.0.0","type":"tree","id":"a"}]"#;
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
        });
        let provider = GitLabProvider::at(format!("http://{address}")).unwrap();
        let source = PackageSource::parse("gitlab:group/repo/math").unwrap();
        assert_eq!(provider.list_versions(&source).unwrap(), ["v1.0.0"]);
        server.join().unwrap();
    }
}
