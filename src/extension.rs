// TODO: This


pub mod prelude {
    pub use crate::frontend::parse::ast;
    pub use super::Item;
}

use crate::frontend::parse::ast;

/// Valid AST items for meta usage
pub enum Item {
    Function(ast::Function),
    Struct(ast::Struct),
    Statement(ast::Statement),
    // TODO: This may not be fully integrated within the compiler
    // Block(ast::BlockExpression),
}

