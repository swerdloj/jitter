mod ast;

use crate::lex::SpannedToken;
use crate::lex::Token;
use crate::lex::Keyword;

macro_rules! parser_error {
    ($message:expr, $token:expr) => {
        // TODO: Print span with file name, etc. as well as error message
        panic!($message, $token);
    };
}

pub struct Parser<'a> {
    tokens: Vec<SpannedToken<'a>>,
    // RefCell allows for referencing current/next while also advancing the position
    position: std::cell::RefCell<usize>,
}

// NOTE: Parse functions expect to parse the desired token.
//       This means that their first indicating token was already seen.
//
//       For example, `parse_function_definition` will not check whether the
//       current token is the `fn` keyword. Instead, it will simply parse
//       the remainder of the item.
impl<'a> Parser<'a> {
    pub fn new(tokens: Vec<SpannedToken<'a>>) -> Self {
        Self {
            tokens,
            position: std::cell::RefCell::new(0),
        }
    }

    fn peek_next(&self) -> &Token {
        &self.tokens[*self.position.borrow() + 1].token
    }

    fn current(&self) -> &Token {
        &self.tokens[*self.position.borrow()].token
    }

    fn advance(&self) {
        *self.position.borrow_mut() += 1;
    }

    pub fn parse_remainder(&self) -> ast::AST {
        let mut ast = Vec::new();

        while *self.position.borrow() < self.tokens.len() {
            ast.push(self.parse_top_level())
        }

        ast
    }

    pub fn parse_top_level(&self) -> ast::TopLevel {
        match self.current() {
            Token::Keyword(keyword) => {
                match keyword {
                    Keyword::Fn => {
                        self.advance();

                        return ast::TopLevel::Function(
                            self.parse_function_definition()
                        );
                    }

                    kw => {
                        parser_error!("Expected one of `fn`, `const`. Found unexpected keyword `{:?}`", kw);
                    }
                }
            }

            // Not a valid TopLevel item
            token => {
                parser_error!("Expected a function definition, const declaration. Found unexpected token `{:?}`", token);
            }
        }
    }

    // fn ident(param: type, ..) -> return_type { statements }
    pub fn parse_function_definition(&self) -> ast::Function {
        if let Token::Ident(name) = self.current() {
            self.advance();

            let parameters = if let Token::OpenParen = self.current() {
                self.advance();
                self.parse_function_parameters()
            } else {
                panic!("Expected `(` after function name. Found `{:?}`", self.current());
            };

            let return_type = if let Token::Minus = self.current() {
                self.advance();

                if let Token::RightAngleBracket = self.current() {
                    self.advance();

                    if let Token::Ident(return_type) = self.current() {
                        self.advance();
                        Some(*return_type)
                    } else {
                        parser_error!("Expected a return type after `->`. Founds `{:?}`", self.current());
                    }
                } else {
                    parser_error!("Expected `->`. Found `{:?}`", self.current());
                }
            } else {
                None
            };

            let statements = if let Token::OpenCurlyBrace = self.current() {
                self.advance();
                self.parse_statement_block()
            } else {
                parser_error!("Expected `{{` to form a statement block implementing a function. Found `{:?}`", self.current());
            };

            ast::Function {
                name,
                parameters,
                return_type,
                statements,
            }
        } else {
            parser_error!("Expected identifier, found `{:?}` while parsing function definition", self.current());
        }
    }

    pub fn parse_function_parameters(&self) -> Vec<ast::FunctionParameters> {
        let mut parameters = Vec::new();
        
        loop {
            // Allows one comma after the final field
            if let Token::Comma = self.current() {
                parser_error!("Only one additional comma is allowed in function parameters following the final parameter{}", "");
            }

            if let Token::Ident(field_name) = self.current() {
                self.advance();
                if let Token::Colon = self.current() {
                    self.advance();
                    if let Token::Ident(field_type) = self.current() {
                        self.advance();
                        parameters.push(
                            ast::FunctionParameters {
                                field_name,
                                field_type,
                            }
                        );
                    } else {
                        parser_error!("Expected parameter type after `:`. Found `{:?}", self.current());
                    }
                } else {
                    parser_error!("Expected `:` after function parameter. Found `{:?}`", self.current());
                }
            }

            if let Token::Comma = self.current() {
                self.advance();
                continue;
            }

            break;
        }

        if let Token::CloseParen = self.current() {
            self.advance();
        } else {
            parser_error!("Expected `)` to end function parameter list. Found `{:?}`", self.current());
        }

        parameters
    }

    pub fn parse_statement_block(&self) -> Vec<ast::Statement> {
        let mut statements = Vec::new();
        
        while !(Token::CloseCurlyBrace == *self.current()) {
            statements.push(self.parse_statement());
        }
        self.advance();
        
        statements
    }

    pub fn parse_statement(&self) -> ast::Statement {
        let statement;

        // let mut ident: type = expr;
        if Token::Keyword(Keyword::Let) == *self.current() {
            self.advance();

            let mutable = if Token::Keyword(Keyword::Mut) == *self.current() {
                self.advance();
                true
            } else {
                false
            };

            let ident;
            let type_;
            let expression;

            if let Token::Ident(ident_) = self.current() {
                self.advance();
                ident = ident_;

                type_ = if let Token::Colon = self.current() {
                    self.advance();
                    if let Token::Ident(type_) = self.current() {
                        self.advance();
                        Some(*type_)
                    } else {
                        parser_error!("Expected type after `:`. Found `{:?}`", self.current());
                    }
                } else {
                    None
                };

                expression = if let Token::Equals = self.current() {
                    self.advance();
                    Some(self.parse_expression())
                } else {
                    None
                };
            } else {
                parser_error!("Expected identifier after `let`. Found `{:?}`", self.current());
            }

            if let Token::Semicolon = self.current() {
                self.advance();
            } else {
                parser_error!("Expected `;` to terminate a statement. Found `{:?}`", self.current());
            }

            statement = ast::Statement::Let {
                ident,
                mutable,
                type_,
                value: expression,
            };
        } else {
            parser_error!("Expected a statement. Found `{:?}`", self.current());
        }

        statement
    }

    // TODO: this
    fn parse_expression(&self) -> ast::Expression {
        let expression;

        // TEMP: for testing
        if let Token::Number(number) = self.current() {
            self.advance();
            expression = ast::Expression::Literal(ast::Literal::Number(*number));
        } else {
            parser_error!("Expected an expression. Found `{:?}`", self.current());
        }

        expression
    }
}