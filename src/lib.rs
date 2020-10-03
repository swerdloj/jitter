pub mod lex;
pub mod parse;

#[derive(Debug)]
pub struct Span {
    line: usize,
    column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self {line, column}
    }
}