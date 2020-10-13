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

    unsafe {
        let todo: fn(i32, i32) -> i32 = std::mem::transmute(jit.get_fn("multiply"));
        println!("Call: {}", todo(1, 2));
    }

    // TODO: 
    // let env = Environment::new(jit);
}