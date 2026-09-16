use crate::generator::Generator;
use std::path::Path;
use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_source::SourceId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct FuzzReport {
    pub corpus_cases: usize,
    pub generated_cases: usize,
    pub bytes: usize,
}

/// Exercises saved and generated inputs through lexer, parser, and lossless tree.
///
/// # Errors
/// Returns an I/O error when a corpus entry cannot be read.
pub fn run(corpus: &Path, iterations: usize, seed: u64) -> Result<FuzzReport, std::io::Error> {
    let mut sources = Vec::new();
    if corpus.is_dir() {
        for entry in std::fs::read_dir(corpus)? {
            let path = entry?.path();
            if path.is_file() {
                sources.push(std::fs::read_to_string(path)?);
            }
        }
    }
    sources.sort();
    let corpus_cases = sources.len();
    let mut bytes = 0;
    for source in &sources {
        exercise(source);
        bytes += source.len();
    }
    let mut generator = Generator::new(seed);
    for _ in 0..iterations {
        let source = generator.source();
        exercise(&source);
        bytes += source.len();
    }
    Ok(FuzzReport {
        corpus_cases,
        generated_cases: iterations,
        bytes,
    })
}

fn exercise(source: &str) {
    let id = SourceId::from_index(0);
    let (tokens, _) = Lexer::new(id, source).lex();
    let _ = Parser::new(id, source, tokens).parse();
    let tree = vut_tooling::LosslessSyntaxTree::parse(source);
    assert_eq!(tree.reconstruct(), source);
    assert_eq!(tree.covered_bytes(), source.len());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn deterministic_generated_corpus_never_panics() {
        let report = run(Path::new("missing"), 5_000, 0x0056_5554).unwrap();
        assert_eq!(report.generated_cases, 5_000);
    }
}
