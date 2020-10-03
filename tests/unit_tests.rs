#[cfg(test)]
mod tests {
    use parse_test::*;

    #[test]
    fn test_parser() {

    }

   #[test]
   fn test_lexer() {
        use lex::*;

        let test_input = "123 \t -7    for_ =_test * y2 /\r\n _3_:; fn\n (for){}let[] <>";
        let test_tokens = Lexer::lex_str("", test_input, true)
            .into_iter()
            .map(|spanned| {
                spanned.token
            })
            .collect::<Vec<Token>>();

        let expected = vec![
            Token::Number(123),
            Token::Minus,
            Token::Number(7),
            Token::Ident("for_"),
            Token::Equals,
            Token::Ident("_test"),
            Token::Asterisk,
            Token::Ident("y2"),
            Token::Slash,
            Token::Ident("_3_"),
            Token::Colon,
            Token::Semicolon,
            Token::Keyword(lex::Keyword::Fn),
            Token::OpenParen,
            Token::Keyword(lex::Keyword::For),
            Token::CloseParen,
            Token::OpenCurlyBrace,
            Token::CloseCurlyBrace,
            Token::Keyword(lex::Keyword::Let),
            Token::OpenSquareBracket,
            Token::CloseSquareBracket,
            Token::LeftAngleBracket,
            Token::RightAngleBracket,
        ];

        assert_eq!(test_tokens, expected);
    }
}