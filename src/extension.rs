pub use crate::frontend::parse::ast::*;
pub use crate::frontend::validate::types::Type;


type TopLevelMetaFn = fn(Item, Vec<&str>) -> ExtensionResult;
type StatementMetaFn = fn(Statement, Vec<&str>) -> ExtensionResult;

pub(crate) struct Extension {
    lib: libloading::Library,
}

impl Extension {
    pub(crate) unsafe fn new(extension_path: &str) -> Self {
        let lib = libloading::Library::new(extension_path)
            .expect(&format!("Failed to load lib: `{}`", extension_path));

        Self {
            lib,
        }
    }

    pub(crate) fn transform_top_level(&self, item: TopLevel, inputs: Vec<&str>) -> ExtensionResult {
        let meta = unsafe {
            self.lib.get::<TopLevelMetaFn>(b"transform_top_level")
                .map_err(|e| e.to_string())?
        };

        let top = match item {
            TopLevel::Function(f) => {
                Item::Function(f.item)
            }
            TopLevel::Struct(s) => {
                Item::Struct(s.item)
            }
            // TODO: These
            // TopLevel::ExternBlock(_) => {}
            // TopLevel::Operator(_, _) => {}
            // TopLevel::Trait(_) => {}
            // TopLevel::Impl(_) => {}
            // TopLevel::Use(_) => {}
            // TopLevel::ConstDeclaration => {}
            _ => {
                return Err(format!("Invalid meta item: `{:?}`", item));
            }
        };

        meta(top, inputs)
    }

    pub(crate) fn transform_statement(&self, statement: Statement, inputs: Vec<&str>) -> ExtensionResult {
        let meta = unsafe {
            self.lib.get::<StatementMetaFn>(b"transform_statement")
                .map_err(|e| e.to_string())?
        };

        meta(statement, inputs)
    }
}


pub type ExtensionResult = Result<Vec<Item>, String>;

pub trait Nodify<T> : Sized {
    /// Turns anything into an arbitrary AST `Node`
    fn nodify(self) -> Node<Self>;
}

impl<T> Nodify<T> for T {
    fn nodify(self) -> Node<Self> {
        // FIXME: Create a way to specify spans as being meta-created?
        Node::new(self, crate::Span::new(0, 0, 0, 0))
    }
}

/// Valid AST items for meta usage
pub enum Item {
    Function(Function),
    Struct(Struct),
    Statement(Statement),
    // TODO: This may not be fully integrated within the compiler
    // Block(ast::BlockExpression),
}

impl Into<TopLevel> for Item {
    fn into(self) -> TopLevel {
        match self {
            Item::Function(f) => TopLevel::Function(f.nodify()),
            Item::Struct(s) => TopLevel::Struct(s.nodify()),
            Item::Statement(s) => todo!("Handle statement"),
        }
    }
}