use parse_test::lex::Lexer;
use parse_test::parse::Parser;

fn main() {
    let path = "./tests/test.lang";
    let input = &std::fs::read_to_string(path).unwrap();

    let tokens = Lexer::lex_str(path, input, true);
    // println!("Tokens:\n{:?}", tokens);

    let parser = Parser::new(path, tokens);
    let ast = parser.parse_AST();
    println!("AST:\n{:#?}", ast);
}