pub mod ast;

pub(self) use super::lex; // for convenience

use crate::Span;
use ast::{Literal, Node};
use lex::{Token, SpannedToken, Keyword};
use crate::frontend::validate::types::Type;

// TODO: Return Results from everything.
// TODO: Handle errors by simply return the expected node, but poisoned.
//       Then, print all errors before exiting.

// TODO: Replace this macro with a printed error message, then return the expected
// token as a poisoned Node to allow the parser to continue.

// TODO: Print the context of the error with the span underlined (like rustc does)

/// Print error and its location, then exit without panic
macro_rules! parser_error {
    ( $path:expr, $span:expr, $($item:expr),+ ) => {
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
    //       hence the looking back. This removes the need to save temp spans without
    //       knowing whether or not they will be needed
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

    fn look_ahead(&self, n: usize) -> &Token {
        &self.tokens[*self.position.borrow() + n].token
    }

    fn advance(&self) {
        *self.position.borrow_mut() += 1;
    }

    // Returns true if there are any unparsed tokens
    fn is_anything_unparsed(&self) -> bool {
        *self.position.borrow() < self.tokens.len()
    }

    ///////////// Parse Functions /////////////

    // TODO: Might want AST to be a type where each TopLevel
    //       is a field containing lists of those items
    pub fn parse_ast(&self) -> ast::AST {
        let mut ast = ast::AST::new();

        while self.is_anything_unparsed() {
            ast.insert_top_level(self.parse_top_level());
        }

        ast
    }

    // TopLevel items are all nodes by themselves
    pub fn parse_top_level(&self) -> ast::TopLevel {
        match self.current_token() {
            Token::Keyword(keyword) => {
                match keyword {
                    Keyword::Extern => {
                        self.advance();
                        ast::TopLevel::ExternBlock(
                            self.parse_extern_block()
                        )
                    }

                    Keyword::Use => {
                        todo!("modules")
                    }

                    Keyword::Fn => {
                        self.advance();

                        ast::TopLevel::Function(
                            self.parse_function_definition()
                        )
                    }

                    Keyword::Trait => {
                        self.advance();

                        ast::TopLevel::Trait(
                            self.parse_trait_definition()
                        )
                    }

                    Keyword::Impl => {
                        self.advance();

                        ast::TopLevel::Impl(
                            self.parse_impl()
                        )
                    }

                    Keyword::Struct => {
                        self.advance();

                        ast::TopLevel::Struct(
                            self.parse_struct_definition()
                        )
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
    }

    pub fn parse_extern_block(&self) -> Node<ast::ExternBlock> {
        // span of `extern` keyword
        let start = self.previous_span();
        let mut externs = Vec::new();

        if let Token::OpenCurlyBrace = self.current_token() {
            self.advance();
            loop {
                if let Token::Keyword(Keyword::Fn) = self.current_token() { 
                    self.advance();

                    externs.push(self.parse_function_prototype());

                    if let Token::Semicolon = self.current_token() {
                        self.advance();
                    } else {
                        parser_error!(self.file_path, self.current_span(), "Expected `;` following extern function prototype. Found `{}`", self.current_token());
                    }
                }
                
                if let Token::CloseCurlyBrace = self.current_token() {
                    self.advance();
                    break;
                }
            }
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected `{{` to begin extern block. Found `{}`", self.current_token());
        }
        
        Node::new(externs, start.extend(*self.previous_span()))
    }

    // TODO: Use this whenever possible for parsing types
    //       Need to look through to see where applicable
    /// Recursively evaluates types
    pub fn parse_type(&self) -> Type { //Result<Type, String> {
        // TODO: `dyn`, `impl`, etc.
        match self.current_token() {
            // `T`
            Token::Ident(ident) => {
                self.advance();
                Type::resolve_builtin(ident)
            }

            // `&T` or `&mut T`
            Token::And => {
                self.advance();
                let mutable = &Token::Keyword(Keyword::Mut) == self.current_token();

                Type::Reference {
                    ty: Box::new(self.parse_type()),
                    mutable,
                }
            }

            // `()` or Tuple
            Token::OpenParen => {
                self.advance();

                // Unit: ()
                if let Token::CloseParen = self.current_token() {
                    self.advance();
                    Type::Unit
                // Tuple: (A, B, C, ..)
                } else {
                    let mut tuple_types = Vec::new();
                    // TODO: Need to prevent loop from eating an entire file
                    // let mut loop_count = 0;
                    loop {
                        // Allows one comma after the final field
                        if let Token::Comma = self.current_token() {
                            parser_error!(self.file_path, self.current_span(), "Only one trailing comma is allowed in tuples following the final parameter");
                        }

                        tuple_types.push(self.parse_type());

                        if let Token::Comma = self.current_token() {
                            self.advance();
                        }
                        if let Token::CloseParen = self.current_token() {
                            self.advance();
                            break;
                        }
                    }

                    Type::Tuple(tuple_types)
                }
            }

            // TODO: Arrays
            Token::OpenSquareBracket => {
                self.advance();
                todo!("arrays")
            }

            // Invalid type component
            x => {
                parser_error!(self.file_path, self.current_span(), "Expected a type component. Found `{}`", x);
            }
        }
    }

    pub fn parse_trait_definition(&self) -> Node<ast::Trait> {
        // span of `trait` keyword
        let start = self.previous_span();

        if let Token::Ident(name) = self.current_token() {
            self.advance();
            if let Token::OpenCurlyBrace = self.current_token() {
                self.advance();

                let mut required_functions = Vec::new();
                let mut default_functions = Vec::new();

                while *self.current_token() != Token::CloseCurlyBrace {
                    match self.current_token() {
                        // TODO: Constants, assosiated types, etc.
                        Token::Keyword(Keyword::Fn) => {
                            let fn_start = self.current_span();
                            self.advance();

                            let prototype = self.parse_function_prototype();
                            // No default implementation
                            if let Token::Semicolon = self.current_token() {
                                self.advance();
                                required_functions.push(prototype);
                            } else {
                                let body = self.parse_expression_block();
                                let function = ast::Function {
                                    prototype,
                                    body,
                                };
                                default_functions.push(Node::new(function, fn_start.extend(*self.previous_span())));
                            }
                        }

                        // Token::Keyword(Keyword::Const) => {}
                        // Token::Keyword(Keyword::Type) => {}

                        _ => {
                            parser_error!(self.file_path, self.current_span(), "Expected one of `fn`, `const`, `type`. Found `{}`", self.current_token());
                        }
                    }
                }
                // Advance past closing `}`
                self.advance();

                let trait_ = ast::Trait {
                    name,
                    default_functions,
                    required_functions,
                };

                Node::new(trait_, start.extend(*self.previous_span()))
            } else {
                parser_error!(self.file_path, self.current_span(), "Expected `{{`, found `{}`", self.current_token());
            }
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected trait identifier. Found `{}`", self.current_token());
        }
    }

    // impl Trait for Type {..}
    // or
    // impl Type {..}
    pub fn parse_impl(&self) -> Node<ast::Impl> {
        // span of `impl` keyword
        let start = self.previous_span();

        // TODO: Create a `parse_impl_body` to simplify this
        // FIXME: Copy/pasted sections
        if let Token::Ident(name1) = self.current_token() {
            self.advance();

            match self.current_token() {
                // impl trait for type {..}
                Token::Keyword(Keyword::For) => {
                    self.advance();
                    
                    if let Token::Ident(target_name) = self.current_token() {
                        self.advance();

                        // FIXME: This body is copy/pasted
                        if let Token::OpenCurlyBrace = self.current_token() {
                            self.advance();

                            let mut functions = Vec::new();
                            // TODO: Constants, etc.
                            while let Token::Keyword(Keyword::Fn) = self.current_token() {
                                self.advance();
                                functions.push(self.parse_function_definition());
                            }

                            if let Token::CloseCurlyBrace = self.current_token() {
                                self.advance();

                                let impl_ = ast::Impl {
                                    // No name implies base impl
                                    trait_name: name1,
                                    target_name,
                                    functions,
                                };

                                Node::new(impl_, start.extend(*self.previous_span()))
                            } else {
                                parser_error!(self.file_path, self.current_span(), "Expected `}}`. Found `{}`", self.current_token());
                            }
                        } else {
                            parser_error!(self.file_path, self.current_span(), "Expected `{{`. Found `{}`", self.current_token());
                        }
                    } else {
                        parser_error!(self.file_path, self.current_span(), "Expected identifier. Found `{}`", self.current_token());
                    }
                }

                // impl type {..}
                // FIXME: This body is duplicated above
                Token::OpenCurlyBrace => {
                    self.advance();

                    let mut functions = Vec::new();
                    // TODO: Constants, etc.
                    while let Token::Keyword(Keyword::Fn) = self.current_token() {
                        self.advance();
                        functions.push(self.parse_function_definition());
                    }

                    if let Token::CloseCurlyBrace = self.current_token() {
                        self.advance();

                        let impl_ = ast::Impl {
                            // No name implies base impl
                            trait_name: "",
                            target_name: name1,
                            functions,
                        };

                        Node::new(impl_, start.extend(*self.previous_span()))
                    } else {
                        parser_error!(self.file_path, self.current_span(), "Expected `}}`. Found `{}`", self.current_token());
                    }
                }

                x => {
                    parser_error!(self.file_path, self.current_span(), "Expected `for` or `{{`. Found `{}`", x);
                }
            }
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected identifier. Found `{}`", self.current_token());
        }
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

                    let field = ast::StructField {
                        field_name,
                        field_type: self.parse_type(),
                    };

                    fields.push(
                        Node::new(field, span.extend(*self.previous_span()))
                    );

                    // parser_error!(self.file_path, self.current_span(), "Expected type parameter type after `:`. Found `{}", self.current_token());
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

    // fn ident(param: type, ..) -> return_type { statements.. }
    pub fn parse_function_definition(&self) -> Node<ast::Function> {
        // span of `fn` keyword
        let start = self.previous_span();

        let prototype = self.parse_function_prototype();
        let body = self.parse_expression_block();

        let function = ast::Function {
            prototype,
            body,
        };

        Node::new(function, start.extend(*self.previous_span()))
    }

    // fn ident(param: type, ..) -> return_type
    pub fn parse_function_prototype(&self) -> Node<ast::FunctionPrototype> {
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

            // `()` type is same as Rust's
            let return_type = if let Token::Minus = self.current_token() {
                self.advance();
                // found `->`
                if let Token::RightAngleBracket = self.current_token() {
                    self.advance();

                    self.parse_type()
                    // parser_error!(self.file_path, self.current_span(), "Expected a return type after `->`. Founds `{}`", self.current_token());
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected `->`. Found `{}`", self.current_token());
                }
            } else {
                // No return type -> unit type (void)
                Type::Unit
            };

            let prototype = ast::FunctionPrototype {
                name,
                parameters,
                return_type,
            };

            Node::new(prototype, start.extend(*self.previous_span()))
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected identifier, found `{}` while parsing function definition", self.current_token());
        }
    }

    // (ident: type, ident: type, ..)
    // `self` becomes `self: Unknown` which later becomes `self: T` for `T`
    pub fn parse_function_parameters(&self) -> Node<ast::FunctionParameterList> {
        let mut parameters = Vec::new();
        // span of `(` token
        let start = self.previous_span();
        
        loop {
            let span = self.current_span();

            let mut mutable = false;

            // Allows one comma after the final field
            if let Token::Comma = self.current_token() {
                parser_error!(self.file_path, self.current_span(), "Only one trailing comma is allowed in function parameters following the final parameter");
            }

            if let Token::Keyword(Keyword::Mut) = self.current_token() {
                self.advance();
                mutable = true;
            }

            // fn ident(&? mut? self, ...)
            if let Token::Keyword(Keyword::Self_) = self.current_token() {
                self.advance();
                let param = ast::FunctionParameter {
                    mutable,
                    name: "self",
                    // FIXME: This will later be replaced with the proper `User` variant. 
                    //        Is there a better approach?
                    ty: Type::Unknown,
                };
                parameters.push(Node::new(param, span.extend(*self.previous_span())));
            }
            else if let Token::Ident(field_name) = self.current_token() {
                self.advance();
                if let Token::Colon = self.current_token() {
                    self.advance();

                    let param = ast::FunctionParameter {
                        mutable,
                        name: field_name,
                        ty: self.parse_type(),
                    };

                    // parser_error!(self.file_path, self.current_span(), "Expected type parameter type after `:`. Found `{}", self.current_token());
                    parameters.push(Node::new(param, span.extend(*self.previous_span())));
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

    // TODO: This function needs a major refactor
    /// Parses a statement terminated by ';'  
    /// Assumes implicit return for non-terminated expressions
    pub fn parse_statement(&self) -> Node<ast::Statement> {
        let statement;
        // span of first statement element (`let` keyword, expression, etc.)
        let start = self.current_span();

        let mut needs_semicolon = true;

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
                let ty;
                let expression;

                if let Token::Ident(ident_) = self.current_token() {
                    self.advance();
                    ident = ident_;

                    // Has `:` -> Type must be specified
                    ty = if let Token::Colon = self.current_token() {
                        self.advance();
                        self.parse_type()
                        // parser_error!(self.file_path, self.current_span(), "Expected type after `:`. Found `{}`", self.current_token());
                    } else {
                        Type::Unknown
                    };

                    // Has `=` -> Must have assignment expression
                    expression = if let Token::Equals = self.current_token() {
                        self.advance();
                        Some(self.parse_expression())
                    } else {
                        None
                    };
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected identifier after `let`. Found `{}`", self.current_token());
                }
                
                statement = ast::Statement::Let {
                    ident,
                    mutable,
                    ty,
                    value: expression,
                };
            }

            // return optional_expr;
            Token::Keyword(Keyword::Return) => {
                self.advance();
                let expression = if let Token::Semicolon = self.current_token() {
                    // there is no expression -> return value is `()`
                    let unit_return = ast::Expression::Literal {
                        value: ast::Literal::UnitType,
                        ty: Type::Unit,
                    };
                    Node::new(unit_return, *self.previous_span())
                } else {
                    self.parse_expression()
                };

                statement = ast::Statement::Return {
                    expression,
                };
            }

            // TODO: Support derefs here too (*a.b.c)
            // FIXME: This needs to be refactored
            Token::Ident(_) => {
                let base = self.parse_expression_field_access();

                // Check whether this is an assignment statement
                let is_assignment = match self.current_token() {
                    Token::Equals => true,
                    Token::Plus | Token::Minus | Token::Asterisk | Token::Slash => {
                        self.look_ahead(1) == &Token::Equals
                    }

                    _ => false,
                };

                match self.current_token() {
                    // x [+=, -=, *=, /=] expression
                    _ if is_assignment => {
                        let op_token = self.current();

                        // Special case (advance past the op in an op-assign)
                        if !(Token::Equals == *self.current_token()) {
                            self.advance();
                        }
                        if let Token::Equals = *self.current_token() {
                            self.advance();
                            let op = ast::AssignmentOp::from_token(op_token);

                            statement = ast::Statement::Assign {
                                // TODO: Replace `variable` with `expression`
                                lhs: base,
                                operator: Node::new(op, op_token.span.extend(*self.previous_span())),
                                expression: self.parse_expression(),
                            };
                        } else {
                            unreachable!() // confirmed this above for `is_assignment`
                            // parser_error!(self.file_path, op_token.span, "Expected `{}=` to create an op-assign statement. Found `{}`", op_token.token, op_token.token);
                        }
                    }

                    // Found: `expression;`
                    Token::Semicolon => {
                        statement = ast::Statement::Expression(base);
                    }

                    // NOTE: Current token is not a semicolon
                    // Return the base expression
                    _ => {
                        statement = if let Token::Semicolon = self.current_token() {
                            ast::Statement::Expression(base)
                        } else {
                            needs_semicolon = false;

                            // TODO: Ensure this is correct
                            ast::Statement::ImplicitReturn {
                                expression: base, //Node::new(ast::Expression::Ident(ident), start.extend(*self.previous_span())),
                                is_function_return: false,
                            }
                        };

                    }
                }
            }

            // Must be an expression
            _ => {
                let expression = self.parse_expression();
                statement = if let Token::Semicolon = self.current_token() {   
                    // Terminated by semicolon
                    ast::Statement::Expression(expression)
                } else {
                    needs_semicolon = false;
                    // Not terminated -> assume implicit return
                    ast::Statement::ImplicitReturn {
                        expression,
                        is_function_return: false,
                    }
                };
            }
        }

        // expects semicolon
        if let Token::Semicolon = self.current_token() {
            self.advance();
        } else if needs_semicolon {
            parser_error!(self.file_path, self.current_span(), "Expected `;` to terminate a statement. Found `{}`", self.current_token());
        }

        Node::new(statement, start.extend(*self.previous_span()))
    }

    // NOTE: Special case (not technically an expression)
    fn parse_expression_block(&self) -> Node<ast::BlockExpression> {
        // Starting `{`
        let start = self.current_span();

        let body = if let Token::OpenCurlyBrace = self.current_token() {
            self.advance();

            let mut body = Vec::new();
    
            loop {
                if let Token::CloseCurlyBrace = self.current_token() {
                    self.advance();
                    break;
                }
    
                // This will not allow an infinite loop
                body.push(self.parse_statement());
            }

            Node::new(body, start.extend(*self.previous_span()))
        } else {
            parser_error!(self.file_path, self.current_span(), "Expected `{{` to form a statement block implementing a function. Found `{}`", self.current_token());
        };

        let block_expression = ast::BlockExpression {
            block: body,
            ty: Type::Unknown,
        };

        Node::new(block_expression, start.extend(*self.previous_span()))
    }

    ////////////////// ONLY EXPRESSIONS BELOW HERE ///////////////////
    ////////// Precedence: lowest at top, highest at bottom //////////
    ////////// Helper functions go after base_expression    //////////

    // Employs recursive descent
    fn parse_expression(&self) -> Node<ast::Expression> {
        self.parse_expression_additive()
    }

    // Precedence for [+, -]
    fn parse_expression_additive(&self) -> Node<ast::Expression> {
        let start = self.current_span();
        let mut expression = self.parse_expression_multiplicative();

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
                        ty: Type::Unknown,
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
        let start = self.current_span();
        let mut expression = self.parse_expression_unary();

        loop {
            match self.current_token() {
                Token::Asterisk | Token::Slash => {
                    let op_token = self.current();
                    let op = ast::BinaryOp::from_token(op_token);
                    self.advance();

                    let rhs = self.parse_expression_unary();
                    let expr = ast::Expression::BinaryExpression {
                        lhs: Box::new(expression),
                        op: Node::new(op, op_token.span),
                        rhs: Box::new(rhs),
                        ty: Type::Unknown,
                    };
                    expression = Node::new(expr, start.extend(*self.previous_span()));
                }

                _ => break,
            }
        }

        expression
    }

    // Precedence for [negation, not]
    fn parse_expression_unary(&self) -> Node<ast::Expression> {
        let start = self.current_span();
        let expression;

        match self.current_token() {
            Token::Minus => {
                self.advance();
                
                // TODO: If next token is number, return negative literal
                if let Token::Number(number) = self.current_token() {
                    expression = self.parse_numeric_literal(number, true);
                } else {   
                    expression = ast::Expression::UnaryExpression {
                        op: Node::new(ast::UnaryOp::Negate, *self.previous_span()),
                        expr: Box::new(self.parse_expression()),
                        ty: Type::Unknown,
                    };
                }
            }

            Token::Bang => {
                self.advance();
                expression = ast::Expression::UnaryExpression {
                    op: Node::new(ast::UnaryOp::Not, *self.previous_span()),
                    expr: Box::new(self.parse_expression()),
                    ty: Type::Unknown,
                };
            }

            _ => {
                return self.parse_expression_field_access();
            }
        }

        Node::new(expression, start.extend(*self.previous_span()))
    }

    // `a.b.c` ..etc.
    // Should always have 3rd highest precedence (below method call and base expressions)
    fn parse_expression_field_access(&self) -> Node<ast::Expression> {
        let start = self.current_span();
        // let base = self.parse_expression_method_call();
        let mut base = self.parse_expression_base();
        
        // FIXME: There is a nice recursive solution here that I can't figure out
        //        (although nothing is wrong with this implementation)
        loop {
            if let Token::Dot = self.current_token() {
                self.advance();
                
                // TODO: Can parse method calls in this branch too
                //       might want to keep separate function for readability though
                if let Token::Ident(ident) = self.current_token() {
                    self.advance();

                    let access = ast::Expression::FieldAccess {
                        base_expr: Box::new(base),
                        field: ident,
                        ty: Type::Unknown,
                    };
                    base = Node::new(access, start.extend(*self.current_span()));
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected ident to create field access. Found `{}`", self.current_token());
                }
            } else {
                break;
            }
        }

        base
    }

    // `a.b()`
    // Should always have 2nd highest precedence (below base expression)
    fn parse_expression_method_call(&self) -> Node<ast::Expression> {
        todo!()
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

                // `()` -> unit type
                if let Token::CloseParen = self.current_token() {
                    self.advance();
                    let expr = ast::Expression::Literal {
                        value: Literal::UnitType,
                        ty: Type::Unit,
                    };

                    return Node::new(expr, start.extend(*self.previous_span()));
                }


                let inner = self.parse_expression();
                if let Token::CloseParen = self.current_token() {
                    self.advance();
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected ')' to end parenthesized expression. Found `{}`", self.current_token());
                }
                return inner;
            }

            // Numeric literal
            Token::Number(number) => {
                expression = self.parse_numeric_literal(number, false);
            }

            // Identifier, Constructor, or FunctionCall
            Token::Ident(ident) => {
                self.advance();

                match self.current_token() {
                    // FieldConstructor
                    Token::OpenCurlyBrace => {
                        expression = self.parse_field_constructor(ident);
                    }

                    // FunctionCall
                    Token::OpenParen => {
                        let inputs = self.parse_function_call_inputs();
                        expression = ast::Expression::FunctionCall {
                            name: ident,
                            inputs,
                            ty: Type::Unknown,
                        };
                    }

                    // Just an identifier
                    _ => {
                        expression = ast::Expression::Ident {
                            name: ident,
                            ty: Type::Unknown,
                        };
                    }
                }
            }

            _ => {
                parser_error!(self.file_path, self.current_span(), "Expected a base expression. Found `{}`", self.current_token());
            }
        }

        Node::new(expression, start.extend(*self.previous_span()))
    }

    //////////////////// EXPRESSION HELPER FUNCTIONS ////////////////////

    fn parse_function_call_inputs(&self) -> Vec<Node<ast::Expression>> {
        // Eat opening `(`
        self.advance();

        let mut inputs = Vec::new();

        loop {
            // Allow trailing comma
            if let Token::Comma = self.current_token() {
                parser_error!(self.file_path, self.current_span(), "Only one trailing comma is allowed in function call inputs following the final parameter");
            }

            if let Token::CloseParen = self.current_token() {
                self.advance();
                break;
            }

            inputs.push(self.parse_expression());

            if let Token::Comma = self.current_token() {
                self.advance();
            }
        }

        inputs
    }

    fn parse_numeric_literal(&self, number: &usize, negative: bool) -> ast::Expression {
        self.advance();

        // `#.` -> must be floating point number
        if let Token::Dot = self.current_token() {
            self.advance();
            let mut float: f64 = if let Token::Number(decimal) = self.current_token() {
                self.advance();
                // `#.#`
                format!("{}.{}", number, decimal).parse().expect("parse float")
            } else {
                // Don't allow `#.type` to avoid struct-field confusion
                if let Token::Ident(_) = self.current_token() {
                    parser_error!(self.file_path, self.current_span(), "Floating point numbers with trailing decimal points cannot have type specifiers (use `1.0f32` or `1f32` instead of `1.f32`)");
                }

                // `#.` -> `#.0`
                format!("{}.0", number).parse().expect("parse float no decimal")
            };

            if negative {
                float = -float;
            }

            // `#.#type`
            let ty = if let Token::Ident(type_specifier) = self.current_token() {
                self.advance();
                let float_type = Type::resolve_builtin(type_specifier);
                if !float_type.is_float() {
                    parser_error!(self.file_path, self.previous_span(), "`{}` is not a valid floating-point type specifier", type_specifier);
                }

                float_type
            } else {
                Type::Unknown
            };

            ast::Expression::Literal {
                value: Literal::Float(float),
                ty,
            }
        }
        // No decimal -> could be any builtin
        else {
            let ty = if let Token::Ident(type_specifier) = self.current_token() {
                self.advance();
                let specified = Type::resolve_builtin(type_specifier);
                if !specified.is_numeric() {
                    parser_error!(self.file_path, self.previous_span(), "`{}` is not a valid type specifier", type_specifier);
                }
                
                // Only signed integers and floats can be negative
                if negative && !(specified.is_signed_integer() || specified.is_float()) {
                    parser_error!(self.file_path, self.previous_span(), "Only signed types can be negative (got unsigned type, `{}`)", type_specifier);
                }

                specified
            } else {
                Type::Unknown
            };

            let value = if ty.is_integer() { // parse integer
                let mut integer: isize = number.to_string().parse().expect("parse integer");

                // This is already confirmed valid
                if negative {
                    integer = -integer;
                }

                Literal::Integer(integer)
            } else { // Already confirmed that only other case is float
                let float = format!("{}", number).parse().expect("parse float");
                Literal::Float(float)
            };

            ast::Expression::Literal {
                value,
                ty,
            }
        }
    }

    // Helper function -- separated for readability/complexity
    // Note that FieldConstructor is an expression
    fn parse_field_constructor(&self, ident: &'a str) -> ast::Expression {
        self.advance();
        let mut fields = std::collections::HashMap::new();

        loop {
            if let Token::Comma = self.current_token() {
                parser_error!(self.file_path, self.current_span(), "Only one trailing comma is allowed in field constructors following the final parameter");
            }
            
            if let Token::Ident(field_name) = self.current_token() {
                self.advance();
                
                // Get the assignment either by ident shorthand or by `field: value`
                let field_assignment = if let Token::Colon = self.current_token() {
                    self.advance();
                    self.parse_expression()
                }
                // Shorthand by identifier
                else if (self.current_token() == &Token::Comma) || (self.current_token() == &Token::CloseCurlyBrace) {
                    Node::new(
                        ast::Expression::Ident { 
                            name: field_name,
                            ty: Type::Unknown,
                        },
                        // Span of the ident token
                        *self.previous_span()
                    )    
                } else {
                    parser_error!(self.file_path, self.current_span(), "Expected `:` after field name. Found {}", self.current_token());
                };

                fields.insert(*field_name, field_assignment)
                    .map(|_existing| {
                        parser_error!(self.file_path, self.current_span(), "Field `{}` was already defined", self.current_token());
                    });
            } else {
                parser_error!(self.file_path, self.current_span(), "Expected field name identifier. Found {}", self.current_token());
            }

            if let Token::Comma = self.current_token() {
                self.advance();
            }
            if let Token::CloseCurlyBrace = self.current_token() {
                self.advance();
                break;
            }
        }

        ast::Expression::FieldConstructor {
            ty: Type::User(ident),
            fields,
        }
    }
}