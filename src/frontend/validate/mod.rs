pub mod context;
pub mod types;


/// Accepts a newly parsed AST and does the following:
/// 1. Type-Checking & Type Validation
/// 2. Generates an easily-queried context containing information
///    such as functions and their types, structs and their fields, etc.
///    This context is used in codegen
pub fn validate_ast<'input>(ast: super::parse::ast::AST<'input>) -> Result<context::Context<'input>, String> {
    let mut context = context::Context::new();
    context.validate(ast)?;

    Ok(context)
}
