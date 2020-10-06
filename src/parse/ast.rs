use crate::lex::Token;

macro_rules! make_ast_node {
    ( $($t:ident $(, $l:lifetime)?);+ $(;)?) => {
        // Create variants for the AST variant
        #[derive(Debug)]
        pub enum NodeData<'input> {
            $(
                $t($t$(<$l>)?),
            )+
        }

        // Impl Into for the AST variant
        $(
            impl<'input> Into<NodeData<'input>> for $t$(<$l>)? {
                fn into(self) -> NodeData<'input> {
                    NodeData::$t(self)
                }
            }
        )+
    };
}

make_ast_node! {
    TopLevel, 'input;
    Function, 'input;
    Struct, 'input;
    StructField, 'input;
    FunctionParameter, 'input;
    Statement, 'input;
    Expression, 'input;
    Literal;
    UnaryOp;
    BinaryOp;
    AssignmentOp;
}

// TODO: Get nodes from parser instead of AST variants
#[derive(Debug)]
pub struct Node<'input> {
    data: NodeData<'input>,
    span: crate::Span,
    is_error_recovery_node: bool,
}

impl<'input> Node<'input> {
    pub fn new(data: impl Into<NodeData<'input>>, span: crate::Span) -> Self {
        Self {
            data: data.into(),
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

// TEMP: See `Node`
pub type AST<'input> = Vec<TopLevel<'input>>;

#[derive(Debug)]
pub enum TopLevel<'input> {
    Function(Function<'input>),
    Struct(Struct<'input>),
    ConstDeclaration,
    UseStatement,
}

#[derive(Debug)]
pub struct Function<'input> {
    pub name: &'input str,
    pub parameters: Vec<FunctionParameter<'input>>,
    pub return_type: Option<&'input str>,
    pub statements: Vec<Statement<'input>>,
}

#[derive(Debug)]
pub struct Struct<'input> {
    pub name: &'input str,
    pub fields: Vec<StructField<'input>>,
}

#[derive(Debug)]
pub struct StructField<'input> {
    pub field_name: &'input str,
    pub field_type: &'input str,
}

#[derive(Debug)]
pub struct FunctionParameter<'input> {
    pub mutable: bool,
    pub field_name: &'input str,
    pub field_type: &'input str,
}

#[derive(Debug)]
pub enum Statement<'input> {
    Let {
        ident: &'input str,
        mutable: bool,
        type_: Option<&'input str>,
        value: Option<Expression<'input>>,
    },

    Assign {
        variable: &'input str,
        operator: AssignmentOp,
        expression: Expression<'input>,
    },

    Expression(Expression<'input>),
}

#[derive(Debug)]
pub enum Expression<'input> {
    BinaryExpression {
        lhs: Box<Expression<'input>>,
        op: BinaryOp,
        rhs: Box<Expression<'input>>,
    },

    UnaryExpression {
        op: UnaryOp,
        expr: Box<Expression<'input>>,
    },

    Parenthesized(Box<Expression<'input>>),

    Literal(Literal),
    Ident(&'input str),
}

#[derive(Debug)]
pub enum Literal {
    Number(usize),
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
    pub fn from_token(symbol_token: &Token) -> Self {
        match symbol_token {
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
    pub fn from_token(op_token: &Token) -> Self {
        match op_token {
            Token::Equals => AssignmentOp::Assign,
            Token::Plus => AssignmentOp::AddAssign,
            Token::Minus => AssignmentOp::SubtractAssign,
            Token::Asterisk => AssignmentOp::MultiplyAssign,
            Token::Slash => AssignmentOp::DivideAssign,

            _ => panic!("Cannot create AssignmentOp from {:?}", op_token),
        }
    }
}