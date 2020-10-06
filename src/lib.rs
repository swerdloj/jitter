pub mod lex;
pub mod parse;

#[derive(Debug)]
pub struct Span {
    // TODO: This should track left-pos to right-pos
    // from: usize,
    // to: usize,
    line: usize,
    column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Self {line, column}
    }
}