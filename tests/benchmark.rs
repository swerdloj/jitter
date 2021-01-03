struct Timer<'a> {
    name: &'a str,
    start: std::time::Instant,
}

impl<'a> Timer<'a> {
    /// Creates a new, **started** timer
    fn start(name: &'a str) -> Self {
        Self {
            name,
            start: std::time::Instant::now(),
        }
    }
    
    /// Ends the timer and prints a "name -> TIMEms"
    /// Returns the elapsed time
    fn end(self) -> f32 {
        let elapsed_ms = self.start.elapsed().as_micros() as f32 / 1000.0;
        println!("{} -> {}ms", self.name, elapsed_ms);
        elapsed_ms
    }
}

// shorthand
fn time(m:&str) -> Timer {
    Timer::start(m)
}

// TODO: see https://doc.rust-lang.org/1.7.0/book/benchmark-tests.html

#[cfg(test)]
mod tests {
    use super::time;
    #[test]
    fn benchmark() {
        let path = "./tests/integration_test.jitter";

        let mut total_time = 0f32;


        let read_and_tokenize = time("read_and_tokenize");
        // Read file to string
        let input = &std::fs::read_to_string(path).unwrap();
        // Tokenize the input
        let tokens = jitter::frontend::lex::Lexer::lex_str(path.as_ref(), input, true);
        total_time += read_and_tokenize.end();


        let parse = time("parse");

        // Create parser and parse AST
        let parser = jitter::frontend::parse::Parser::new(path, tokens);
        let ast = parser.parse_ast();

        total_time += parse.end();
        // log!("AST:\n{:#?}", ast);


        let validate = time("validate");

        // Validate the AST
        let validation_context = jitter::frontend::validate::validate_ast(ast).unwrap();
        
        total_time += validate.end();


        let jit_comp = time("jit_comp");

        // Create JIT compiler context and compile the input
        let mut jit = jitter::backend::jit::JITContext::default();
        jit.translate(validation_context).unwrap();

        total_time += jit_comp.end();

        println!("-----Total running time: {:.2}ms-----", total_time);
    }
}
