use super::lex::{SpannedToken, Token};

#[derive(Debug)]
pub struct Node<NodeType> {
    pub item: NodeType,
    pub span: crate::Span,
    // TODO: This flag might not be needed (just knowing at least one error exists is enough)
    pub is_error_recovery_node: bool,
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


pub type AST<'input> = Vec<TopLevel<'input>>;

#[derive(Debug)]
pub enum TopLevel<'input> {
    Function(Node<Function<'input>>),
    Struct(Node<Struct<'input>>),
    ConstDeclaration,
    UseStatement,
}

#[derive(Debug)]
pub struct Function<'input> {
    pub name: &'input str,
    pub parameters: Node<FunctionParameterList<'input>>,
    pub return_type: &'input str,
    pub statements: Node<StatementBlock<'input>>,
}

#[derive(Debug)]
pub struct Struct<'input> {
    pub name: &'input str,
    pub fields: Node<StructFieldList<'input>>,
}

pub type StructFieldList<'input> = Vec<Node<StructField<'input>>>;

#[derive(Debug)]
pub struct StructField<'input> {
    pub field_name: &'input str,
    pub field_type: &'input str,
}

pub type FunctionParameterList<'input> = Vec<Node<FunctionParameter<'input>>>;

#[derive(Debug)]
pub struct FunctionParameter<'input> {
    pub mutable: bool,
    pub field_name: &'input str,
    pub field_type: &'input str,
}


pub type StatementBlock<'input> = Vec<Node<Statement<'input>>>;

#[derive(Debug)]
pub enum Statement<'input> {
    Let {
        ident: &'input str,
        mutable: bool,
        type_: Option<&'input str>,
        value: Option<Node<Expression<'input>>>,
    },

    Assign {
        variable: &'input str,
        operator: Node<AssignmentOp>,
        expression: Node<Expression<'input>>,
    },

    Return {
        expression: Node<Expression<'input>>,
    },

    Expression(Node<Expression<'input>>),
}

#[derive(Debug)]
pub enum Expression<'input> {
    BinaryExpression {
        lhs: Box<Node<Expression<'input>>>,
        op: Node<BinaryOp>,
        rhs: Box<Node<Expression<'input>>>,
    },

    UnaryExpression {
        op: Node<UnaryOp>,
        expr: Box<Node<Expression<'input>>>,
    },

    Parenthesized(Box<Node<Expression<'input>>>),

    Literal(Literal),
    Ident(&'input str),
}

#[derive(Debug)]
pub enum Literal {
    Number(usize),
    UnitType, // `()` type
}

#[derive(Debug)]
pub enum UnaryOp {
    Negate,
}

#[derive(Debug)]
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

#[derive(Debug)]
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