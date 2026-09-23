//! Global package command surface.
//!
//! These commands install and run published package command-line entry points
//! (`[[bin]]`) from the shared Vut home (`~/.vut`):
//!
//! ```text
//! vpm install <package>[@version]   install a published CLI package
//! vpm uninstall <package>           remove an installed CLI package
//! vpm exec <package> [--bin <name>] run a published CLI package
//! ```
//!
//! Global commands live in `~/.vut/bin`; ad-hoc `exec` builds live under
//! `~/.vut/cache/exec`.

mod build;
mod registry;

use crate::{Manifest, PackageId, PackageSource, ResolvedGraph};
use registry::{Installed, Registry};
use semver::Version;

/// Outcome of `vpm install <package>`.
pub struct GlobalInstallReport {
    pub package: String,
    pub bins: Vec<String>,
}

/// Installs a published CLI package globally.
pub fn install(spec: &str) -> Result<GlobalInstallReport, Box<dyn std::error::Error>> {
    let (source, requested) = parse_spec(spec)?;
    let graph = resolve(&source, requested)?;
    let store = crate::store::Store::global()?;
    store.install(&graph)?;
    let package = root_package(&graph, &source)?.clone();
    let manifest = manifest_of(&graph, &package)?;
    if manifest.bin.is_empty() {
        return Err(format!("package `{}` has no CLI `[[bin]]` entries", package.name).into());
    }

    let bin_dir = store.bin_dir();
    let mut registry = Registry::load(&bin_dir)?;
    for bin in &manifest.bin {
        if let Some(owner) = registry.owner_of(&bin.name)
            && owner != package.name
        {
            return Err(format!(
                "cannot install `{}`: bin `{}` is already provided by package `{owner}`",
                package.name, bin.name
            )
            .into());
        }
    }

    let mut bins = Vec::with_capacity(manifest.bin.len());
    for bin in &manifest.bin {
        let output = bin_dir.join(executable_name(&bin.name));
        build::build_bin(&store, &graph, &package, &manifest, bin, &output)?;
        bins.push(bin.name.clone());
    }
    registry.upsert(Installed {
        name: package.name.clone(),
        version: package.version.to_string(),
        source: package.source.identity(),
        bins: bins.clone(),
    });
    registry.save(&bin_dir)?;
    Ok(GlobalInstallReport {
        package: package.name,
        bins,
    })
}

/// Removes a globally installed CLI package and its binaries.
pub fn uninstall(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let store = crate::store::Store::global()?;
    let bin_dir = store.bin_dir();
    let mut registry = Registry::load(&bin_dir)?;
    let entry = registry
        .remove(name)
        .ok_or_else(|| format!("package `{name}` is not installed globally"))?;
    for bin in &entry.bins {
        let path = bin_dir.join(executable_name(bin));
        if path.is_file() {
            std::fs::remove_file(path)?;
        }
    }
    registry.save(&bin_dir)?;
    Ok(())
}

/// Runs a published CLI package without installing it globally.
///
/// Returns the process exit code.
pub fn exec(
    spec: &str,
    bin: Option<&str>,
    args: &[String],
) -> Result<i32, Box<dyn std::error::Error>> {
    let (source, requested) = parse_spec(spec)?;
    let graph = resolve(&source, requested)?;
    let store = crate::store::Store::global()?;
    store.install(&graph)?;
    let package = root_package(&graph, &source)?.clone();
    let manifest = manifest_of(&graph, &package)?;
    let selected = select_bin(&manifest.bin, &package.name, bin)?;
    let output = store
        .cache_dir()
        .join("exec")
        .join(build::component(&package.name))
        .join(build::component(&package.version.to_string()))
        .join("bins")
        .join(executable_name(&selected.name));
    build::build_bin(&store, &graph, &package, &manifest, selected, &output)?;
    let status = std::process::Command::new(&output).args(args).status()?;
    Ok(status.code().unwrap_or(1))
}

