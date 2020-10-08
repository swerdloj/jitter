pub mod ast;

use ast::Node;
use crate::Span;
use crate::lex::{Token, SpannedToken, Keyword};

// TODO: Return Results from everything.
// TODO: Handle errors by simply return the expected node, but poisoned.
//       Then, print all errors before exiting.

// TODO: Replace this macro with a printed error message, then return the expected
// token as a poisoned Node to allow the parser to continue.

/// Print error and its location, then exit without panic
macro_rules! parser_error {
    ( $path:expr, $span:expr, $($item:expr),+) => {
        eprintln!(
            "Parsing Error at {}:{}:{}:\n\n{}\n",
            $path, $span.start_line, $span.start_column,
            format!(  $($item,)+  )
        ); 
        
        std::process::exit(1);
    };
}

pub struct Parser<'a> {
    file_path: &'a str,
    tokens: Vec<SpannedToken<'a>>,
    // Interior mutability allows nesting method calls without worrying about `self` usage
    position: std::cell::RefCell<usize>,
}

// NOTE: Some functions expect to parse only the desired token.
//       This means that their first indicating token was already seen.
//
//       For example, `parse_function_definition` will not check whether the
//       current token is the `fn` keyword. Instead, it will simply parse
//       the remainder of the item.
//       
//       This is why spans begin with the `previous_span`.
//       Similarly, because each parsed item advances the current position,
//       the final span is extended by `previous_span` because current
//       refers to the current token needing evaluation (rather than the most recently
//       parsed token)
impl<'a> Parser<'a> {
    pub fn new(file_path: &'a str, tokens: Vec<SpannedToken<'a>>) -> Self {
        Self {
            file_path,
            tokens,
            position: std::cell::RefCell::new(0),
        }
    }

    // NOTE: This is used to determine certain item spans *after* parsing,
    //       hence the looking back
    fn previous_span(&self) -> &Span {
        &self.tokens[*self.position.borrow()-1].span
    }

    fn current_span(&self) -> &Span {
        &self.tokens[*self.position.borrow()].span
    }

    fn current(&self) -> &SpannedToken {
        &self.tokens[*self.position.borrow()]
    }

    fn current_token(&self) -> &Token {
        &self.current().token
    }

    fn advance(&self) {
        *self.position.borrow_mut() += 1;
    }

    ///////////// Parse Functions /////////////

    #[allow(non_snake_case)]
    pub fn parse_AST(&self) -> ast::AST {
        let mut ast = Vec::new();

        while *self.position.borrow() < self.tokens.len() {
            ast.push(self.parse_top_level());
        }

        ast
    }

    // TopLevel items are all nodes by themselves
    pub fn parse_top_level(&self) -> ast::TopLevel {
        let item;

        match self.current_token() {
            Token::Keyword(keyword) => {
                match keyword {
                    Keyword::Fn => {
                        self.advance();

                        item = ast::TopLevel::Function(
                            self.parse_function_definition()
                        );
                    }

                    Keyword::Struct => {
                        self.advance();

                        item = ast::TopLevel::Struct(
                            self.parse_struct_definition()
                        );
                    }

                    _ => {
                        parser_error!(self.file_path, self.current_span(), "Expected one of TODO:. Found unexpected keyword `{}`", self.current_token());
                    }
                }
            }

            // Not a valid TopLevel item
            _ => {
                parser_error!(self.file_path, self.current_span(), "Expected a TODO:. Found unexpected token `{}`", self.current_token());
            }
        }

        item
    }

    // TODO: Do I want tuple structs and/or unit structs?
    // struct ident {field1: type1, ..}
    pub fn parse_struct_definition(&self) -> Node<ast::Struct> {
        // span of `struct` keyword
        let start = self.previous_span();

        if let Token::Ident(name) = self.current_token() {
            self.advance();
            if let Token::OpenCurlyBrace = self.current_token() {
                self.advance();
                let fields = self.parse_struct_fields();
                let item = ast::Struct {
                    name,
                    fields,
                };

                Node::new(item, start.extend(*self.previous_span()))
            } else {
                parser_error!(self.file_path, self.current_span(), "Expected `{{` after struct name. Found `{}`", self.current_token());
            }
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected identifier after keyword `struct`. Found `{}`", self.current_token());
        }
    }

    pub fn parse_struct_fields(&self) -> Node<ast::StructFieldList> {
        let mut fields = Vec::new();
        // span of `{` token
        let start = self.previous_span();
        
        loop {
            let span = self.current_span();

            // Allows one comma after the final field
            if let Token::Comma = self.current_token() {
                parser_error!(self.file_path, self.current_span(), "Only one trailing comma is allowed after struct fields");
            }

            if let Token::Ident(field_name) = self.current_token() {
                self.advance();
                if let Token::Colon = self.current_token() {
                    self.advance();
                    if let Token::Ident(field_type) = self.current_token() {
                        self.advance();
                        let field = ast::StructField {
                            field_name,
                            field_type,
                        };
                        fields.push(
                            Node::new(field, span.extend(*self.previous_span()))
                        );
                    } else {
                        parser_error!(self.file_path, self.current_span(), "Expected type parameter type after `:`. Found `{}", self.current_token());
                    }
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected `:` after struct field name. Found `{}`", self.current_token());
                }
            }

            if let Token::Comma = self.current_token() {
                self.advance();
                continue;
            }

            break;
        }

        if let Token::CloseCurlyBrace = self.current_token() {
            self.advance();
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected `}}` to end struct declaration. Found `{}`", self.current_token());
        }

        Node::new(fields, start.extend(*self.previous_span()))
    }

