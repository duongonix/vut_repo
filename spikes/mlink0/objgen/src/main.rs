//! M-LINK.0 spike tool: emit the executable object for a Vut source root.
//!
//! This mirrors `CompilerSession::emit_source_file_executable` up to (but not
//! including) the link step, so the spike can drive a raw linker itself.

use std::path::PathBuf;
use std::process::ExitCode;

use vut_compiler::{BuildMode, CompilerConfig, CompilerSession};
use vut_resolver::SymbolKind;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "usage: vut-objgen <source-root> <object-out> [--release] [--no-memory-check] [--target <triple>]"
        );
        return ExitCode::from(2);
    }
    let root = PathBuf::from(&args[1]);
    let object_out = PathBuf::from(&args[2]);
    let mut release = false;
    let mut memory_check = true;
    let mut target = vut_codegen::Target::host().triple;

    let mut index = 3;
    while index < args.len() {
        match args[index].as_str() {
            "--release" => release = true,
            "--no-memory-check" => memory_check = false,
            "--target" => {
                index += 1;
                if index >= args.len() {
                    eprintln!("--target needs a value");
                    return ExitCode::from(2);
                }
                target.clone_from(&args[index]);
            }
            other => {
                eprintln!("unknown argument: {other}");
                return ExitCode::from(2);
            }
        }
        index += 1;
    }

    if let Err(error) = vut_codegen::verify_runtime_abi(vut_runtime::abi::VERSION) {
        eprintln!("runtime ABI error: {error}");
        return ExitCode::FAILURE;
    }

    let mode = if release {
        BuildMode::Release
    } else {
        BuildMode::Debug
    };
    let mut session = CompilerSession::new(CompilerConfig::for_target(target.clone(), mode));
    let roots = [(root, Vec::<String>::new())];
    let mut checked = match session.check_source_roots(&roots) {
        Ok(checked) => checked,
        Err(error) => {
            eprintln!("discover error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if checked.resolution.diagnostics.has_errors() || checked.semantics.diagnostics.has_errors() {
        eprint!("{}", session.render_diagnostics(&checked));
        return ExitCode::FAILURE;
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
        .map(|symbol| symbol.id);
    let Some(entry) = entry else {
        eprintln!("no `main` entry point found");
        return ExitCode::FAILURE;
    };

    if release {
        vut_mir::optimize(&mut checked.mir);
    }

    let backend = vut_codegen::CraneliftBackend::with_optimization(
        vut_codegen::Target { triple: target },
        release,
    );
    let object = match backend.compile_executable_module_with_memory_check(
        &checked.mir,
        entry,
        memory_check,
    ) {
        Ok(object) => object,
        Err(error) => {
            eprintln!("codegen error: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Err(error) = std::fs::write(&object_out, object) {
        eprintln!("write error: {error}");
        return ExitCode::FAILURE;
    }
    println!("{}", object_out.display());
    ExitCode::SUCCESS
}
