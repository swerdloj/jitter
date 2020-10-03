use parse_test::lex::Lexer;
use parse_test::parse::Parser;

fn main() {
    let path = "./tests/test.lang";
    let test = &std::fs::read_to_string(path).unwrap();

    let tokens = Lexer::lex_str(path, test, true);
    // println!("Tokens:\n{:?}", tokens);

    let parser = Parser::new(tokens);
    let ast = parser.parse_remainder();
    println!("AST:\n{:#?}", ast);
}