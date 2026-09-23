//! Optimized and unoptimized executables must agree on numeric semantics.
#[path = "stdlib_common/mod.rs"]
mod stdlib_common;

#[test]
fn typed_literal_arithmetic_preserves_width_and_unsigned_division() {
    let source = r"fn quotient() -> u64:
  18446744073709551615 / 3
fn remainder() -> u64:
  18446744073709551615 % 3
fn narrow() -> u8:
  255 - 1
fn main():
  out(quotient())
  out(remainder())
  out(narrow())
";
    for mode in [
        vut_compiler::BuildMode::Debug,
        vut_compiler::BuildMode::Release,
    ] {
        let scratch = stdlib_common::scratch("numeric_optimization");
        let input = scratch.root.join("main.vut");
        std::fs::write(&input, source).unwrap();
        let output = scratch.root.join(if cfg!(windows) {
            "program.exe"
        } else {
            "program"
        });
        let mut session = vut_compiler::CompilerSession::new(vut_compiler::CompilerConfig {
            build_mode: mode,
            runtime_library: Some(stdlib_common::runtime_library()),
            ..vut_compiler::CompilerConfig::default()
        });
        let checked = session.check_source_file(&input).unwrap();
        assert!(
            !checked.semantics.diagnostics.has_errors(),
            "{}",
            session.render_diagnostics(&checked)
        );
        session
            .emit_source_file_executable(&input, &output)
            .unwrap();
        let (code, stdout, stderr) = stdlib_common::run_executable(&output, &[], None);
        assert_eq!(code, Some(0), "{mode:?}: {stderr}");
        assert_eq!(stdout, "6148914691236517205\n0\n254\n", "{mode:?}");
    }
}