    // fn ident(param: type, ..) -> return_type { statements }
    pub fn parse_function_definition(&self) -> Node<ast::Function> {
        // span of `fn` keyword
        let start = self.previous_span();

        if let Token::Ident(name) = self.current_token() {
            self.advance();

            let parameters = if let Token::OpenParen = self.current_token() {
                self.advance();
                self.parse_function_parameters()
            } else {
                panic!("Expected `(` after function name. Found `{}`", self.current_token());
            };

            let return_type = if let Token::Minus = self.current_token() {
                self.advance();

                if let Token::RightAngleBracket = self.current_token() {
                    self.advance();

                    if let Token::Ident(return_type) = self.current_token() {
                        self.advance();
                        Some(*return_type)
                    } else {
                        parser_error!(self.file_path, self.current_span(), "Expected a return type after `->`. Founds `{}`", self.current_token());
                    }
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected `->`. Found `{}`", self.current_token());
                }
            } else {
                None
            };

            let statements = if let Token::OpenCurlyBrace = self.current_token() {
                self.advance();
                self.parse_statement_block()
            } else {
                parser_error!(self.file_path, self.current_span(), "Expected `{{` to form a statement block implementing a function. Found `{}`", self.current_token());
            };

            let function = ast::Function {
                name,
                parameters,
                return_type,
                statements,
            };

            Node::new(function, start.extend(*self.previous_span()))
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected identifier, found `{}` while parsing function definition", self.current_token());
        }
    }

    // (ident: type, ident: type, ..)
    pub fn parse_function_parameters(&self) -> Node<ast::FunctionParameterList> {
        let mut parameters = Vec::new();
        // span of `(` token
        let start = self.previous_span();
        
        loop {
            let span = self.current_span();

            let mut mutable = false;

            // Allows one comma after the final field
            if let Token::Comma = self.current_token() {
                parser_error!(self.file_path, self.current_span(), "Only one additional comma is allowed in function parameters following the final parameter");
            }

            if let Token::Keyword(Keyword::Mut) = self.current_token() {
                self.advance();
                mutable = true;
            }

            if let Token::Ident(field_name) = self.current_token() {
                self.advance();
                if let Token::Colon = self.current_token() {
                    self.advance();
                    if let Token::Ident(field_type) = self.current_token() {
                        self.advance();
                        let param = ast::FunctionParameter {
                            mutable,
                            field_name,
                            field_type,
                        };
                        parameters.push(Node::new(param, span.extend(*self.previous_span())));
                    } else {
                        parser_error!(self.file_path, self.current_span(), "Expected type parameter type after `:`. Found `{}", self.current_token());
                    }
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected `:` after function parameter. Found `{}`", self.current_token());
                }
            }

            if let Token::Comma = self.current_token() {
                self.advance();
                continue;
            }

            break;
        }

        if let Token::CloseParen = self.current_token() {
            self.advance();
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected `)` to end function parameter list. Found `{}`", self.current_token());
        }

        Node::new(parameters, start.extend(*self.previous_span()))
    }

    pub fn parse_statement_block(&self) -> Node<ast::StatementBlock> {
        let mut statements = Vec::new();
        // span of `{` token
        let start = self.previous_span();

        loop {
            if let Token::CloseCurlyBrace = self.current_token() {
                self.advance();
                break;
            }

            // This will not allow an infinite loop
            statements.push(self.parse_statement());
        }
        
        Node::new(statements, start.extend(*self.previous_span()))
    }

