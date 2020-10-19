pub mod frontend;
pub mod backend;

pub use proc_macros::{link, export};


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


/// Creates a JIT-compiled Jitter context using the specified `.jitter` file
pub fn create_local_context(path: &str) -> backend::codegen::JITContext {
    #[cfg(feature = "benchmark")]
    let now = std::time::Instant::now();


    let input = &std::fs::read_to_string(path).unwrap();

    let tokens = frontend::lex::Lexer::lex_str(path.as_ref(), input, true);
    // log!("Tokens:\n{:#?}", tokens);

    let parser = frontend::parse::Parser::new(path, tokens);
    let ast = parser.parse_ast();
    log!("AST:\n{:#?}", ast);


    let mut jit = backend::codegen::JITContext::new();
    jit.translate(ast).unwrap();


    // NOTE: Do not use `println` prior to this call if analyzing compile time. 
    // Printing is slow and irregular. Use `log!` with feature `benchmark`
    #[cfg(feature = "benchmark")]
    println!("Running time: {}ms", now.elapsed().as_micros() as f32 / 1000.0);

    jit
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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