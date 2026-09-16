use crate::{Dependency, Manifest, PackageSource};
use semver::Version;
use std::{
    fmt::Write as _,
    io,
    path::{Path, PathBuf},
};
use toml_edit::{DocumentMut, Item, Table, value};
use vut_compiler::{BuildMode, CompilerConfig, CompilerSession};

/// Outcome of `vpm add`, used by the CLI to report the resolved dependency.
pub struct AddReport {
    pub import_name: String,
    pub version: String,
    pub source: String,
}

/// Outcome of `vpm install` / `vpm update`.
pub struct InstallReport {
    pub installed: usize,
    pub total: usize,
    pub up_to_date: bool,
}

pub fn new(root: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if root.exists() {
        return Err(format!("destination `{}` already exists", root.display()).into());
    }
    std::fs::create_dir(root)?;
    if let Err(error) = init_named(root, name) {
        let _ = std::fs::remove_dir_all(root);
        return Err(error);
    }
    Ok(())
}
pub fn init(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let name = root
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or("current directory has no valid project name")?
        .to_owned();
    init_named(root, &name)?;
    Ok(name)
}
pub(crate) fn init_named(root: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    crate::manifest::validate_name(name)?;
    for file in ["vpm.toml", "vpm.lock"] {
        if root.join(file).exists() {
            return Err(format!("refusing to overwrite `{file}`").into());
        }
    }
    std::fs::create_dir_all(root.join("src"))?;
    std::fs::create_dir_all(root.join("tests"))?;
    atomic_write(
        &root.join("vpm.toml"),
        &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n[dependencies]\n"),
    )?;
    atomic_write(&root.join("vpm.lock"), "lock-version = 1\npackage = []\n")?;
    if !root.join("src/main.vut").exists() {
        atomic_write(
            &root.join("src/main.vut"),
            "fn main():\n  out(\"Hello from Vut\")\n",
        )?;
    }
    if !root.join(".gitignore").exists() {
        atomic_write(&root.join(".gitignore"), "/build/\n/.vpm/\n")?;
    }
    Ok(())
}
pub fn add(root: &Path, spec: &str) -> Result<AddReport, Box<dyn std::error::Error>> {
    let (source, requested) = spec
        .rsplit_once('@')
        .map_or((spec, None), |(s, v)| (s, Some(v)));
    if let Some(value) = requested
        && value != "latest"
    {
        Version::parse(value)?;
    }
    let package_source = PackageSource::parse(source)?;
    let name = package_source.name();
    crate::manifest::validate_name(name)?;
    let path = root.join("vpm.toml");
    let mut doc: DocumentMut = std::fs::read_to_string(&path)?.parse()?;
    if doc["dependencies"].get(name).is_some() {
        return Err(format!("dependency namespace `{name}` already exists").into());
    }
    let providers = crate::resolver::Providers::new()?;
    let graph = crate::Resolver::new(&providers)
        .resolve(&[(package_source.clone(), requested.map(str::to_owned))])?;
    let root_package = graph
        .packages
        .keys()
        .find(|id| id.source == package_source)
        .ok_or("resolved root package is missing")?;
    crate::store::Store::global()?.install(&graph)?;
    if source.contains('/') {
        let mut table = Table::new();
        table["source"] = value(source);
        table["version"] = value(root_package.version.to_string());
        doc["dependencies"][name] = Item::Table(table);
    } else {
        doc["dependencies"][name] = value(root_package.version.to_string());
    }
    atomic_write(&path, &doc.to_string())?;
    write_lock_graph(root, &graph)?;
    Ok(AddReport {
        import_name: name.to_owned(),
        version: root_package.version.to_string(),
        source: package_source.identity(),
    })
}
pub fn remove(root: &Path, name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let path = root.join("vpm.toml");
    let mut doc: DocumentMut = std::fs::read_to_string(&path)?.parse()?;
    if doc["dependencies"]
        .as_table_mut()
        .and_then(|t| t.remove(name))
        .is_none()
    {
        return Err(format!("dependency `{name}` is not present").into());
    }
    atomic_write(&path, &doc.to_string())?;
    update(root).map(|_| ())
}
pub fn install(root: &Path) -> Result<InstallReport, Box<dyn std::error::Error>> {
    let manifest = Manifest::read(root)?;
    ensure_layout(root)?;
    if manifest.dependencies.is_empty() {
        write_lock_graph(root, &crate::ResolvedGraph::default())?;
        return Ok(InstallReport {
            installed: 0,
            total: 0,
            up_to_date: true,
        });
    }
    let store = crate::store::Store::global()?;
    if crate::lockfile::all_installed(root, &store, &manifest)? {
        return Ok(InstallReport {
            installed: 0,
            total: manifest.dependencies.len(),
            up_to_date: true,
        });
    }
    let graph = resolve_manifest(&manifest)?;
    let installed = store.install(&graph)?;
    write_lock_graph(root, &graph)?;
    Ok(InstallReport {
        installed,
        total: graph.packages.len(),
        up_to_date: installed == 0,
    })
}
pub fn update(root: &Path) -> Result<InstallReport, Box<dyn std::error::Error>> {
    let manifest = Manifest::read(root)?;
    let graph = resolve_manifest(&manifest)?;
    let installed = if graph.packages.is_empty() {
        0
    } else {
        crate::store::Store::global()?.install(&graph)?
    };
    write_lock_graph(root, &graph)?;
    Ok(InstallReport {
        installed,
        total: graph.packages.len(),
        up_to_date: installed == 0,
    })
}
pub fn check(root: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let manifest = Manifest::read(root)?;
    install(root)?;
    let store = crate::store::Store::global()?;
    let roots = crate::lockfile::source_roots(root, &store)?;
    let mut compiler = CompilerSession::new(CompilerConfig::default());
    let checked = compiler.check_source_roots(&roots)?;
    if checked.resolution.diagnostics.has_errors() || checked.semantics.diagnostics.has_errors() {
        return Err(format!(
            "compilation failed\n{}",
            compiler.render_diagnostics(&checked)
        )
        .into());
    }
    Ok(manifest.package.name)
}
pub fn build(root: &Path, release: bool) -> Result<PathBuf, Box<dyn std::error::Error>> {
    install(root)?;
    let store = crate::store::Store::global()?;
    let roots = crate::lockfile::source_roots(root, &store)?;
    let mode = if release {
        BuildMode::Release
    } else {
        BuildMode::Debug
    };
    let mut config = CompilerConfig::for_target(CompilerConfig::default().target, mode);
    config.runtime_library = runtime_library(root);
    config.output_dir = Some(store.build_cache(&config.target, release));
    let manifest = Manifest::read(root)?;
    config.native_libraries = manifest
        .native
        .libraries
        .iter()
        .map(|library| root.join(library))
        .collect();
    for library in &config.native_libraries {
        if !library.is_file() {
            return Err(format!("native library `{}` was not found", library.display()).into());
        }
    }
    config
        .system_libraries
        .clone_from(&manifest.native.system_libraries);
    let output = root
        .join("build")
        .join(if release { "release" } else { "debug" })
        .join(if cfg!(windows) { "app.exe" } else { "app" });
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    CompilerSession::new(config).emit_executable_from_roots(&roots, &output)?;
    Ok(output)
}
fn resolve_manifest(
    manifest: &Manifest,
) -> Result<crate::ResolvedGraph, Box<dyn std::error::Error>> {
    let requests = manifest
        .dependencies
        .iter()
        .map(|(name, dependency)| match dependency {
            Dependency::Registry(version) => Ok((
                PackageSource::Registry { name: name.clone() },
                Some(version.clone()),
            )),
            Dependency::Hosted { source, version } => {
                Ok((PackageSource::parse(source)?, Some(version.clone())))
            }
            Dependency::Detailed {
                package,
                source,
                version,
            } => {
                let source = source.as_ref().map_or_else(
                    || {
                        Ok(PackageSource::Registry {
                            name: package.clone().unwrap_or_else(|| name.clone()),
                        })
                    },
                    |source| PackageSource::parse(source),
                )?;
                Ok((source, Some(version.clone())))
            }
        })
        .collect::<Result<Vec<_>, String>>()?;
    let providers = crate::resolver::Providers::new()?;
    Ok(crate::Resolver::new(&providers).resolve(&requests)?)
}
fn write_lock_graph(
    root: &Path,
    graph: &crate::ResolvedGraph,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut text = format!("lock-version = {}\n", crate::lockfile::LOCKFILE_VERSION);
    for package in graph.packages.values() {
        write!(
            text,
            "\n[[package]]\nname = \"{}\"\nversion = \"{}\"\nsource = \"{}\"\nrevision = \"{}\"\nchecksum = \"{}\"\n",
            package.id.name,
            package.id.version,
            package.id.source,
            package.revision,
            package.checksum
        )?;
        if !package.dependencies.is_empty() {
            let dependencies = package
                .dependencies
                .iter()
                .map(|id| format!("\"{} {} {}\"", id.name, id.version, id.source))
                .collect::<Vec<_>>()
                .join(", ");
            writeln!(text, "dependencies = [{dependencies}]")?;
        }
    }
    atomic_write(&root.join("vpm.lock"), &text)?;
    Ok(())
}
fn ensure_layout(root: &Path) -> io::Result<()> {
    for name in ["packages", "downloads", "build"] {
        std::fs::create_dir_all(root.join(".vpm").join(name))?;
    }
    Ok(())
}
pub(crate) fn runtime_library(root: &Path) -> Option<PathBuf> {
    std::env::var_os("VUT_RUNTIME_LIBRARY")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
        .or_else(|| {
            let name = if cfg!(windows) {
                "libvut_runtime.rlib"
            } else {
                "libvut_runtime.a"
            };
            std::env::current_exe().ok().and_then(|executable| {
                let parent = executable.parent()?;
                [
                    parent.join(name),
                    parent
                        .parent()
                        .map_or_else(PathBuf::new, |path| path.join(name)),
                ]
                .into_iter()
                .find(|path| path.is_file())
            })
        })
        .or_else(|| {
            let p = root.join("target/debug/libvut_runtime.rlib");
            p.is_file().then_some(p)
        })
}
pub(crate) fn atomic_write(path: &Path, content: &str) -> io::Result<()> {
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, content)?;
    std::fs::rename(temp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn temp(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("vpm-{name}-{}", std::process::id()))
    }
    #[test]
    fn init_add_remove_are_atomic_and_parseable() {
        let root = temp("workflow");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        init_named(&root, "demo").unwrap();
        let path = root.join("vpm.toml");
        let mut doc: DocumentMut = std::fs::read_to_string(&path).unwrap().parse().unwrap();
        doc["dependencies"]["math"] = value("1.2.0");
        atomic_write(&path, &doc.to_string()).unwrap();
        assert!(
            Manifest::read(&root)
                .unwrap()
                .dependencies
                .contains_key("math")
        );
        remove(&root, "math").unwrap();
        assert!(Manifest::read(&root).unwrap().dependencies.is_empty());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn init_refuses_overwrite() {
        let root = temp("overwrite");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir(&root).unwrap();
        init_named(&root, "demo").unwrap();
        assert!(init_named(&root, "demo").is_err());
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn runtime_discovery_does_not_depend_on_project_directory() {
        let project = std::env::temp_dir().join("vpm-unrelated-project");
        let runtime = runtime_library(&project);
        if cfg!(windows) {
            assert!(
                runtime.is_some(),
                "workspace test build should locate the Vut runtime independently of the project"
            );
        }
    }
}
