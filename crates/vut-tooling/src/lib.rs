//! Source discovery, lossless syntax formatting, and conservative linting.
#![allow(clippy::missing_errors_doc)]

mod discovery;
mod formatter;
mod lint;
mod syntax;
mod validation;

pub use discovery::discover;
pub use formatter::format_source;
pub use lint::{Lint, lint_source};
pub use syntax::{LosslessSyntaxTree, SyntaxKind, SyntaxToken};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formatter_is_canonical_lossless_and_idempotent() {
        let input = "### adds values\nfn add(a: int,b: int) -> int:\n    text=\"a+b # untouched\" # note\n    a+b\n";
        let tree = LosslessSyntaxTree::parse(input);
        assert_eq!(tree.reconstruct(), input);
        assert_eq!(tree.covered_bytes(), input.len());
        let formatted = format_source(input).unwrap();
        assert_eq!(
            formatted,
            "### adds values\nfn add(a: int, b: int) -> int:\n  text = \"a+b # untouched\" # note\n  a + b\n"
        );
        assert_eq!(format_source(&formatted).unwrap(), formatted);
    }

    #[test]
    fn formatter_preserves_multiline_comments_and_crlf_content() {
        let input = "fn main():\r\n    ## keep\r\nverbatim + text\r\n##\r\n    out(\"x\")\r\n";
        let tree = LosslessSyntaxTree::parse(input);
        assert_eq!(tree.reconstruct(), input);
        let formatted = format_source(input).unwrap();
        assert!(formatted.contains("## keep\r\nverbatim + text\r\n##"));
        assert!(formatted.ends_with("  out(\"x\")\n"));
    }

    #[test]
    fn formatter_never_rewrites_template_expression_strings() {
        let input = "fn main():\n    out(\"nested=$(1+2) # untouched\")\n";
        let formatted = format_source(input).unwrap();
        assert_eq!(
            formatted,
            "fn main():\n  out(\"nested=$(1+2) # untouched\")\n"
        );
    }

    #[test]
    fn formatter_rejects_invalid_source() {
        assert!(format_source("fn broken(:\n").is_err());
    }

    #[test]
    fn formatter_handles_block_expressions_idempotently() {
        let input = "fn f(x: int) -> int:\n  match x:\n    0:\n      a=1\n      a+2\n    _: 0\n\nfn g() -> int:\n  total: int =\n    a=10\n    b=20\n    a+b\n  total\n";
        let formatted = format_source(input).unwrap();
        assert_eq!(format_source(&formatted).unwrap(), formatted);
        assert!(
            formatted.contains("    0:\n      a = 1\n      a + 2\n"),
            "{formatted:?}"
        );
        assert!(
            formatted.contains("  total: int =\n    a = 10\n    b = 20\n    a + b\n"),
            "{formatted:?}"
        );
    }

    #[test]
    fn linter_reports_stable_warning_codes() {
        let source = "fn main():\n  unused = 1\n  if true:\n    return\n    out(\"never\")\n";
        let lints = lint_source(source).unwrap();
        assert!(lints.iter().any(|lint| lint.code == "W2001"));
        assert!(lints.iter().any(|lint| lint.code == "W2004"));
        assert!(lints.iter().any(|lint| lint.code == "W2003"));
    }
}
