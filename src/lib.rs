pub mod frontend;
pub mod backend;
pub mod macros;
pub mod extension;

/// Includes essential types and convenience macros
pub mod prelude {
    pub use crate::{Return, Jitter, GetFunction, GetFunctions};
    pub use crate::frontend::LexerCallback;
    pub use crate::backend::jit::{JitterContextBuilder, JitterContext};
}

/// Wrapper around a stack-allocated Jitter-returned type.
///
/// Note that the underlying type **must** be `#[repr(C)]` for non-primitive types.  
/// Similarly, the Rust and Jitter layouts must be the same (i.e.: fields in same order).
#[repr(transparent)]
pub struct Return<T> {
    value: *mut T,
}

impl<T> Return<T> {
    /// Converts Jitter return value to underlying type
    pub fn into(self) -> T {
        // NOTE: Dereferencing a raw pointer drops it immediately. This is the only way
        //       I have been able to get the proper data (zeroed out otherwise)
        unsafe { std::ptr::read(self.value) }
    }
}


// TODO: make `pub(crate)`
/// Token/AST spans
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