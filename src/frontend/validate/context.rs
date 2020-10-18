use std::collections::HashMap;

use crate::frontend::parse::ast;

use super::types::{self, Type};

struct StructDefinition<'input> {
    fields: Vec<(&'input str, Type<'input>)>,
}

struct FunctionDefinition<'input> {
    // (name, type, mutable)
    parameters: Vec<(&'input str, Type<'input>, bool)>,
    return_type: Type<'input>,
}


pub struct Context<'input> {
    functions: HashMap<String, FunctionDefinition<'input>>,
    structs: HashMap<String, StructDefinition<'input>>
}

impl<'a> Context<'a> {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            structs: HashMap::new(),
        }
    }

    pub fn register_struct(&mut self, struct_: ast::Struct<'a>) -> Result<(), String> {
        self.structs.insert(
            struct_.name.to_owned(),
            StructDefinition {
                fields: struct_.fields.item.iter().map(|node| {
                    let field_type = types::resolve(node.item.field_type);
                    (node.item.field_name, field_type)
                }).collect()
            }
        ).map(|_already_existing| {
            return Err::<(), String>(format!("Struct `{}` is already defined", struct_.name));
        });

        Ok(())
    }

    pub fn register_function(&mut self, function: ast::Function<'a>) -> Result<(), String> {
        // Registers a function's name and assigns internal types
        self.functions.insert(
            function.name.to_owned(),
            FunctionDefinition {
                parameters: function.parameters.item.iter().map(|node| {
                        let field_name = node.item.field_name;
                        let field_type = types::resolve(node.item.field_type);
                        (field_name, field_type, node.item.mutable)
                    }).collect(),
                return_type: types::resolve(function.return_type)
            }
        ).map(|_already_existing| {
            return Err::<(), String>(format!("Function `{}` is already defined", function.name));
        });

        for statement in function.statements.item {
            self.validate_statement(statement.item)?;
        }


        Ok(())
    }

    pub fn validate_statement(&mut self, statement: ast::Statement<'a>) -> Result<(), String> {
        match statement {
            ast::Statement::Let { ident, mutable, type_, value } => {
                todo!()
            }

            ast::Statement::Assign { variable, operator, expression } => {
                todo!()
            }

            ast::Statement::Return { expression } => {
                todo!()
            }

            ast::Statement::Expression(expr) => {
                todo!()
            }
        }

        Ok(())
    }
}