fn resolve(
    source: &PackageSource,
    requested: Option<String>,
) -> Result<ResolvedGraph, Box<dyn std::error::Error>> {
    let providers = crate::resolver::Providers::new()?;
    Ok(crate::Resolver::new(&providers).resolve(&[(source.clone(), requested)])?)
}

fn root_package<'a>(
    graph: &'a ResolvedGraph,
    source: &PackageSource,
) -> Result<&'a PackageId, Box<dyn std::error::Error>> {
    graph
        .packages
        .keys()
        .find(|id| id.source == *source)
        .ok_or_else(|| "resolved package is missing".into())
}

fn manifest_of(
    graph: &ResolvedGraph,
    id: &PackageId,
) -> Result<Manifest, Box<dyn std::error::Error>> {
    let resolved = graph
        .packages
        .get(id)
        .ok_or("resolved package is missing")?;
    let bytes = resolved
        .snapshot
        .files
        .get("vpm.toml")
        .ok_or("package has no vpm.toml")?;
    let manifest: Manifest = toml::from_str(std::str::from_utf8(bytes)?)?;
    manifest.validate()?;
    Ok(manifest)
}

fn select_bin<'a>(
    bins: &'a [crate::manifest::Bin],
    package: &str,
    requested: Option<&str>,
) -> Result<&'a crate::manifest::Bin, Box<dyn std::error::Error>> {
    if bins.is_empty() {
        return Err(format!("package `{package}` has no CLI `[[bin]]` entries").into());
    }
    let available = bins
        .iter()
        .map(|bin| bin.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if let Some(requested) = requested {
        return bins
            .iter()
            .find(|bin| bin.name == requested)
            .ok_or_else(|| {
                format!("package `{package}` has no bin `{requested}`; available: {available}")
                    .into()
            });
    }
    if let Some(bin) = bins.iter().find(|bin| bin.name == package) {
        return Ok(bin);
    }
    if bins.len() == 1 {
        return Ok(&bins[0]);
    }
    Err(format!("package `{package}` has multiple bins; select one with --bin: {available}").into())
}

fn parse_spec(spec: &str) -> Result<(PackageSource, Option<String>), Box<dyn std::error::Error>> {
    let (source, requested) = spec
        .rsplit_once('@')
        .map_or((spec, None), |(source, version)| (source, Some(version)));
    if let Some(value) = requested
        && value != "latest"
    {
        Version::parse(value)?;
    }
    Ok((PackageSource::parse(source)?, requested.map(str::to_owned)))
}

fn executable_name(bin: &str) -> String {
    if cfg!(windows) {
        format!("{bin}.exe")
    } else {
        bin.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bin(name: &str) -> crate::manifest::Bin {
        crate::manifest::Bin {
            name: name.into(),
            path: format!("src/bin/{name}.vut"),
        }
    }

    #[test]
    fn parses_registry_and_hosted_specs() {
        let (source, version) = parse_spec("math@1.2.0").unwrap();
        assert!(matches!(source, PackageSource::Registry { .. }));
        assert_eq!(version.as_deref(), Some("1.2.0"));
        let (source, version) = parse_spec("alice/vut-packages/math").unwrap();
        assert_eq!(source.name(), "math");
        assert!(version.is_none());
        assert!(parse_spec("math@nope").is_err());
    }

    #[test]
    fn bin_selection_prefers_name_then_sole_bin() {
        let bins = [bin("tool"), bin("helper")];
        assert_eq!(
            select_bin(&bins, "math", Some("helper")).unwrap().name,
            "helper"
        );
        assert!(select_bin(&bins, "math", None).is_err());
        assert!(select_bin(&bins, "math", Some("missing")).is_err());

        let named = [bin("math"), bin("helper")];
        assert_eq!(select_bin(&named, "math", None).unwrap().name, "math");

        let single = [bin("only")];
        assert_eq!(select_bin(&single, "math", None).unwrap().name, "only");
    }
}
