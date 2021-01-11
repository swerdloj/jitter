#[cfg(test)]
mod tests {
    use jitter::{
        frontend::lex::*,
        frontend::parse::*,
        frontend::validate::types::Type,
        Span,
    };

    #[test]
    fn nodes() {
        use ast::*;

        let child = Expression::Literal { 
            value: Literal::Integer(7),
            ty: Type::i32,
        };

        let child_node = Node::new(child, Span::new(0, 0, 0, 0));

        let parent = Expression::UnaryExpression {
            op: Node::new(UnaryOp::Negate, Span::new(0, 0, 0, 0)),
            expr: Box::new(child_node),
            ty: Type::Unknown,
        };

        let _parent_node = Node::new(parent, Span::new(0, 0, 0, 0));
    }

    #[test]
    fn parser() {
        let path = "./tests/parse_test.jitter";
        let input = &std::fs::read_to_string(path).unwrap();

        let tokens = Lexer::lex_str(path, input, true);

        let parser = Parser::new(path, tokens);
        parser.parse_ast("parser_test");
    }

   #[test]
   fn lexer() {
        let path = "./tests/lex_test.txt";
        let test_input = &std::fs::read_to_string(path).unwrap();

        // Remove spans
        let test_tokens = Lexer::lex_str(path, test_input, true)
            .into_iter()
            .map(|spanned| {
                spanned.token
            })
            .collect::<Vec<Token>>();

        let expected = vec![
            // Numbers
            Token::Number(1230),

            Token::Number(321),

            Token::Number(123),
            Token::Dot,
            Token::Number(456),

            Token::Number(99),
            Token::Dot,
            Token::Number(78),

            Token::Number(10),
            Token::Dot,
            Token::Number(88),
            Token::Ident("f32"),

            Token::Number(100),
            Token::Ident("usize"),
            // Identifiers
            Token::Ident("ident"),
            Token::Ident("_0_1"),
            Token::Ident("_1test"),
            Token::Ident("test1_"),
            // Keywords
            Token::Keyword(Keyword::Box),
            Token::Keyword(Keyword::Extern),
            Token::Keyword(Keyword::Enum),
            Token::Keyword(Keyword::For),
            Token::Keyword(Keyword::Fn),
            Token::Keyword(Keyword::Impl),
            Token::Keyword(Keyword::Let),
            Token::Keyword(Keyword::Mut),
            Token::Keyword(Keyword::Pub),
            Token::Keyword(Keyword::Return),
            Token::Keyword(Keyword::Self_),
            Token::Keyword(Keyword::Struct),
            Token::Keyword(Keyword::Trait),
            Token::Keyword(Keyword::Use),
            // Symbols
            Token::At,
            Token::Minus,
            Token::Plus,
            Token::Asterisk,
            Token::Slash,
            Token::Equals,
            Token::LeftAngleBracket,
            Token::RightAngleBracket,
            Token::Dot,
            Token::Comma,
            Token::Colon,
            Token::Semicolon,
            Token::And,
            Token::Bang,
            Token::Pipe,
            Token::OpenParen,
            Token::CloseParen,
            Token::OpenCurlyBrace,
            Token::CloseCurlyBrace,
            Token::OpenSquareBracket,
            Token::CloseSquareBracket,
        ];

        assert_eq!(test_tokens, expected);
    }
}