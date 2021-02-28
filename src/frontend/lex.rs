// TODO: 
// Look into this: https://github.com/maciejhirsz/logos

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    Binary,
    Box,
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
    Unary,
    Use,
}

// NOTE: Using the lifetime prevents allocations at the cost of one infectious lifetime
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Token {
    Number(usize),
    Ident(String),
    Keyword(Keyword),
    String(String),
    
    At,                 // '@'

    DoubleQuote,        // '"'
    DollarSign,         // '$'
    Carrot,             // '^'
    BackSlash,          // '\'
    Backtick,           // '`'
    Pound,              // '#'

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

    NewLine,            // '\n' (special case)
    Whitespace,         // '\r', '\t', ' ', .. 

    OpenParen,          // '('
    CloseParen,         // ')'

    OpenCurlyBrace,     // '{'
    CloseCurlyBrace,    // '}'

    OpenSquareBracket,  // '['
    CloseSquareBracket, // ']'
}

impl std::fmt::Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let string = match self {
            Token::Number(number) => format!("numeric literal: {}", number),
            Token::Ident(ident) => format!("identifier: {}", ident),
            Token::String(string) => format!("string literal: {}", string),
            Token::Keyword(keyword) => {
                let word = match keyword {
                    Keyword::Binary => "binary",
                    Keyword::Box => "box",
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
                    Keyword::Unary => "unary",
                    Keyword::Use => "use",
                };
                format!("keyword: {}", word)
            },
            Token::At => "@".to_owned(),
            Token::DollarSign => "$".to_owned(),
            Token::Carrot => "^".to_owned(),
            Token::BackSlash => "\\".to_owned(),
            Token::Backtick => "`".to_owned(),
            Token::Pound => "#".to_owned(),
            Token::DoubleQuote => "\"".to_owned(),
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
            Token::NewLine => "NEWLINE".to_owned(), // TODO: These?
            Token::Whitespace => "WHITESPACE".to_owned(),
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

impl Token {
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

impl Token {
    pub fn spanned(self, start_line: usize, start_column: usize, end_line: usize, end_column: usize) -> SpannedToken {
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
pub struct SpannedToken {
    pub token: Token,
    pub span: crate::Span,
}


pub enum PreprocessorState {
    FoundPound,
    Define,
    Include,
    AwaitingNewLine,
    None,
}

pub struct Preprocessor {
    state: PreprocessorState,

    define_from: Token,
    define_to: Vec<Token>,

    include_path: String,
}

impl Preprocessor {
    pub fn new() -> Self {
        Self {
            state: PreprocessorState::None,

            // NOTE: This is just a default. Will be replaced before use
            //       Pound cannot appear in this position, so this safe anyway.
            define_from: Token::Pound,
            
            define_to: Vec::new(),

            include_path: String::with_capacity(0),
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new();
    }
}

pub struct Lexer {
    file_path: String,
    input: String,
    position: usize,

    last_line: usize,
    last_column: usize,
    current_line: usize,
    current_column: usize,

    strip_whitespace: bool,

    preprocessor: Preprocessor,

    // Token replacements (seen_token -> becomes)
    custom_replacements: HashMap<Token, Vec<Token>>,
}

impl Lexer {
    pub fn new(file_path: String, input: String, strip_whitespace: bool) -> Self {
        Self {
            file_path,
            input,
            position: 0,
            last_line: 0,
            last_column: 0,
            current_line: 1,
            current_column: 0,
            strip_whitespace,

            preprocessor: Preprocessor::new(),

            custom_replacements: HashMap::new(),
        }
    }

    pub fn parse_callbacks(&mut self, callbacks: Vec<super::LexerCallback>) {
        for cb in callbacks {
            let mut input_lexer = Self::new("custom input".to_owned(), cb.string.to_owned(), true);
            let mut output_lexer = Self::new("custom output".to_owned(), cb.replacement.to_owned(), true);

            let input = input_lexer.lex();
            let output = output_lexer
                .lex()
                .into_iter()
                .map(|spanned| spanned.token)
                .collect();

            if input.len() != 1 {
                panic!("Only single-token `string`s are currently supported. Found {:?}", input);
            }

            self.custom_replacements.insert(input[0].token.clone(), output);
        }
    }

    fn make_spanned(&self, token: Token) -> SpannedToken {
        token.spanned(self.last_line, self.last_column, self.current_line, self.current_column)
    }

    pub fn reset_last_position(&mut self) {
        self.last_line = self.current_line;
        self.last_column = self.current_column;
    }

    /// Converts the given input to tokens. `file_path` is used only for printing errors.
    pub fn lex_str(file_path: String, input: String, strip_whitespace: bool) -> Vec<SpannedToken> {
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
        self.input.chars().nth(self.position).unwrap()// as char
    }

    /// Returns the next character. Returns `None` if no characters remain.
    fn peek_next(&mut self) -> Result<char, String> {
        self.input.chars().nth(self.position + 1)
            .ok_or(format!("Unexpected EOF: {}:{}:{}", self.file_path, self.current_line, self.current_column))
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
        let next = self.peek_next();


        // FIXME: This is a hack to avoid unexpected EOFs in custom callbacks
        if next.is_err() {
            return Ok(false);
        }
        let next = next.unwrap();


        if next.is_ascii_alphanumeric() || next == '_' {
            Ok(true)
        } else {   
            Ok(false)
        }
    }

    // TODO: Don't exit, but flag the parser to not finish compilation
    pub fn lex(&mut self) -> Vec<SpannedToken> {
        let mut tokens = Vec::new();
        let mut errors = Vec::new();

        while self.position < self.input.len() {
            match self.lex_next_token() {
                Ok(token) => {
                    if self.strip_whitespace {
                        if let Token::Whitespace = token.token {
                            continue;
                        }
                    }

                    // Found `#` in `#directive from to`
                    if let Token::Pound = token.token {
                        if let PreprocessorState::None = self.preprocessor.state {
                            self.preprocessor.state = PreprocessorState::FoundPound;
                        } else {
                            errors.push(format!("Preprocessor directive cannot include `#` symbol"));
                        }
                        continue;
                    }
                    
                    match self.preprocessor.state {
                        // Identify the directive in `#directive`
                        PreprocessorState::FoundPound => {
                            if let Token::Ident(directive) = token.token {
                                match directive.to_lowercase().as_str() {
                                    "define" => self.preprocessor.state = PreprocessorState::Define,
                                    "include" => self.preprocessor.state = PreprocessorState::Include,
    
                                    _ => {
                                        errors.push(format!("Invalid preprocessor directive: `{}`\nValid options are `define`, `include`", directive));
                                    }
                                }
                                continue;
                            }
                        }

                        // Identify `from`
                        PreprocessorState::Define => {
                            // FIXME: Pound is the default value, meaning
                            //        the field is "empty". This could be
                            //        changed to something more clear.
                            if self.preprocessor.define_from == Token::Pound {
                                self.preprocessor.define_from = token.token;
                                self.preprocessor.state = PreprocessorState::AwaitingNewLine;
                            } 
                            continue;
                        }
                        PreprocessorState::Include => {
                            // Get `file` in `#include file`
                            if self.preprocessor.include_path.is_empty() {
                                if let Token::String(string) = token.token {
                                    // self.preprocessor.include_path = string;

                                    
                                    // 0. Convert this source path to actual path
                                    let mut target_path = std::path::PathBuf::from(&self.file_path);
                                    target_path.pop();
                                    target_path.push(&string);
                                    
                                    // 1. Read the file to string
                                    // println!("Inserting file: {:?}", target_path);
                                    let target_source = std::fs::read_to_string(target_path);
                                    if target_source.is_err() {
                                        errors.push(format!("Failed to read file: `./{}`", &string));
                                        continue;
                                    }
                                    let target_source = target_source.unwrap();

                                    // 2. Lex the file, obtaining spanned tokens
                                    let target_tokens = Lexer::lex_str(string, target_source, true);
                                    // 3. Insert the tokens into this lexer (via `tokens.push()`)
                                    target_tokens.into_iter().for_each(|t| tokens.push(t));
        
                                    self.preprocessor.state = PreprocessorState::AwaitingNewLine;
                                }
                            }

                            continue;
                        }

                        PreprocessorState::AwaitingNewLine => {
                            if let Token::NewLine = token.token {
                                
                                // Finalize a `#define`
                                if self.preprocessor.define_from != Token::Pound {
                                    // println!("Registering definition: ({}) -> {:?}", self.preprocessor.define_from, self.preprocessor.define_to);
                                    self.custom_replacements.insert(self.preprocessor.define_from.clone(), self.preprocessor.define_to.clone());
                                }

                                self.preprocessor.reset();
                            } else {
                                // Have `#define item`, look for the actual definition
                                if self.preprocessor.define_from != Token::Pound {
                                    self.preprocessor.define_to.push(token.token);
                                } else { 
                                    // Do not allow dangling tokens
                                    errors.push(format!("Found unexpected token `{}` while waiting for new line.", token.token));
                                }
                            }

                            continue;
                        }

                        // Not looking for anything
                        PreprocessorState::None => {
                            if let Token::NewLine = token.token {
                                if self.strip_whitespace {
                                    continue;
                                }
                            }
                        }
                    }

                    if let Some(rule) = self.custom_replacements.get(&token.token) {
                        let span = token.span;
                        for t in rule {
                            tokens.push(SpannedToken {
                                token: t.clone(),
                                span: span.clone(),
                            });
                        }
                    } else {   
                        tokens.push(token);
                    }
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
    fn lex_next_token(&mut self) -> Result<SpannedToken, String> {
        use Token::*;
        
        self.reset_last_position();

        let token = match self.current() {
            // Ignore whitespace
            it if it.is_whitespace() => {
                // Handle new lines
                if it == '\n' {
                    self.current_column = 0;
                    self.current_line += 1;
                    self.advance();
                    NewLine
                } else {
                    self.advance();
                    Whitespace
                }
            }

            // Attempt to find string literal
            // FIXME: This is a simple, naive approach
            '"' => {
                self.advance();

                let mut string = std::string::String::new();
                // TODO: Bounds check on lexer position
                // TODO: Can set flag upon seeing \ which ignores following "
                while self.current() != '"' {
                    string.push(self.current());
                    self.advance();
                }
                self.advance();

                String(string)
                // DoubleQuote
            }
            '@' => {
                self.advance();
                At
            }
            '`' => {
                self.advance();
                Backtick
            }
            '\\' => {
                self.advance();
                BackSlash
            }
            '^' => {
                self.advance();
                Carrot
            }
            '$' => {
                self.advance();
                DollarSign
            }
            '#' => {
                self.advance();
                Pound
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
                    while self.position < self.input.len() && self.current() != '\n' {
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
                //       but this is faster
                match it {
                    'b' => {
                        // binary
                        if self.is_next('i')? {
                            self.advance();
                            if self.is_next('n')? {
                                self.advance();
                                if self.is_next('a')? {
                                    self.advance();
                                    if self.is_next('r')? {
                                        self.advance();
                                        if self.is_next('y')? {
                                            self.advance();
                                            if !self.is_next_alphanumeric()? {
                                                self.advance();
                                                token = Some(Token::Keyword(self::Keyword::Binary));
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // box
                        if self.is_next('o')? {
                            self.advance();
                            if self.is_next('x')? {
                                self.advance();
                                if !self.is_next_alphanumeric()? {
                                    self.advance();
                                    token = Some(Token::Keyword(self::Keyword::Box));
                                }
                            }
                        }
                    }

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

                    'u' => {
                        // Unary
                        if self.is_next('n')? {
                            self.advance();
                            if self.is_next('a')? {
                                self.advance();
                                if self.is_next('r')? {
                                    self.advance();
                                    if self.is_next('y')? {
                                        self.advance();
                                        if !self.is_next_alphanumeric()? {
                                            self.advance();
                                            token = Some(Token::Keyword(self::Keyword::Unary));
                                        }
                                    }
                                }
                            }
                        }
                        // Use
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

                if token.is_none() {
                    while let Ok(next) = self.peek_next() {
                        if next.is_ascii_alphanumeric() || next == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    
                    let to = self.position;
                    
                    self.advance();
                    
                    token = Some(Ident(self.input[from..=to].into()));
                }

                token.unwrap()
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