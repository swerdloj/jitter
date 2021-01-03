pub mod frontend;
pub mod backend;


pub use proc_macros::{link, export};

#[macro_export]
macro_rules! FFI {
    ($f:ident) => {
        (stringify!($f), $f as *const u8)
    }
}

pub mod prelude {
    pub use crate::FFI;
    pub use crate::backend::jit::{JITContextBuilder, JITContext};
}


// FIXME: I want these macros to be private within the crate
#[macro_use]
pub(crate) mod macros {
    #[cfg(not(feature = "benchmark"))]
    #[macro_export]
    macro_rules! log {
        ($($e:tt)*) => {
            println!(
                $( $e )*
            );
        };
    }
    
    #[cfg(feature = "benchmark")]
    #[macro_export]
    macro_rules! log {
        ($($e:tt)*) => {};
    }
}


#[derive(Copy, Clone)]
pub struct Span {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
}

impl std::fmt::Debug for Span {
    // FIXME: Make printing spans optional
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl Span {
    pub fn new(start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> Self {
        Self {
            start_line, 
            start_column,
            end_line,
            end_column,
        }
    }

    /// Extends the span to start at `self` and end at `other`
    pub fn extend(mut self, other: Span) -> Span {
        self.end_line = other.end_line;
        self.end_column = other.end_column;
        self
    }
}