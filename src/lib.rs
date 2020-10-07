pub mod lex;
pub mod parse;

#[derive(Debug, Copy, Clone)]
pub struct Span {
    pub start_line: usize,
    pub start_column: usize,
    // TODO: This
    pub end_line: usize,
    pub end_column: usize,
}

impl Span {
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            start_line, 
            start_column,
            // TODO: This
            end_line: 0,
            end_column: 0,
        }
    }

    pub fn merge(&mut self, other: Span) {
        // this span begins before other
        if (self.start_line < other.start_line) && (self.start_column < other.start_column) {
            
        } else { // other span starts before this
            self.start_line = other.start_line;
            self.start_column = other.start_column;
        }
    }
}