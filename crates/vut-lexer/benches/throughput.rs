use std::{hint::black_box, time::Instant};
use vut_lexer::Lexer;
use vut_source::SourceId;

#[allow(clippy::cast_precision_loss)]
fn main() {
    let line = "value = out(\"result = $((a + b) * (c - d))\") # comment\n";
    let source = line.repeat(20_000);
    let started = Instant::now();
    let mut bytes = 0usize;
    for _ in 0..20 {
        let (tokens, diagnostics) = Lexer::new(SourceId::from_index(0), black_box(&source)).lex();
        assert!(!diagnostics.has_errors());
        black_box(tokens);
        bytes += source.len();
    }
    let elapsed = started.elapsed();
    let mib_per_second = bytes as f64 / elapsed.as_secs_f64() / (1024.0 * 1024.0);
    println!("lexed {mib_per_second:.1} MiB/s");
}
