//! Target-sized literal boundaries are checked independently of the host.
use super::*;

fn check(source: &str, pointer_bytes: usize) -> SemanticResult {
    let id = SourceId::from_index(0);
    let (tokens, lexical) = Lexer::new(id, source).lex();
    assert!(!lexical.has_errors());
    let (file, syntax) = Parser::new(id, source, tokens).parse();
    assert!(!syntax.has_errors());
    let resolution = Resolver::new(vec![ModuleInput {
        logical_path: ModulePath(vec!["main".into()]),
        filesystem_path: None,
        file,
    }])
    .resolve();
    assert!(!resolution.diagnostics.has_errors());
    Analyzer::for_target(&resolution, pointer_bytes).analyze()
}

#[test]
fn pointer_sized_literals_use_target_range() {
    let source =
        "fn main():\n  a: usize = 4294967295\n  b: isize = 2147483647\n  c: isize = -2147483648\n";
    assert!(!check(source, 4).diagnostics.has_errors());
    for literal in [
        "a: usize = 4294967296",
        "a: isize = 2147483648",
        "a: isize = -2147483649",
    ] {
        let source = format!("fn main():\n  {literal}\n");
        assert!(check(&source, 4).diagnostics.has_errors(), "{literal}");
        assert!(!check(&source, 8).diagnostics.has_errors(), "{literal}");
    }
}

#[test]
fn unit_is_not_void_or_a_raw_c_scalar() {
    for source in [
        "extern \"C\" fn bad(value: unit)\n",
        "extern \"C\" fn bad() -> unit\n",
        "fn nothing():\n  out(\"none\")\nfn main():\n  value: unit = nothing()\n",
    ] {
        assert!(check(source, 8).diagnostics.has_errors(), "{source}");
    }
}
