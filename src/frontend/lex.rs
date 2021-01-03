// TODO: 
// Look into this: https://github.com/maciejhirsz/logos

#[derive(Debug, PartialEq)]
pub enum Keyword {
    Extern,
    Enum,
    For,
    Fn,
    Impl,
    Let,
    Mut,
    Pub,
    Return,
    Self_,
    Struct,
    Trait,
    Use,
}

// NOTE: Using the lifetime prevents allocations at the cost of one infectious lifetime
#[derive(Debug, PartialEq)]
pub enum Token<'input> {
    Number(usize),
    Ident(&'input str),
    Keyword(Keyword),
    
    At,                 // '@'

    Minus,              // '-'
    Plus,               // '+'
    Asterisk,           // '*'
    Slash,              // '/'
    Equals,             // '='

    LeftAngleBracket,   // '<'
    RightAngleBracket,  // '>'

    Dot,                // '.'
    Comma,              // ','
    Colon,              // ':'
    Semicolon,          // ';'

    And,                // '&'
    Bang,               // '!'
    Pipe,               // '|'

    Whitespace,         // '\r', '\n', '\t', ' ', .. 

    OpenParen,          // '('
    CloseParen,         // ')'

    OpenCurlyBrace,     // '{'
    CloseCurlyBrace,    // '}'

    OpenSquareBracket,  // '['
    CloseSquareBracket, // ']'
}

impl<'input> std::fmt::Display for Token<'input> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Token::Number(number) => format!("numeric literal: {}", number),
            Token::Ident(ident) => format!("identifier: {}", ident),
            Token::Keyword(keyword) => {
                let word = match keyword {
                    Keyword::Extern => "extern",
                    Keyword::Enum => "enum",
                    Keyword::For => "for",
                    Keyword::Fn => "fn",
                    Keyword::Impl => "impl",
                    Keyword::Let => "let",
                    Keyword::Mut => "mut",
                    Keyword::Pub => "pub",
                    Keyword::Return => "return",
                    Keyword::Self_ => "self",
                    Keyword::Struct => "struct",
                    Keyword::Trait => "trait",
                    Keyword::Use => "use",
                };
                format!("keyword: {}", word)
            },
            Token::At => "@".to_owned(),
            Token::Minus => "-".to_owned(),
            Token::Plus => "+".to_owned(),
            Token::Asterisk => "*".to_owned(),
            Token::Slash => "/".to_owned(),
            Token::Equals => "=".to_owned(),
            Token::LeftAngleBracket => "<".to_owned(),
            Token::RightAngleBracket => ">".to_owned(),
            Token::Dot => ".".to_owned(),
            Token::Comma => ",".to_owned(),
            Token::Colon => ":".to_owned(),
            Token::Semicolon => ";".to_owned(),
            Token::Bang => "!".to_owned(),
            Token::Pipe => "|".to_owned(),
            Token::And => "&".to_owned(),
            Token::Whitespace => panic!("TODO: Display whitespace?"),
            Token::OpenParen => "(".to_owned(),
            Token::CloseParen => ")".to_owned(),
            Token::OpenCurlyBrace => "{".to_owned(),
            Token::CloseCurlyBrace => "}".to_owned(),
            Token::OpenSquareBracket => "[".to_owned(),
            Token::CloseSquareBracket => "]".to_owned(),
        };

        write!(f, "{}", string)
    }
}

impl<'input> Token<'input> {
    pub fn is_number(&self) -> bool {
        if let Token::Number(_) = self { true } else { false }
    }
    pub fn is_ident(&self) -> bool {
        if let Token::Ident(_) = self { true } else { false }
    }
    pub fn is_keyword(&self) -> bool {
        if let Token::Keyword(_) = self { true } else { false }
    }
}

impl<'input> Token<'input> {
    pub fn spanned(self, start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> SpannedToken<'input> {
        SpannedToken {
            token: self,
            span: crate::Span {
                start_line,
                start_column,
                end_line,
                end_column,
            },
        }
    }
}

#[derive(Debug)]
pub struct SpannedToken<'input> {
    pub token: Token<'input>,
    pub span: crate::Span,
}

pub struct Lexer<'input> {
    file_path: &'input str,
    input: &'input str,
    bytes: &'input [u8],
    position: usize,

    last_line: usize,
    last_column: usize,
    current_line: usize,
    current_column: usize,

    strip_whitespace: bool,
}

impl<'input> Lexer<'input> {
    pub fn new(file_path: &'input str, input: &'input str, strip_whitespace: bool) -> Self {
        Self {
            file_path,
            input,
            bytes: input.as_bytes(),
            position: 0,
            last_line: 0,
            last_column: 0,
            current_line: 1,
            current_column: 0,
            strip_whitespace,
        }
    }

