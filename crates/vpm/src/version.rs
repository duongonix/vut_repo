use semver::Version;

pub fn select(names: &[String], requested: Option<&str>) -> Result<Version, String> {
    let mut versions: Vec<_> = names
        .iter()
        .filter_map(|name| name.strip_prefix('v'))
        .filter_map(|value| Version::parse(value).ok())
        .collect();
    versions.sort();
    versions.dedup();
    match requested.unwrap_or("latest") {
        "latest" => versions
            .into_iter()
            .rfind(|version| version.pre.is_empty())
            .ok_or_else(|| "package has no valid stable versions".into()),
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
        let names = ["v0.9.0", "v0.10.0", "v2.0.0-alpha.1", "v1", "README"].map(str::to_owned);
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
}
