use semver::Version;

/// Parses remote version-directory names into supported semantic versions.
///
/// Version directories are named exactly `<semver>` (for example `1.0.0/`),
/// with no `v` prefix. Malformed or non-version names are ignored.
#[must_use]
pub fn parse_versions(names: &[String]) -> Vec<Version> {
    let mut versions: Vec<_> = names
        .iter()
        .filter_map(|name| Version::parse(name).ok())
        .collect();
    versions.sort();
    versions.dedup();
    versions
}

/// The highest stable (non-prerelease) version in a sorted version list.
#[must_use]
pub fn latest_stable(versions: &[Version]) -> Option<Version> {
    versions
        .iter()
        .rev()
        .find(|version| version.pre.is_empty())
        .cloned()
}

pub fn select(names: &[String], requested: Option<&str>) -> Result<Version, String> {
    let versions = parse_versions(names);
    match requested.unwrap_or("latest") {
        "latest" => {
            latest_stable(&versions).ok_or_else(|| "package has no valid stable versions".into())
        }
        exact => {
            let version = Version::parse(exact)
                .map_err(|error| format!("invalid exact version `{exact}`: {error}"))?;
            versions
                .binary_search(&version)
                .map(|_| version.clone())
                .map_err(|_| format!("version `{version}` was not found"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn latest_is_semantic_stable_and_malformed_is_ignored() {
        let names = ["0.9.0", "0.10.0", "2.0.0-alpha.1", "1", "README"].map(str::to_owned);
        assert_eq!(
            select(&names, None).unwrap(),
            Version::parse("0.10.0").unwrap()
        );
        assert_eq!(
            select(&names, Some("2.0.0-alpha.1")).unwrap(),
            Version::parse("2.0.0-alpha.1").unwrap()
        );
        assert!(select(&names, Some("1.0.0")).is_err());
    }

    #[test]
    fn version_directories_with_a_v_prefix_are_not_versions() {
        let names = ["v1.0.0", "v0.9.0"].map(str::to_owned);
        assert!(parse_versions(&names).is_empty());
        assert!(select(&names, None).is_err());
    }
}
