#[derive(Debug, PartialEq)]
pub enum Keyword {
    Let,
    Mut,
    Fn,
    For,
    Struct,
}

// NOTE: Using the lifetime prevents allocations at the cost of one infectious lifetime
#[derive(Debug, PartialEq)]
pub enum Token<'input> {
    Number(usize),
    Ident(&'input str),
    Keyword(Keyword),
    
    Minus,              // '-'
    Plus,               // '+'
    Asterisk,           // '*'
    Slash,              // '/'
    Equals,             // '='

    LeftAngleBracket,   // '<'
    RightAngleBracket,  // '>'

    Comma,              // ','
    Colon,              // ':'
    Semicolon,          // ';'

    Whitespace,         // '\r', '\n', ' ', .. 

    OpenParen,          // '('
    CloseParen,         // ')'

    OpenCurlyBrace,     // '{'
    CloseCurlyBrace,    // '}'

    OpenSquareBracket,  // '['
    CloseSquareBracket, // ']'
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
    pub fn spanned(self, line: usize, col: usize) -> SpannedToken<'input> {
        SpannedToken {
            token: self,
            span: crate::Span::new(line, col),
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
    line: usize,
    column: usize,
    strip_whitespace: bool,
}

impl<'input> Lexer<'input> {
    pub fn new(file_path: &'input str, input: &'input str, strip_whitespace: bool) -> Self {
        Self {
            file_path,
            input,
            bytes: input.as_bytes(),
            position: 0,
            line: 1,
            column: 0,
            strip_whitespace,
        }
    }

    /// Converts the given input to tokens. `file_path` is used only for printing errors.
    pub fn lex_str(file_path: &'input str, input: &'input str, strip_whitespace: bool) -> Vec<SpannedToken<'input>> {
        Lexer::new(file_path, input, strip_whitespace).lex()
    }

    // TODO: Bounds check?
    /// Advances the lexer forward one character
    fn advance(&mut self) {
        self.position += 1;
        self.column += 1;
    }

    /// Returns the character at the current position
    fn current(&mut self) -> char {
        self.bytes[self.position] as char
    }

    /// Returns the next character. Returns `None` if no characters remain.
    fn peek_next(&mut self) -> Option<char> {
        self.bytes.get(self.position + 1).map(|byte| *byte as char)
    }

    /// Returns true if the next character is the desired character
    fn is_next(&mut self, c: char) -> bool {
        if let Some(next) = self.peek_next() {
            if next == c {
                return true;
            }
        }

        false
    }

    /// Returns whether the next character is an ascii letter, number, or underscore
    fn is_next_alphanumeric(&mut self) -> bool {
        if let Some(next) = self.peek_next() {
            if next.is_ascii_alphanumeric() || next == '_' {
                return true;
            }
        }

        false
    }

    /// Lexes the input, returning spanned tokens
    pub fn lex(&mut self) -> Vec<SpannedToken<'input>> {
        use Token::*;
    
        let mut tokens = Vec::new();
    
        while self.position < self.bytes.len() {
            let token = match self.current() {
                // Ignore whitespace
                it if it.is_whitespace() => {
                    // Handle new lines
                    if it == '\n' {
                        self.column = 0;
                        self.line += 1;
                    }

                    self.advance();
                    if self.strip_whitespace {  
                        continue;
                    }
                    Whitespace
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
                        while self.current() != '\n' {
                            self.advance();
                        }
                        // don't advance here to re-use whitespace logic
                        continue;
                    }

                    Slash
                }
                '*' => {
                    self.advance();
                    Asterisk
                }
                '=' => {
                    self.advance();
                    Equals
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

                    // TODO: This could be simplified quite a bit. Consider a macro?
                    // NOTE: Could just treat everything as idents, then check those for keywords,
                    //       but this is much faster
                    match it {
                        'f' => {
                            // fn
                            if self.is_next('n') {
                                self.advance();
                                if !self.is_next_alphanumeric() {
                                    self.advance();
                                    tokens.push(Token::Keyword(self::Keyword::Fn).spanned(self.line, self.column));
                                    continue;
                                }
                            // for
                            } else if self.is_next('o') {
                                self.advance();
                                if self.is_next('r') {
                                    self.advance();
                                    if !self.is_next_alphanumeric() {
                                        self.advance();
                                        tokens.push(Token::Keyword(self::Keyword::For).spanned(self.line, self.column));
                                        continue;
                                    }
                                }
                            }
                        }

                        // let
                        'l' => {
                            if self.is_next('e') {
                                self.advance();
                                if self.is_next('t') {
                                    self.advance();
                                    if !self.is_next_alphanumeric() {
                                        self.advance();
                                        tokens.push(Token::Keyword(self::Keyword::Let).spanned(self.line, self.column));
                                        continue;
                                    }
                                }
                            }
                        }

                        // mut
                        'm' => {
                            if self.is_next('u') {
                                self.advance();
                                if self.is_next('t') {
                                    self.advance();
                                    if !self.is_next_alphanumeric() {
                                        self.advance();
                                        tokens.push(Token::Keyword(self::Keyword::Mut).spanned(self.line, self.column));
                                        continue;
                                    }
                                }
                            }
                        }

                        // struct
                        's' => {
                            if self.is_next('t') {
                                self.advance();
                                if self.is_next('r') {
                                    self.advance();
                                    if self.is_next('u') {
                                        self.advance();
                                        if self.is_next('c') {
                                            self.advance();
                                            if self.is_next('t') {
                                                self.advance();
                                                if !self.is_next_alphanumeric() {
                                                    self.advance();
                                                    tokens.push(Token::Keyword(self::Keyword::Struct).spanned(self.line, self.column));
                                                    continue;
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        // Not a keyword => identifier
                        _ => {}
                    }

                    while let Some(next) = self.peek_next() {
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
    
                it if it.is_digit(10) => {
                    let from = self.position;

                    while let Some(next) = self.peek_next() {
                        if next.is_digit(10) {
                            self.advance();
                        } else if next.is_ascii_alphabetic() || next == '_' {
                            // TODO: Allow underscores after numbers
                            panic!("{}:{}:{}: Invalid input: letter or underscore following a number", self.file_path, self.line, self.column);
                        } else {
                            break;
                        }
                    }

                    let to = self.position;

                    self.advance();

                    // FIXME: There must be a better way to do this
                    Number(self.input[from..=to].to_owned().parse().unwrap())
                }
    
                invalid => panic!("{}:{}:{}: Invalid character: `{}`", self.file_path, self.line, self.column, invalid)
            }; // end `let token = match`

            if let Whitespace = token {}
            else {   
                tokens.push(token.spanned(self.line, self.column));
            }
        } // end `loop`
    
        tokens
    }
}