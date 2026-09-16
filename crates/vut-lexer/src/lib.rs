//! Vut lexical analysis facade.
mod lexer;
mod token;
pub use lexer::Lexer;
pub use token::{Token, TokenKind};
