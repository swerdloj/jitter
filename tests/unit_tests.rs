#[cfg(test)]
mod tests {
    use parse_test::{
        lex::*,
        parse::*,
    };

    #[test]
    fn test_parser() {
        let path = "./tests/test.lang";
        let input = &std::fs::read_to_string(path).unwrap();

        let tokens = Lexer::lex_str(path, input, true);

        let parser = Parser::new(path, tokens);
        parser.parse_AST();
    }

   #[test]
   fn test_lexer() {
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
            Token::Keyword(Keyword::Fn),
            Token::Keyword(Keyword::For),
            Token::Keyword(Keyword::Struct),
            Token::Keyword(Keyword::Let),
            Token::Keyword(Keyword::Mut),
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