    pub fn parse_statement(&self) -> Node<ast::Statement> {
        // expects semicolon in a reusable code bit
        let expect_semicolon = || {
            if let Token::Semicolon = self.current_token() {
                self.advance();
            } else {
                parser_error!(self.file_path, self.current_span(), "Expected `;` to terminate a statement. Found `{}`", self.current_token());
            }
        };

        let statement;
        // span of first statement element (`let` keyword, expression, etc.)
        let start = self.current_span();

        match self.current_token() {
            // let mut ident: type = expr;
            Token::Keyword(Keyword::Let) => {
                self.advance();

                let mutable = if Token::Keyword(Keyword::Mut) == *self.current_token() {
                    self.advance();
                    true
                } else {
                    false
                };

                let ident;
                let type_;
                let expression;

                if let Token::Ident(ident_) = self.current_token() {
                    self.advance();
                    ident = ident_;

                    type_ = if let Token::Colon = self.current_token() {
                        self.advance();
                        if let Token::Ident(type_) = self.current_token() {
                            self.advance();
                            Some(*type_)
                        } else {
                            parser_error!(self.file_path, self.current_span(), "Expected type after `:`. Found `{}`", self.current_token());
                        }
                    } else {
                        None
                    };

                    expression = if let Token::Equals = self.current_token() {
                        self.advance();
                        Some(self.parse_expression())
                    } else {
                        None
                    };
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected identifier after `let`. Found `{}`", self.current_token());
                }
                
                expect_semicolon();

                statement = ast::Statement::Let {
                    ident,
                    mutable,
                    type_,
                    value: expression,
                };
            }

            // TODO: `.` access, function calls
            Token::Ident(ident) => {
                self.advance();
                match self.current_token() {
                    // x [+=, -=, *=, /=] expression
                    Token::Equals
                    | Token::Plus
                    | Token::Minus 
                    | Token::Asterisk
                    | Token::Slash => {
                        let op_token = self.current();

                        // Special case (advance past the op in an op-assign)
                        if !(Token::Equals == *self.current_token()) {
                            self.advance();
                        }
                        if let Token::Equals = *self.current_token() {
                            self.advance();
                            let op = ast::AssignmentOp::from_token(op_token);

                            statement = ast::Statement::Assign {
                                variable: ident,
                                operator: Node::new(op, op_token.span.extend(*self.previous_span())),
                                expression: self.parse_expression(),
                            };

                            expect_semicolon();
                        } else {
                            parser_error!(self.file_path, self.current_span(), "Expected `=` to create an op-assign statement. Found `{}`", self.current_token());
                        }
                    }

                    // x;
                    Token::Semicolon => {
                        parser_error!(self.file_path, self.current_span(), "Identifier as statement does nothing");
                    }

                    _ => {
                        parser_error!(self.file_path, self.current_span(), "Expected beginning of statement. Found `{}`", self.current_token());
                    }
                }
            }

            _ => {
                // TODO: Could it be an expression?
                parser_error!(self.file_path, self.current_span(), "Expected a statement. Found `{}`", self.current_token());
            }
        }

        Node::new(statement, start.extend(*self.previous_span()))
    }

    //////////////// ONLY EXPRESSIONS BELOW THIS LINE ////////////////
    ////////// Precedence: Lowest at top, highest at bottom //////////

    // Employs recursive descent
    fn parse_expression(&self) -> Node<ast::Expression> {
        self.parse_expression_additive()
    }

    // Precedence for [+, -]
    fn parse_expression_additive(&self) -> Node<ast::Expression> {
        let mut expression = self.parse_expression_multiplicative();
        // Span begins with the previous expression
        let start = expression.span.clone();

        // loop => associative
        // Note that the expression is built up with each iteration
        loop {
            match self.current_token() {
                Token::Plus | Token::Minus => {
                    let op_token = self.current();
                    let op = ast::BinaryOp::from_token(op_token);
                    self.advance();


                    let rhs = self.parse_expression_multiplicative();
                    let expr = ast::Expression::BinaryExpression {
                        lhs: Box::new(expression),
                        op: Node::new(op, op_token.span),
                        rhs: Box::new(rhs),
                    };
                    expression = Node::new(expr, start.extend(*self.previous_span()));
                }

                _ => break,
            }
        }

        expression
    }

    // Precedence for [*, /]
    fn parse_expression_multiplicative(&self) -> Node<ast::Expression> {
        let mut expression = self.parse_expression_base();
        // Span begins with the previous expression
        let start = expression.span.clone();

        loop {
            match self.current_token() {
                Token::Asterisk | Token::Slash => {
                    let op_token = self.current();
                    let op = ast::BinaryOp::from_token(op_token);
                    self.advance();

                    let rhs = self.parse_expression_base();
                    let expr = ast::Expression::BinaryExpression {
                        lhs: Box::new(expression),
                        op: Node::new(op, op_token.span),
                        rhs: Box::new(rhs),
                    };
                    expression = Node::new(expr, start.extend(*self.previous_span()));
                }

                _ => break,
            }
        }

        expression
    }

    // Precedence for [parentheticals, literals, identifiers]
    fn parse_expression_base(&self) -> Node<ast::Expression> {
        let expression;
        // This is a terminal item, so span contains the element about to be parsed
        let start = self.current_span();

        // Base cases
        match self.current_token() {
            // ( expression )
            Token::OpenParen => {
                self.advance();
                expression = ast::Expression::Parenthesized(
                    Box::new(self.parse_expression())
                );
                if let Token::CloseParen = self.current_token() {
                    self.advance();
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected ')' to end parenthesized expression. Found `{}`", self.current_token());
                }
            }

            // Numeric literal
            Token::Number(number) => {
                self.advance();
                expression = ast::Expression::Literal(ast::Literal::Number(*number));
            }

            // Identifier
            Token::Ident(ident) => {
                self.advance();
                expression = ast::Expression::Ident(ident);
            }

            _ => {
                parser_error!(self.file_path, self.current_span(), "Expected a base expression. Found `{}`", self.current_token());
            }
        }

        Node::new(expression, start.extend(*self.previous_span()))
    }
}