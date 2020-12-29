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

        let child = Expression::Literal(Literal::Integer(7));
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
        let path = "./tests/test.jitter";
        let input = &std::fs::read_to_string(path).unwrap();

        let tokens = Lexer::lex_str(path, input, true);

        let parser = Parser::new(path, tokens);
        parser.parse_ast();
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
            Token::Number(1230),
            Token::Ident("ident"),
            Token::Ident("_0_1"),
            Token::Ident("_1test"),
            Token::Ident("test1_"),
            Token::Keyword(Keyword::Let),
            Token::Keyword(Keyword::Mut),
            Token::Keyword(Keyword::Fn),
            Token::Keyword(Keyword::For),
            Token::Keyword(Keyword::Struct),
            Token::Keyword(Keyword::Enum),
            Token::Keyword(Keyword::Return),
            Token::Keyword(Keyword::Impl),
            Token::Keyword(Keyword::Trait),
            Token::Keyword(Keyword::Self_),
            Token::At,
            Token::Minus,
            Token::Plus,
            Token::Asterisk,
            Token::Slash,
            Token::Equals,
            Token::LeftAngleBracket,
            Token::RightAngleBracket,
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