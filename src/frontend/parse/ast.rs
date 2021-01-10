use super::lex::{SpannedToken, Token};
use crate::frontend::validate::types::Type;


#[derive(Debug, Clone)]
pub struct Node<NodeType> {
    pub item: NodeType,
    pub span: crate::Span,
    // TODO: This flag might not be needed (just knowing at least one error exists is enough)
    pub is_error_recovery_node: bool,
}

// Removes the need to add `.item` everywhere
impl<T> std::ops::Deref for Node<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.item
    }
}
impl<T> std::ops::DerefMut for Node<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.item
    }
}

impl<T> Node<T> {
    pub fn new(item: T, span: crate::Span) -> Self {
        Self {
            item,
            span,
            is_error_recovery_node: false,
        }
    }

    pub fn poison(mut self) -> Self {
        self.is_error_recovery_node = true;
        self
    }
}

///////////////// AST VARIANTS /////////////////


/// An AST represented as indexed parts  
/// i.e.: All `TopLevel` items exist as fields which can be iterated over
///
/// For example, to see all functions defined in the AST,
/// see `AST.functions`
// TODO: Might want to make these HashMaps instead of Vecs
//       for convenience
pub struct AST<'input> {
    pub externs:   Vec<Node<ExternBlock<'input>>>,
    pub functions: Vec<Node<Function<'input>>>,
    pub traits:    Vec<Node<Trait<'input>>>,
    pub impls:     Vec<Node<Impl<'input>>>,
    pub structs:   Vec<Node<Struct<'input>>>,
    pub uses:      Vec<Node<Use<'input>>>,
     // TODO: These
    // pub constants: Vec<Node<()>>,
}

impl<'input> AST<'input> {
    pub fn new() -> Self {
        Self {
            externs:   Vec::new(),
            functions: Vec::new(),
            traits:    Vec::new(),
            impls:     Vec::new(),
            structs:   Vec::new(),
            uses:      Vec::new(),
        }
    }

    /// Create a placeholder AST with no heap allocations
    pub(crate) fn placeholder() -> Self {
        Self {
            externs:   Vec::with_capacity(0),
            functions: Vec::with_capacity(0),
            traits:    Vec::with_capacity(0),
            impls:     Vec::with_capacity(0),
            structs:   Vec::with_capacity(0),
            uses:      Vec::with_capacity(0),
        }
    }

    // FIXME: This is a bit of indirection that can be avoided by simply
    //        using `parse_top_level` to directly insert into the AST
    //        (rather than going through `TopLevel`)
    pub fn insert_top_level(&mut self, item: TopLevel<'input>) {
        match item {
            TopLevel::ExternBlock(i) => self.externs.push(i),
            TopLevel::Function(i) => self.functions.push(i),
            TopLevel::Trait(i) => self.traits.push(i),
            TopLevel::Impl(i) => self.impls.push(i),
            TopLevel::Struct(i) => self.structs.push(i),
            TopLevel::Use(i) => self.uses.push(i),
            // TopLevel::ConstDeclaration => self.constants.(i),
            _ => todo!(),
        }
    }
}

#[derive(Debug)]
pub enum TopLevel<'input> {
    ExternBlock(Node<ExternBlock<'input>>),
    Function(Node<Function<'input>>),
    Trait(Node<Trait<'input>>),
    Impl(Node<Impl<'input>>),
    Struct(Node<Struct<'input>>),
    Use(Node<Use<'input>>),
    ConstDeclaration,
}

#[derive(Debug)]
pub struct Use<'input> {
    // a::b::c becomes [a, b, c]
    pub path: Vec<&'input str>,
}

pub type ExternBlock<'input> = Vec<Node<FunctionPrototype<'input>>>;

#[derive(Debug)]
pub struct Function<'input> {
    pub prototype: Node<FunctionPrototype<'input>>,
    pub body: Node<BlockExpression<'input>>,
    pub is_public: bool,
}

#[derive(Debug)]
pub struct Trait<'input> {
    pub name: &'input str,
    pub default_functions: Vec<Node<Function<'input>>>,
    pub required_functions: Vec<Node<FunctionPrototype<'input>>>,
    pub is_public: bool,
    // TODO: Constants, associated types, etc.
}

#[derive(Debug)]
pub struct Impl<'input> {
    pub trait_name: &'input str,
    pub target_name: &'input str,
    pub functions: Vec<Node<Function<'input>>>,
    // TODO: Constants, etc.
}

#[derive(Debug)]
pub struct FunctionPrototype<'input> {
    pub name: &'input str,
    pub parameters: Node<FunctionParameterList<'input>>,
    pub return_type: Type<'input>,
}

