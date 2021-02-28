use super::lex::{SpannedToken, Token};
use crate::frontend::validate::types::Type;


#[derive(Debug, Clone)]
pub struct Node<NodeType> {
    pub item: NodeType,
    pub span: crate::Span,
    // TODO: This flag might not be needed (just knowing at least one error exists is enough)
    // pub is_error_recovery_node: bool,
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
            // is_error_recovery_node: false,
        }
    }

    // pub fn poison(mut self) -> Self {
    //     self.is_error_recovery_node = true;
    //     self
    // }
}

///////////////// AST VARIANTS /////////////////


/// An AST represented as indexed parts  
/// i.e.: All `TopLevel` items exist as fields which can be iterated over
///
/// For example, to see all functions defined in the AST,
/// see `AST.functions`
// TODO: Might want to make these HashMaps instead of Vecs
//       for convenience
#[derive(Debug)]
pub struct AST {
    pub module:    String,
    pub externs:   Vec<Node<ExternBlock>>,
    pub functions: Vec<Node<Function>>,
    pub operators: Vec<Operator>,
    pub traits:    Vec<Node<Trait>>,
    pub impls:     Vec<Node<Impl>>,
    pub structs:   Vec<Node<Struct>>,
    pub uses:      Vec<Node<Use>>,
     // TODO: These
    // pub constants: Vec<Node<()>>,
}

impl AST {
    pub fn new(module: String) -> Self {
        Self {
            module,
            externs:   Vec::new(),
            functions: Vec::new(),
            operators: Vec::new(),
            traits:    Vec::new(),
            impls:     Vec::new(),
            structs:   Vec::new(),
            uses:      Vec::new(),
        }
    }

    /// Create a placeholder AST with no heap allocations
    pub(crate) fn placeholder() -> Self {
        Self {
            module:    String::with_capacity(0),
            externs:   Vec::with_capacity(0),
            functions: Vec::with_capacity(0),
            operators: Vec::with_capacity(0),
            traits:    Vec::with_capacity(0),
            impls:     Vec::with_capacity(0),
            structs:   Vec::with_capacity(0),
            uses:      Vec::with_capacity(0),
        }
    }

    // FIXME: This is a bit of indirection that can be avoided by simply
    //        using `parse_top_level` to directly insert into the AST
    //        (rather than going through `TopLevel`)
    pub fn insert_top_level(&mut self, item: TopLevel) {
        match item {
            TopLevel::ExternBlock(i) => self.externs.push(i),
            TopLevel::Function(i) => self.functions.push(i),
            TopLevel::Operator(o, f) => {
                self.operators.push(o.item);
                self.functions.push(f);
            },
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
pub enum TopLevel {
    ExternBlock(Node<ExternBlock>),
    Function(Node<Function>),
    Operator(Node<Operator>, Node<Function>),
    Trait(Node<Trait>),
    Impl(Node<Impl>),
    Struct(Node<Struct>),
    Use(Node<Use>),
    ConstDeclaration,
}

#[derive(Debug)]
pub struct Use {
    // a::b::c becomes [a, b, c]
    pub path: Vec<String>,
}

pub type ExternBlock = Vec<Node<FunctionPrototype>>;

#[derive(Debug)]
pub struct Function {
    pub prototype: Node<FunctionPrototype>,
    pub body: Node<BlockExpression>,
    pub is_public: bool,
}

#[derive(Debug)]
pub struct Operator {
    pub pattern: Vec<Token>,
    pub associated_function: String,
    pub is_binary: bool,
    pub is_public: bool,
}

#[derive(Debug)]
pub struct Trait {
    pub name: String,
    pub default_functions: Vec<Node<Function>>,
    pub required_functions: Vec<Node<FunctionPrototype>>,
    pub is_public: bool,
    // TODO: Constants, associated types, etc.
}

#[derive(Debug)]
pub struct Impl {
    pub trait_name: String,
    pub target_name: String,
    pub functions: Vec<Node<Function>>,
    // TODO: Constants, etc.
}

#[derive(Debug)]
pub struct FunctionPrototype {
    pub name: String,
    pub parameters: Node<FunctionParameterList>,
    pub return_type: Type,
}

#[derive(Debug)]
pub struct Struct {
    pub name: String,
    pub fields: Node<StructFieldList>,
    pub is_public: bool,
}

pub type StructFieldList = Vec<Node<StructField>>;

#[derive(Debug)]
pub struct StructField {
    pub name: String,
    pub ty: Type,
    pub is_public: bool,
}

pub type FunctionParameterList = Vec<Node<FunctionParameter>>;

#[derive(Debug)]
pub struct FunctionParameter {
    pub mutable: bool,
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Let {
        ident: String,
        mutable: bool,
        ty: Type,
        value: Option<Node<Expression>>,
    },

    Assign {
        lhs: Node<Expression>,
        operator: Node<AssignmentOp>,
        expression: Node<Expression>,
    },

    ImplicitReturn {
        expression: Node<Expression>,
        is_function_return: bool,
    },

    Return {
        expression: Node<Expression>,
    },

    Expression(Node<Expression>),
}

#[derive(Debug, Clone)]
pub enum Expression {
    BinaryExpression {
        lhs: Box<Node<Expression>>,
        op: Node<BinaryOp>,
        rhs: Box<Node<Expression>>,
        ty: Type,
    },

    UnaryExpression {
        op: Node<UnaryOp>,
        expr: Box<Node<Expression>>,
        ty: Type,
    },

    /// Constructor for a type with fields
    FieldConstructor {
        // Name of type
        ty: Type,
        // Map of (field_name -> value)
        fields: std::collections::HashMap<String, Node<Expression>>,
    },

    /// Accessing a field of a type
    FieldAccess {
        /// The `lhs` of `lhs.field`
        base_expr: Box<Node<Expression>>,
        /// The field identifier being used
        field: String,
        /// The type of this FieldAccess (the field's type)
        ty: Type,
    },

    // MethodCall {
    //     ty: Type,
    // }

    FunctionCall {
        /// Name of function being called
        name: String,
        /// Expressions passed as input to the function (in order)
        inputs: Vec<Node<Expression>>,
        /// Type returned by the function
        ty: Type,
    },

    Block(BlockExpression),

    Literal { 
        value: Literal,
        ty: Type,
    },

    Ident {
        name: String,
        ty: Type,
    },
}

impl Expression {
    /// Returns the type of the expression as known at that time.  
    /// Type may be unknown for non-validated expressions
    pub fn get_type(&self) -> &Type {
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
pub struct BlockExpression {
    pub block: Node<Vec<Node<Statement>>>,
    pub ty: Type,
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
    Custom(Vec<Token>),
}

#[derive(Debug, Clone)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Custom(Vec<Token>),
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