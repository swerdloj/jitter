use super::parse::ast;

/// Accepts a newly parsed AST and does the following:
/// 1. Type-Checking & Type Validation
/// 2. Generates an easily-queried context containing information
///    such as functions and their types, structs and their fields, etc.
///    This context is used in codegen
pub fn validate_ast(ast: ast::AST) {
    for node in ast {
        match node {
            ast::TopLevel::Function(_) => {
                todo!()
            }
            ast::TopLevel::Struct(_) => {
                todo!()
            }
            ast::TopLevel::ConstDeclaration => {
                todo!()
            }
            ast::TopLevel::UseStatement => {
                todo!()
            }
        }
    }
}