//! `CompilerSession` orchestration methods.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use vut_diagnostics::{Diagnostic, DiagnosticSink, codes};
use vut_resolver::{
    DiscoverError, ModuleInput, Resolver, SymbolKind, load_source_file, load_source_root,
};
use vut_source::{SourceError, SourceId};
use vut_types::{Analyzer, Type};

use super::permissions::restore_executable_permissions;
use super::{BuildMode, CheckedProgram, CompileError, CompilerSession};

/// Locates the official Vut standard library source root.
///
/// Resolution order:
/// 1. `VUT_STDLIB_PATH` (development override)
/// 2. `~/.vut/std` (installed distribution)
/// 3. the bundled `vut-stdlib/std` tree (repository/CI builds)
fn stdlib_root() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VUT_STDLIB_PATH") {
        let path = PathBuf::from(path);
        if path.is_dir() {
            return Some(path);
        }
    }
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    if let Some(home) = home {
        let path = PathBuf::from(home).join(".vut").join("std");
        if path.is_dir() {
            return Some(path);
        }
    }
    let bundled = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vut-stdlib/std");
    bundled.is_dir().then_some(bundled)
}

/// Locates the native stdlib runtime archive built from
/// `vut-stdlib/native/vut-runtime`.
///
/// A bundled static library is preferred so that the HTTP/TLS dependencies are
/// linked transitively without the compiler knowing anything about them.
fn stdlib_runtime_library(config: &super::CompilerConfig) -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("VUT_STDLIB_RUNTIME") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }
    let directory = config.runtime_library.as_ref()?.parent()?;
    for name in [
        "vut_stdlib_native.lib",
        "libvut_stdlib_native.a",
        "libvut_stdlib_native.rlib",
    ] {
        let candidate = directory.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Returns true when any module imports a top-level standard-library module.
fn references_stdlib(modules: &[ModuleInput], root: &Path) -> bool {
    modules.iter().any(|module| {
        module.file.items.iter().any(|item| {
            matches!(
                item,
                vut_ast::Item::Import(import)
                    if import.path.first().is_some_and(|name| root.join(&name.text).is_dir())
            )
        })
    })
}

impl CompilerSession {
    #[must_use]
    pub fn new(config: super::CompilerConfig) -> Self {
        Self {
            config,
            sources: vut_source::SourceManager::new(),
            diagnostics: DiagnosticSink::new(),
        }
    }
    #[must_use]
    pub fn config(&self) -> &super::CompilerConfig {
        &self.config
    }
    #[must_use]
    pub fn sources(&self) -> &vut_source::SourceManager {
        &self.sources
    }
    pub fn sources_mut(&mut self) -> &mut vut_source::SourceManager {
        &mut self.sources
    }
    #[must_use]
    pub fn diagnostics(&self) -> &DiagnosticSink {
        &self.diagnostics
    }
    /// Renders all resolver and semantic diagnostics with source locations.
    #[must_use]
    pub fn render_diagnostics(&self, checked: &CheckedProgram) -> String {
        let renderer = vut_diagnostics::Renderer::new(vut_diagnostics::ColorChoice::Auto);
        checked
            .resolution
            .diagnostics
            .as_slice()
            .iter()
            .chain(checked.semantics.diagnostics.as_slice())
            .map(|diagnostic| renderer.render(diagnostic, &self.sources))
            .collect::<String>()
    }

    fn language_error(&self, checked: &CheckedProgram) -> CompileError {
        CompileError::Language(self.render_diagnostics(checked))
    }

    /// Appends the bundled/installed standard library modules to a root's
    /// modules so `import fs` and friends resolve as top-level canonical
    /// modules. The library is optional: when absent, compilation proceeds with
    /// user modules only.
    fn with_stdlib(
        &mut self,
        mut modules: Vec<ModuleInput>,
        diagnostics: &mut DiagnosticSink,
    ) -> Result<Vec<ModuleInput>, DiscoverError> {
        let Some(root) = stdlib_root() else {
            return Ok(modules);
        };
        if !references_stdlib(&modules, &root) {
            return Ok(modules);
        }
        let loaded = load_source_root(&root, &[], &mut self.sources)?;
        modules.extend(loaded.modules);
        diagnostics.extend(loaded.diagnostics);
        Ok(modules)
    }
    /// Loads a Vut source file into this session.
    ///
    /// # Errors
    ///
    /// Forwards source-manager I/O and UTF-8 validation errors.
    pub fn load_source(
        &mut self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<SourceId, SourceError> {
        self.sources.load_path(path)
    }

    /// Discovers, parses, and resolves every module below a source root.
    ///
    /// `prefix` is empty for a project root and contains the exposed package
    /// name for a dependency root.
    ///
    /// # Errors
    ///
    /// Returns filesystem, UTF-8, or invalid module-path discovery errors.
    pub fn resolve_source_root(
        &mut self,
        root: &Path,
        prefix: &[String],
    ) -> Result<vut_resolver::Resolution, DiscoverError> {
        let loaded = load_source_root(root, prefix, &mut self.sources)?;
        let mut diagnostics = loaded.diagnostics;
        let modules = self.with_stdlib(loaded.modules, &mut diagnostics)?;
        let mut resolution = Resolver::new(modules).resolve();
        resolution.diagnostics.extend(diagnostics);
        resolution.diagnostics.sort_deterministically();
        Ok(resolution)
    }

    /// Runs the frontend and ownership-aware lowering pipeline through MIR.
    ///
    /// # Errors
    ///
    /// Returns source-root discovery errors. Language errors are accumulated in
    /// the resolution and semantic diagnostic sinks.
    pub fn check_source_root(
        &mut self,
        root: &Path,
        prefix: &[String],
    ) -> Result<CheckedProgram, DiscoverError> {
        let loaded = load_source_root(root, prefix, &mut self.sources)?;
        let diagnostics = loaded.diagnostics;
        self.check_modules(loaded.modules, diagnostics)
    }

    /// Runs the frontend over a set of already-loaded modules, applying the
    /// compiler-generated display pass when `out`/`print` or template
    /// interpolation needs it.
    fn check_modules(
        &mut self,
        modules: Vec<ModuleInput>,
        mut diagnostics: DiagnosticSink,
    ) -> Result<CheckedProgram, DiscoverError> {
        let mut modules = self.with_stdlib(modules, &mut diagnostics)?;
        let mut checked = self.run_frontend(&modules);
        // Semantic errors are expected here: `out(x)` is checked against `str`
        // before the display pass rewrites it. Only resolution errors prevent
        // the transform.
        if checked.resolution.diagnostics.has_errors() {
            checked.resolution.diagnostics.extend(diagnostics);
            checked.resolution.diagnostics.sort_deterministically();
            return Ok(checked);
        }
        let changed = super::display::apply(
            &mut modules,
            &checked.semantics,
            &checked.resolution,
            &mut self.sources,
        );
        if !changed {
            checked.resolution.diagnostics.extend(diagnostics);
            checked.resolution.diagnostics.sort_deterministically();
            return Ok(checked);
        }
        let mut checked = self.run_frontend(&modules);
        checked.resolution.diagnostics.extend(diagnostics);
        checked.resolution.diagnostics.sort_deterministically();
        Ok(checked)
    }

    /// One resolve → HIR → analyze → MIR pass over the given modules.
    fn run_frontend(&self, modules: &[ModuleInput]) -> CheckedProgram {
        let mut resolution = Resolver::new(modules.to_vec()).resolve();
        resolution.diagnostics.sort_deterministically();
        let hir = vut_hir::lower(&resolution);
        let mut semantics = Analyzer::new(&resolution).analyze();
        let mut mir = vut_mir::lower(
            &hir,
            &semantics,
            vut_codegen::pointer_bytes_for_target(&self.config.target),
        );
        semantics
            .diagnostics
            .extend(std::mem::take(&mut mir.diagnostics));
        CheckedProgram {
            resolution,
            hir,
            semantics,
            mir,
        }
    }

    /// Runs the frontend for exactly one standalone source file.
    ///
    /// # Errors
    /// Returns source loading errors. Language errors are retained as diagnostics.
    pub fn check_source_file(&mut self, path: &Path) -> Result<CheckedProgram, DiscoverError> {
        let loaded = load_source_file(path, &mut self.sources)?;
        let diagnostics = loaded.diagnostics;
        self.check_modules(loaded.modules, diagnostics)
    }

    /// Compiles exactly one standalone file to a native executable.
    ///
    /// # Errors
    /// Returns frontend, code-generation, artifact I/O, or linker failures.
    pub fn emit_source_file_executable(
        &mut self,
        source: &Path,
        output: &Path,
    ) -> Result<(), CompileError> {
        use vut_linker::NativeLinker;
        vut_codegen::verify_runtime_abi(vut_runtime::abi::VERSION)
            .map_err(CompileError::Codegen)?;
        let mut checked = self
            .check_source_file(source)
            .map_err(CompileError::Discover)?;
        let object = self.compile_executable_object(&mut checked)?;
        let object_path = output.with_extension("o");
        std::fs::write(&object_path, object).map_err(CompileError::Io)?;
        let plan = self.link_plan(&object_path, output);
        let result = vut_linker::SystemLinker
            .link(&plan)
            .map_err(CompileError::Link);
        let _ = std::fs::remove_file(&object_path);
        result
    }

    /// Runs the frontend over a project root and its explicitly named dependency roots.
    ///
    /// # Errors
    /// Returns source discovery errors for any root.
    pub fn check_source_roots(
        &mut self,
        roots: &[(PathBuf, Vec<String>)],
    ) -> Result<CheckedProgram, DiscoverError> {
        let mut modules = Vec::new();
        let mut discovery_diagnostics = DiagnosticSink::new();
        let mut local_namespaces = BTreeMap::new();
        let mut dependency_namespaces = BTreeSet::new();
        for (root, prefix) in roots {
            let loaded = load_source_root(root, prefix, &mut self.sources)?;
            if let Some(namespace) = prefix.first() {
                dependency_namespaces.insert(namespace.clone());
            } else {
                for module in &loaded.modules {
                    if let Some(namespace) = module.logical_path.0.first() {
                        local_namespaces
                            .entry(namespace.clone())
                            .or_insert_with(|| module.filesystem_path.clone());
                    }
                }
            }
            modules.extend(loaded.modules);
            discovery_diagnostics.extend(loaded.diagnostics);
        }
        for namespace in local_namespaces
            .keys()
            .filter(|name| dependency_namespaces.contains(*name))
        {
            if let Some(module) = modules
                .iter()
                .find(|module| module.logical_path.0.first() == Some(namespace))
            {
                discovery_diagnostics.push(
                    Diagnostic::error(
                        codes::E3008,
                        "module namespace conflict",
                        vut_source::Span::new(module.file.source, 0, 0),
                        format!("dependency namespace `{namespace}` conflicts with local module"),
                    )
                    .with_note(format!(
                        "local module: {}",
                        module
                            .filesystem_path
                            .as_ref()
                            .map_or_else(|| namespace.clone(), |path| path.display().to_string())
                    ))
                    .with_note(format!("dependency import namespace: {namespace}"))
                    .with_help(format!(
                        "alias the dependency to another import name, such as `{namespace}_pkg`"
                    )),
                );
            }
        }
        self.check_modules(modules, discovery_diagnostics)
    }

    /// Compiles a checked source tree to a target-native object file in memory.
    ///
    /// # Errors
    ///
    /// Returns discovery or native backend errors. Language diagnostics remain
    /// available in the returned frontend stages through `check_source_root`.
    pub fn emit_object(&mut self, root: &Path, prefix: &[String]) -> Result<Vec<u8>, CompileError> {
        vut_codegen::verify_runtime_abi(vut_runtime::abi::VERSION)
            .map_err(CompileError::Codegen)?;
        let roots = [(root.to_owned(), prefix.to_vec())];
        let cache = vut_incremental::ArtifactCache::new(
            self.config
                .output_dir
                .clone()
                .unwrap_or_else(|| root.join(".vut-cache/objects")),
        );
        let key = self.cache_key(&roots, "object")?;
        if let Some(object) = cache.get(&key).map_err(CompileError::Cache)? {
            return Ok(object);
        }
        let mut checked = self
            .check_source_root(root, prefix)
            .map_err(CompileError::Discover)?;
        if self.config.build_mode == BuildMode::Release {
            vut_mir::optimize(&mut checked.mir);
        }
        let object = self.emit_checked_object(&checked)?;
        cache.put(&key, &object).map_err(CompileError::Cache)?;
        Ok(object)
    }

    /// Emits and links a native executable using the configured target backend.
    ///
    /// # Errors
    /// Returns frontend, code-generation, artifact I/O, or platform-linker failures.
    pub fn emit_executable(
        &mut self,
        root: &Path,
        prefix: &[String],
        output: &Path,
    ) -> Result<(), CompileError> {
        self.emit_executable_from_roots(&[(root.to_owned(), prefix.to_vec())], output)
    }

    /// Emits an executable from a project source root plus named package roots.
    ///
    /// # Errors
    /// Returns frontend, backend, I/O, or linker failures.
    pub fn emit_executable_from_roots(
        &mut self,
        roots: &[(PathBuf, Vec<String>)],
        output: &Path,
    ) -> Result<(), CompileError> {
        use vut_linker::NativeLinker;
        vut_codegen::verify_runtime_abi(vut_runtime::abi::VERSION)
            .map_err(CompileError::Codegen)?;
        let cache_root = self.config.output_dir.clone().unwrap_or_else(|| {
            output
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(".vut-cache/objects")
        });
        let object_cache = vut_incremental::ArtifactCache::new(cache_root.join("objects"));
        let executable_cache = vut_incremental::ArtifactCache::new(cache_root.join("executables"));
        let key = self.cache_key(roots, "executable-object")?;
        let executable_key = self.cache_key(roots, "executable")?;
        if let Some(executable) = executable_cache
            .get(&executable_key)
            .map_err(CompileError::Cache)?
        {
            std::fs::write(output, executable).map_err(CompileError::Io)?;
            restore_executable_permissions(output).map_err(CompileError::Io)?;
            return Ok(());
        }
        let cached = object_cache.get(&key).map_err(CompileError::Cache)?;
        let object = if let Some(object) = cached {
            object
        } else {
            let mut checked = self
                .check_source_roots(roots)
                .map_err(CompileError::Discover)?;
            let object = self.compile_executable_object(&mut checked)?;
            object_cache
                .put(&key, &object)
                .map_err(CompileError::Cache)?;
            object
        };
        let object_path = output.with_extension("o");
        std::fs::write(&object_path, object).map_err(CompileError::Io)?;
        let plan = self.link_plan(&object_path, output);
        let result = vut_linker::SystemLinker
            .link(&plan)
            .map_err(CompileError::Link);
        let _ = std::fs::remove_file(&object_path);
        result?;
        let executable = std::fs::read(output).map_err(CompileError::Io)?;
        executable_cache
            .put(&executable_key, &executable)
            .map_err(CompileError::Cache)?;
        Ok(())
    }

    fn link_plan(&self, object: &Path, output: &Path) -> vut_linker::LinkPlan {
        let mut static_libraries = self.config.native_libraries.clone();
        if let Some(stdlib) = stdlib_runtime_library(&self.config) {
            static_libraries.push(stdlib);
        }
        vut_linker::LinkPlan {
            objects: vec![object.to_owned()],
            static_libraries,
            system_libraries: self.config.system_libraries.clone(),
            frameworks: Vec::new(),
            runtime: self.config.runtime_library.clone(),
            startup: self.config.startup_object.clone(),
            output: output.to_owned(),
            entry: "vut_entry".into(),
            target: self.config.target.clone(),
        }
    }

    fn compile_executable_object(
        &self,
        checked: &mut CheckedProgram,
    ) -> Result<Vec<u8>, CompileError> {
        if checked.resolution.diagnostics.has_errors() || checked.semantics.diagnostics.has_errors()
        {
            return Err(self.language_error(checked));
        }
        let entry = checked
            .resolution
            .symbols
            .iter()
            .find(|symbol| {
                symbol.name == "main"
                    && symbol.kind == SymbolKind::Function
                    && symbol.receiver.is_none()
            })
            .ok_or(CompileError::MissingEntry)?
            .id;
        let signature = checked
            .semantics
            .function_signatures
            .get(&entry)
            .ok_or(CompileError::InvalidEntry)?;
        if !signature.parameters.is_empty()
            || !matches!(
                checked.semantics.types[signature.result.0],
                Type::Void | Type::Int
            )
        {
            return Err(CompileError::InvalidEntry);
        }
        if self.config.build_mode == BuildMode::Release {
            vut_mir::optimize(&mut checked.mir);
        }
        vut_codegen::CraneliftBackend::with_optimization(
            vut_codegen::Target {
                triple: self.config.target.clone(),
            },
            self.config.build_mode == BuildMode::Release,
        )
        .compile_executable_module_with_memory_check(
            &checked.mir,
            entry,
            self.config.runtime_library.is_some(),
        )
        .map_err(CompileError::Codegen)
    }

    fn emit_checked_object(&self, checked: &CheckedProgram) -> Result<Vec<u8>, CompileError> {
        use vut_codegen::CodegenBackend;
        if checked.resolution.diagnostics.has_errors() || checked.semantics.diagnostics.has_errors()
        {
            return Err(self.language_error(checked));
        }
        vut_codegen::CraneliftBackend::with_optimization(
            vut_codegen::Target {
                triple: self.config.target.clone(),
            },
            self.config.build_mode == BuildMode::Release,
        )
        .compile_module(&checked.mir)
        .map_err(CompileError::Codegen)
    }

    pub(super) fn cache_key(
        &self,
        roots: &[(PathBuf, Vec<String>)],
        namespace: &str,
    ) -> Result<vut_incremental::CacheKey, CompileError> {
        let source = vut_incremental::fingerprint_roots(roots).map_err(CompileError::Io)?;
        let mut key = vut_incremental::CacheKeyBuilder::new(namespace)
            .field("source", source)
            .field("compiler", env!("CARGO_PKG_VERSION"))
            .field("language", "vut-mvp-1")
            .field("target", &self.config.target)
            .field("mode", format!("{:?}", self.config.build_mode))
            .field("runtime-abi", vut_runtime::abi::VERSION.to_le_bytes());
        if let Some(runtime_library) = &self.config.runtime_library {
            key = key.field(
                "runtime-library",
                std::fs::read(runtime_library).map_err(CompileError::Io)?,
            );
        }
        for (index, library) in self.config.native_libraries.iter().enumerate() {
            key = key.field(
                &format!("native-library-{index}"),
                std::fs::read(library).map_err(CompileError::Io)?,
            );
        }
        key = key.field(
            "system-libraries",
            format!("{:?}", self.config.system_libraries).into_bytes(),
        );
        if let Some(stdlib) = stdlib_runtime_library(&self.config) {
            key = key.field(
                "stdlib-runtime",
                std::fs::read(stdlib).map_err(CompileError::Io)?,
            );
        }
        Ok(key.finish())
    }
}
