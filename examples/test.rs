use parse_test::frontend::{
    lex::Lexer, 
    parse::Parser
};

use parse_test::backend::codegen::JITContext;

fn main() {
    let path = "./tests/integration_test.lang";
    let input = &std::fs::read_to_string(path).unwrap();

    let tokens = Lexer::lex_str(path, input, true);
    // println!("Tokens:\n{:#?}", tokens);

    let parser = Parser::new(path, tokens);
    let ast = parser.parse_ast();
    // println!("AST:\n{:#?}", ast);


    let mut jit = JITContext::new();
    let todo = jit.translate(ast).unwrap();

    // TEMP: for testing
    unsafe {
        let todo: fn(i32, i32) -> i32 = std::mem::transmute(jit.get_fn("multiply"));
        println!("Call: {}", todo(1, 2));
    }

    // TODO: growable environment (REPL-style)
    // let env = Environment::new(jit);
}

/*

TODO:

#[lang(link)]
extern "lang" { 
    fn multiply(i32, i32) -> i32; 
}

would generate the following:

fn multiply(a: i32, b: i32) -> i32 {
    let func: fn(i32, i32) -> i32 = 
        std::mem::transmute(static_jit_context.get("multiply"));

    func(a, b)
}

*/