#[derive(Debug)]
pub struct Struct<'input> {
    pub name: &'input str,
    pub fields: Node<StructFieldList<'input>>,
    pub is_public: bool,
}

pub type StructFieldList<'input> = Vec<Node<StructField<'input>>>;

#[derive(Debug)]
pub struct StructField<'input> {
    pub name: &'input str,
    pub ty: Type<'input>,
    pub is_public: bool,
}

pub type FunctionParameterList<'input> = Vec<Node<FunctionParameter<'input>>>;

#[derive(Debug)]
pub struct FunctionParameter<'input> {
    pub mutable: bool,
    pub name: &'input str,
    pub ty: Type<'input>,
}

#[derive(Debug, Clone)]
pub enum Statement<'input> {
    Let {
        ident: &'input str,
        mutable: bool,
        ty: Type<'input>,
        value: Option<Node<Expression<'input>>>,
    },

    Assign {
        lhs: Node<Expression<'input>>,
        operator: Node<AssignmentOp>,
        expression: Node<Expression<'input>>,
    },

    ImplicitReturn {
        expression: Node<Expression<'input>>,
        is_function_return: bool,
    },

    Return {
        expression: Node<Expression<'input>>,
    },

    Expression(Node<Expression<'input>>),
}

#[derive(Debug, Clone)]
pub enum Expression<'input> {
    BinaryExpression {
        lhs: Box<Node<Expression<'input>>>,
        op: Node<BinaryOp>,
        rhs: Box<Node<Expression<'input>>>,
        ty: Type<'input>,
    },

    UnaryExpression {
        op: Node<UnaryOp>,
        expr: Box<Node<Expression<'input>>>,
        ty: Type<'input>,
    },

    /// Constructor for a type with fields
    FieldConstructor {
        // Name of type
        ty: Type<'input>,
        // Map of (field_name -> value)
        fields: std::collections::HashMap<&'input str, Node<Expression<'input>>>,
    },

    /// Accessing a field of a type
    FieldAccess {
        /// The `lhs` of `lhs.field`
        base_expr: Box<Node<Expression<'input>>>,
        /// The field identifier being used
        field: &'input str,
        /// The type of this FieldAccess (the field's type)
        ty: Type<'input>,
    },

    // MethodCall {
    //     ty: Type<'input>,
    // }

    FunctionCall {
        /// Name of function being called
        name: &'input str,
        /// Expressions passed as input to the function (in order)
        inputs: Vec<Node<Expression<'input>>>,
        /// Type returned by the function
        ty: Type<'input>,
    },

    Block(BlockExpression<'input>),

    Literal { 
        value: Literal,
        ty: Type<'input>,
    },

    Ident {
        name: &'input str,
        ty: Type<'input>,
    },
}

impl<'input> Expression<'input> {
    /// Returns the type of the expression as known at that time.  
    /// Type may be unknown for non-validated expressions
    pub fn get_type(&self) -> &Type<'input> {
        match self {
            Expression::BinaryExpression { ty, .. } => ty,
            Expression::UnaryExpression { ty, .. } => ty,
            Expression::FieldConstructor { ty, .. } => ty,
            Expression::FieldAccess { ty, .. } => ty,
            Expression::FunctionCall { ty, .. } => ty,
            Expression::Block(block) => &block.ty,
            Expression::Literal { ty, .. } => ty,
            Expression::Ident { ty, .. } => ty,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockExpression<'input> {
    pub block: Node<Vec<Node<Statement<'input>>>>,
    pub ty: Type<'input>,
}

#[derive(Debug, Clone)]
pub enum Literal {
    /// Integer of any type
    Integer(isize),
    /// Floating point number of any type
    Float(f64),
    /// `()` type
    UnitType, 
}

#[derive(Debug, Clone)]
pub enum UnaryOp {
    Negate,
    Not,
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
}

impl BinaryOp {
    pub fn from_token(symbol_token: &SpannedToken) -> Self {
        match symbol_token.token {
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Subtract,
            Token::Asterisk => BinaryOp::Multiply,
            Token::Slash => BinaryOp::Divide,

            _ => panic!("Cannot create BinaryOp from {:?}", symbol_token),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssignmentOp {
    Assign,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
}

impl AssignmentOp {
    pub fn from_token(op_token: &SpannedToken) -> Self {
        match op_token.token {
            Token::Equals => AssignmentOp::Assign,
            Token::Plus => AssignmentOp::AddAssign,
            Token::Minus => AssignmentOp::SubtractAssign,
            Token::Asterisk => AssignmentOp::MultiplyAssign,
            Token::Slash => AssignmentOp::DivideAssign,

            _ => panic!("Cannot create AssignmentOp from {:?}", op_token),
        }
    }
}