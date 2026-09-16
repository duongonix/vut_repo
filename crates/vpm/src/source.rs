use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ProviderKind {
    Registry,
    GitHub,
    GitLab,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PackageSource {
    Registry {
        name: String,
    },
    Hosted {
        provider: ProviderKind,
        owner: String,
        repository: String,
        path: Vec<String>,
    },
}

impl PackageSource {
    pub fn parse(value: &str) -> Result<Self, String> {
        let (provider, value) = value
            .strip_prefix("gitlab:")
            .map_or((ProviderKind::GitHub, value), |v| (ProviderKind::GitLab, v));
        if !value.contains('/') {
            crate::manifest::validate_name(value)?;
            return Ok(Self::Registry { name: value.into() });
        }
        let parts: Vec<_> = value.split('/').collect();
        if parts.len() < 3
            || parts
                .iter()
                .any(|part| part.is_empty() || *part == "." || *part == "..")
        {
            return Err(format!(
                "invalid hosted source `{value}`; expected owner/repository/package"
            ));
        }
        let path = parts[2..].iter().map(|part| (*part).to_owned()).collect();
        Ok(Self::Hosted {
            provider,
            owner: parts[0].into(),
            repository: parts[1].into(),
            path,
        })
    }
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Registry { name } => name,
            Self::Hosted { path, .. } => path.last().map_or("", String::as_str),
        }
    }
    #[must_use]
    pub fn provider(&self) -> ProviderKind {
        match self {
            Self::Registry { .. } => ProviderKind::Registry,
            Self::Hosted { provider, .. } => *provider,
        }
    }
    #[must_use]
    pub fn remote_path(&self) -> String {
        match self {
            Self::Registry { name } => name.clone(),
            Self::Hosted { path, .. } => path.join("/"),
        }
    }
    #[must_use]
    pub fn identity(&self) -> String {
        self.to_string()
    }
}
impl fmt::Display for PackageSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Registry { name } => write!(f, "registry:{name}"),
            Self::Hosted {
                provider,
                owner,
                repository,
                path,
            } => write!(
                f,
                "{}:{owner}/{repository}/{}",
                match provider {
                    ProviderKind::GitLab => "gitlab",
                    _ => "github",
                },
                path.join("/")
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_all_source_forms_and_rejects_repo_root() {
        assert!(matches!(
            PackageSource::parse("math").unwrap(),
            PackageSource::Registry { .. }
        ));
        assert_eq!(
            PackageSource::parse("a/b/libs/math").unwrap().name(),
            "math"
        );
        assert_eq!(
            PackageSource::parse("gitlab:a/b/math").unwrap().provider(),
            ProviderKind::GitLab
        );
        assert!(PackageSource::parse("a/b").is_err());
        assert!(PackageSource::parse("a/b/../math").is_err());
    }
}
