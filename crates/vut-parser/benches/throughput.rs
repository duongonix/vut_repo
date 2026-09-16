use std::{hint::black_box, time::Instant};
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_source::SourceId;

#[allow(clippy::cast_precision_loss)]
fn main() {
    let function =
        "fn calculate(a: int, b: int) -> int:\n  out(\"result = $((a + b) * 2)\")\n  a + b * 2\n";
    let source = function.repeat(5_000);
    let source_id = SourceId::from_index(0);
    let (tokens, lexical) = Lexer::new(source_id, &source).lex();
    assert!(!lexical.has_errors());
    let started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..10 {
        let (file, diagnostics) =
            Parser::new(source_id, black_box(&source), tokens.clone()).parse();
        assert!(!diagnostics.has_errors());
        black_box(file);
        bytes += source.len();
    }
    let mib_per_second = bytes as f64 / started.elapsed().as_secs_f64() / (1024.0 * 1024.0);
    println!("parsed {mib_per_second:.1} MiB/s");
}
