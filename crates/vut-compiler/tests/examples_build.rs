//! Every shipped example must produce a real executable, without running it.
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn all_shipped_examples_build() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
    let scratch = stdlib_common::scratch("examples");
    let mut sources: Vec<_> = std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "vut"))
        .collect();
    sources.sort();
    assert!(!sources.is_empty());
    let mut failures = Vec::new();
    for source in sources {
        let config = vut_compiler::CompilerConfig {
            runtime_library: Some(stdlib_common::runtime_library()),
            ..vut_compiler::CompilerConfig::default()
        };
        let mut session = vut_compiler::CompilerSession::new(config);
        let output = scratch
            .root
            .join(source.file_stem().unwrap())
            .with_extension(if cfg!(windows) { "exe" } else { "bin" });
        if let Err(error) = session.emit_source_file_executable(&source, &output) {
            let checked = session.check_source_file(&source).unwrap();
            failures.push(format!(
                "{}: {error}\n{}",
                source.display(),
                session.render_diagnostics(&checked)
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
