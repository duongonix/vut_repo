use std::{fmt::Write, time::Instant};
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_resolver::{ModuleInput, ModulePath, Resolver};
use vut_source::SourceId;
use vut_types::Analyzer;

fn main() {
    let mut source = String::new();
    for index in 0..2_000 {
        writeln!(
            source,
            "fn work_{index}(value: int) -> int:\n  result = value + {index}\n  result"
        )
        .unwrap();
    }
    let source_id = SourceId::from_index(0);
    let (tokens, _) = Lexer::new(source_id, &source).lex();
    let (file, _) = Parser::new(source_id, &source, tokens).parse();
    let resolution = Resolver::new(vec![ModuleInput {
        logical_path: ModulePath(vec!["bench".into()]),
        filesystem_path: None,
        file,
    }])
    .resolve();
    let start = Instant::now();
    let result = Analyzer::new(&resolution).analyze();
    let elapsed = start.elapsed();
    assert!(!result.diagnostics.has_errors());
    let bytes = u32::try_from(source.len()).expect("benchmark fixture fits u32");
    let mib = f64::from(bytes) / (1024.0 * 1024.0);
    println!("type-checked {:.1} MiB/s", mib / elapsed.as_secs_f64());
}
