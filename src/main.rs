fn main() {
    let test = 
    "-1 + 3 * 4 - 2"
    ;

    let tokens = Lexer::lex_str(test);

    println!("Tokens:\n{:?}", tokens);
}

struct Lexer<'input> {
    input: &'input str,
    bytes: &'input [u8],
    position: usize,
}

impl<'input> Lexer<'input> {
    pub fn new(input: &'input str) -> Self {
        Self {
            input,
            bytes: input.as_bytes(),
            position: 0,
        }
    }

    pub fn lex_str(input: &'input str) -> Vec<Token> {
        Lexer::new(input).lex_remaining()
    }

    // TODO: Bounds check
    fn advance(&mut self, n: usize) {
        self.position += n;
    }

    fn current(&mut self) -> char {
        self.bytes[self.position] as char
    }

    fn next(&mut self) -> Option<char> {
        self.bytes.get(self.position + 1).map(|byte| *byte as char)
    }

    pub fn lex_remaining(&mut self) -> Vec<Token> {
        use Token::*;
        use crate::Literal::*;
    
        let mut tokens = Vec::new();
    
        while self.position < self.bytes.len() {
            let token = match self.current() {
                // Ignore whitespace
                it if it.is_whitespace() => {
                    self.advance(1);
                    Whitespace
                }

                '+' => {
                    self.advance(1);
                    Plus
                }

                '-' => {
                    self.advance(1);
                    Minus
                }

                '/' => {
                    self.advance(1);
                    Slash
                }

                '*' => {
                    self.advance(1);
                    Asterisk
                }
    
                it if it.is_digit(10) => {
                    let from = self.position;

                    while let Some(next) = self.next() {
                        if next.is_digit(10) {
                            self.advance(1);
                        } else {
                            break;
                        }
                    }

                    let to = self.position;

                    self.advance(1);

                    // FIXME: There must be a better way to do this
                    Literal(Number(std::str::from_utf8(&self.bytes[from..=to]).unwrap().to_owned().parse().unwrap()))
                }
    
                invalid => panic!("Invalid character: `{}`", invalid)
            };

            if let Whitespace = token {}
            else {   
                tokens.push(token);
            }
        }
    
        tokens
    }
}

#[derive(Debug)]
enum Token {
    Literal(Literal),
    Minus,
    Plus,
    Asterisk,
    Slash,

    Whitespace,
}

#[derive(Debug)]
enum Literal {
    Number(usize),
}

enum Expression {
    BinaryExpression {
        lhs: Box<Expression>,
        op: BinaryOp,
        rhs: Box<Expression>,
    },

    UnaryExpression {
        op: UnaryOp,
        expr: Box<Expression>,
    },

    Literal(Literal),
}

enum UnaryOp {
    Negate,
}

enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}