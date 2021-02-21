pub mod lex;
pub mod parse;
pub mod validate;
pub mod modules;

pub struct LexerCallback<'a> {
    pub string: &'a str,
    pub replacement: &'a str,
}