use vut_lexer::Lexer;
use vut_parser::Parser;
use vut_source::SourceId;

pub(crate) fn validate(source: &str) -> Result<(), String> {
    let id = SourceId::from_index(0);
    let (tokens, lexical) = Lexer::new(id, source).lex();
    let (_, syntax) = Parser::new(id, source, tokens).parse();
    if lexical.has_errors() || syntax.has_errors() {
        Err("cannot format source containing syntax errors".into())
    } else {
        Ok(())
    }
}