    fn make_spanned<'a>(&self, token: Token<'a>) -> SpannedToken<'a> {
        token.spanned(self.last_line, self.last_column, self.current_line, self.current_column)
    }

    pub fn reset_last_position(&mut self) {
        self.last_line = self.current_line;
        self.last_column = self.current_column;
    }

    /// Converts the given input to tokens. `file_path` is used only for printing errors.
    pub fn lex_str(file_path: &'input str, input: &'input str, strip_whitespace: bool) -> Vec<SpannedToken<'input>> {
        Lexer::new(file_path, input, strip_whitespace).lex()
    }

    // TODO: Bounds check?
    /// Advances the lexer forward one character
    fn advance(&mut self) {
        self.position += 1;
        self.current_column += 1;
    }

    /// Returns the character at the current position
    fn current(&mut self) -> char {
        self.bytes[self.position] as char
    }

    /// Returns the next character. Returns `None` if no characters remain.
    fn peek_next(&mut self) -> Result<char, String> {
        self.bytes.get(self.position + 1)
            .map(|byte| *byte as char)
            .ok_or("Unexpected EOF".to_owned())
    }

    /// Returns true if the next character is the desired character
    fn is_next(&mut self, c: char) -> Result<bool, String> {
        if self.peek_next()? == c {
            Ok(true)
        } else {   
            Ok(false)
        }
    }

    /// Returns whether the next character is an ascii letter, number, or underscore
    fn is_next_alphanumeric(&mut self) -> Result<bool, String> {
        let next = self.peek_next()?;
        if next.is_ascii_alphanumeric() || next == '_' {
            Ok(true)
        } else {   
            Ok(false)
        }
    }

    // TODO: Don't exit, but flag the parser to not finish compilation
    pub fn lex(&mut self) -> Vec<SpannedToken<'input>> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        while self.position < self.bytes.len() {
            match self.lex_next_token() {
                Ok(token) => {
                    if self.strip_whitespace {
                        if let Token::Whitespace = token.token {
                            continue;
                        }
                    }
        
                    tokens.push(token);
                }

                Err(err) => {
                    errors.push(err);
                    continue;
                }
            }
        }

        if errors.len() > 0 {
            println!("Lexing errors: \n");
            for error in errors {
                println!("{}", error);
            }
            // TEMP: This should just be a flag to return
            std::process::exit(1);
        }

        tokens
    }

    /// Lexes the input, returning spanned tokens
    fn lex_next_token(&mut self) -> Result<SpannedToken<'input>, String> {
        use Token::*;
        
        self.reset_last_position();

        let token = match self.current() {
            // Ignore whitespace
            it if it.is_whitespace() => {
                // Handle new lines
                if it == '\n' {
                    self.current_column = 0;
                    self.current_line += 1;
                }

                self.advance();

                Whitespace
            }

            '@' => {
                self.advance();
                At
            }
            '+' => {
                self.advance();
                Plus
            }
            '-' => {
                self.advance();
                Minus
            }
            // slash or single-line comment
            '/' => {
                self.advance();
                if self.current() == '/' {
                    while self.position < self.bytes.len() && self.current() != '\n' {
                        self.advance();
                    }
                    // don't advance here to re-use whitespace logic
                    Whitespace
                } else {   
                    Slash
                }
            }
            '*' => {
                self.advance();
                Asterisk
            }
            '=' => {
                self.advance();
                Equals
            }
            '.' => {
                self.advance();
                Dot
            }
            ',' => {
                self.advance();
                Comma
            }
            ':' => {
                self.advance();
                Colon
            }
            ';' => {
                self.advance();
                Semicolon
            }
            '&' => {
                self.advance();
                And
            }
            '!' => {
                self.advance();
                Bang
            }
            '|' => {
                self.advance();
                Pipe
            }
            '(' => {
                self.advance();
                OpenParen
            }
            ')' => {
                self.advance();
                CloseParen
            }
            '{' => {
                self.advance();
                OpenCurlyBrace
            }
            '}' => {
                self.advance();
                CloseCurlyBrace
            }
            '[' => {
                self.advance();
                OpenSquareBracket
            }
            ']' => {
                self.advance();
                CloseSquareBracket
            }
            '<' => {
                self.advance();
                LeftAngleBracket
            }
            '>' => {
                self.advance();
                RightAngleBracket
            }

            // Ident or Keyword
            it if it.is_ascii_alphabetic() || it == '_' => {
                let from = self.position;

                let mut token = None;

                // TODO: This could be simplified quite a bit. Consider a macro?
                // NOTE: Could just treat everything as idents, then check those for keywords,
                //       but this is much faster
                match it {
                    'e' => {
                        // enum
                        if self.is_next('n')? {
                            self.advance();
                            if self.is_next('u')? {
                                self.advance();
                                if self.is_next('m')? {
                                    self.advance();
                                    if !self.is_next_alphanumeric()? {
                                        self.advance();
                                        token = Some(Token::Keyword(self::Keyword::Enum));
                                    }
                                }
                            }
                        // extern
                        } else if self.is_next('x')? {
                            self.advance();
                            if self.is_next('t')? {
                                self.advance();
                                if self.is_next('e')? {
                                    self.advance();
                                    if self.is_next('r')? {
                                        self.advance();
                                        if self.is_next('n')? {
                                            self.advance();
                                            if !self.is_next_alphanumeric()? {
                                                self.advance();
                                                token = Some(Token::Keyword(self::Keyword::Extern));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    'f' => {
                        // fn
                        if self.is_next('n')? {
                            self.advance();
                            if !self.is_next_alphanumeric()? {
                                self.advance();
                                token = Some(Token::Keyword(self::Keyword::Fn));
                            }
                        // for
                        } else if self.is_next('o')? {
                            self.advance();
                            if self.is_next('r')? {
                                self.advance();
                                if !self.is_next_alphanumeric()? {
                                    self.advance();
                                    token = Some(Token::Keyword(self::Keyword::For));
                                }
                            }
                        }
                    }

                    // impl
                    'i' => {
                        if self.is_next('m')? {
                            self.advance();
                            if self.is_next('p')? {
                                self.advance();
                                if self.is_next('l')? {
                                    self.advance();
                                    if !self.is_next_alphanumeric()? {
                                        self.advance();
                                        token = Some(Token::Keyword(self::Keyword::Impl));
                                    }
                                }
                            }
                        }
                    }

                    // let
                    'l' => {
                        if self.is_next('e')? {
                            self.advance();
                            if self.is_next('t')? {
                                self.advance();
                                if !self.is_next_alphanumeric()? {
                                    self.advance();
                                    token = Some(Token::Keyword(self::Keyword::Let));
                                }
                            }
                        }
                    }

                    // mut
                    'm' => {
                        if self.is_next('u')? {
                            self.advance();
                            if self.is_next('t')? {
                                self.advance();
                                if !self.is_next_alphanumeric()? {
                                    self.advance();
                                    token = Some(Token::Keyword(self::Keyword::Mut));
                                }
                            }
                        }
                    }

                    // pub
                    'p' => {
                        if self.is_next('u')? {
                            self.advance();
                            if self.is_next('b')? {
                                self.advance();
                                if !self.is_next_alphanumeric()? {
                                    self.advance();
                                    token = Some(Token::Keyword(self::Keyword::Pub));
                                }
                            }
                        }
                    }

                    // return
                    'r' => {
                        if self.is_next('e')? {
                            self.advance();
                            if self.is_next('t')? {
                                self.advance();
                                if self.is_next('u')? {
                                    self.advance();
                                    if self.is_next('r')? {
                                        self.advance();
                                        if self.is_next('n')? {
                                            self.advance();
                                            if !self.is_next_alphanumeric()? {
                                                self.advance();
                                                token = Some(Token::Keyword(self::Keyword::Return));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    's' => {
                        // self
                        if self.is_next('e')? {
                            self.advance();
                            if self.is_next('l')? {
                                self.advance();
                                if self.is_next('f')? {
                                    self.advance();
                                    if !self.is_next_alphanumeric()? {
                                        self.advance();
                                        token = Some(Token::Keyword(self::Keyword::Self_));
                                    }
                                }
                            }
                        }
                        // struct
                        if self.is_next('t')? {
                            self.advance();
                            if self.is_next('r')? {
                                self.advance();
                                if self.is_next('u')? {
                                    self.advance();
                                    if self.is_next('c')? {
                                        self.advance();
                                        if self.is_next('t')? {
                                            self.advance();
                                            if !self.is_next_alphanumeric()? {
                                                self.advance();
                                                token = Some(Token::Keyword(self::Keyword::Struct));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // trait
                    't' => {
                        if self.is_next('r')? {
                            self.advance();
                            if self.is_next('a')? {
                                self.advance();
                                if self.is_next('i')? {
                                    self.advance();
                                    if self.is_next('t')? {
                                        self.advance();
                                        if !self.is_next_alphanumeric()? {
                                            self.advance();
                                            token = Some(Token::Keyword(self::Keyword::Trait));
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Use
                    'u' => {
                        if self.is_next('s')? {
                            self.advance();
                            if self.is_next('e')? {
                                self.advance();
                                if !self.is_next_alphanumeric()? {
                                    self.advance();
                                    token = Some(Token::Keyword(self::Keyword::Use));
                                }
                            }
                        }
                    }

                    // Not a keyword => identifier
                    _ => {}
                }

                if let Some(token) = token {
                    token
                } else {
                    while let Ok(next) = self.peek_next() {
                        if next.is_ascii_alphanumeric() || next == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    let to = self.position;
                    
                    self.advance();
                    
                    Ident(&self.input[from..=to])
                }
            }

            it if it.is_digit(10) => {
                let from = self.position;

                while let Ok(next) = self.peek_next() {
                    // Underscores are allowed in numbers
                    if next.is_digit(10) || next == '_' {
                        self.advance();
                    } else if !next.is_digit(10) {
                        break;
                    }
                }

                let to = self.position;

                self.advance();

                // Remove underscores, then parse
                Number(self.input[from..=to].to_owned().replace("_", "").parse().unwrap())
            }

            // TODO: Read all invalid characters in a row and return only one error for such cases
            invalid => {
                self.advance();
                return Err(format!("{}:{}:{}: Invalid character: `{}`", self.file_path, self.current_line, self.current_column, invalid));
            }
        }; // end `let token = match`

        Ok(self.make_spanned(token))
    }
}