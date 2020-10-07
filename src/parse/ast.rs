use crate::lex::{SpannedToken, Token};

// TODO: Use a struct to add this data. That way, all child nodes will still be
// forced to be of correct types (not just nodes).
// macro_rules! make_ast_node {
//     ( $($t:ident $(, $l:lifetime)?);+ $(;)?) => {
//         // Create variants for the AST variant
//         #[derive(Debug)]
//         pub enum NodeData<'input> {
//             $(
//                 $t($t$(<$l>)?),
//             )+
//         }

//         // Impl Into for the AST variant
//         $(
//             impl<'input> Into<NodeData<'input>> for $t$(<$l>)? {
//                 fn into(self) -> NodeData<'input> {
//                     NodeData::$t(self)
//                 }
//             }
//         )+
//     };
// }

// make_ast_node! {
//     TopLevel, 'input;
//     Function, 'input;
//     Struct, 'input;
//     StructField, 'input;
//     FunctionParameter, 'input;
//     Statement, 'input;
//     Expression, 'input;
//     Literal;
//     UnaryOp;
//     BinaryOp;
//     AssignmentOp;
// }

// #[derive(Debug)]
// pub struct Node<'input> {
//     data: NodeData<'input>,
//     span: crate::Span,
//     is_error_recovery_node: bool,
// }

#[derive(Debug)]
pub struct Node<NodeType> {
    item: NodeType,
    span: crate::Span,
    is_error_recovery_node: bool,
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


pub type AST<'input> = Vec<Node<TopLevel<'input>>>;

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
    pub parameters: Vec<Node<FunctionParameter<'input>>>,
    pub return_type: Option<&'input str>,
    pub statements: Vec<Node<Statement<'input>>>,
}

#[derive(Debug)]
pub struct Struct<'input> {
    pub name: &'input str,
    pub fields: Vec<Node<StructField<'input>>>,
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
        value: Option<Node<Expression<'input>>>,
    },

    Assign {
        variable: &'input str,
        operator: Node<AssignmentOp>,
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
    pub fn from_spanned_token(symbol_token: &SpannedToken) -> Node<Self> {
        let op = match symbol_token.token {
            Token::Plus => BinaryOp::Add,
            Token::Minus => BinaryOp::Subtract,
            Token::Asterisk => BinaryOp::Multiply,
            Token::Slash => BinaryOp::Divide,

            _ => panic!("Cannot create BinaryOp from {:?}", symbol_token),
        };

        Node::new(op, symbol_token.span)
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
    pub fn from_spanned_token(op_token: &SpannedToken) -> Node<Self> {
        let op = match op_token.token {
            Token::Equals => AssignmentOp::Assign,
            Token::Plus => AssignmentOp::AddAssign,
            Token::Minus => AssignmentOp::SubtractAssign,
            Token::Asterisk => AssignmentOp::MultiplyAssign,
            Token::Slash => AssignmentOp::DivideAssign,

            _ => panic!("Cannot create AssignmentOp from {:?}", op_token),
        };

        Node::new(op, op_token.span)
    }
}