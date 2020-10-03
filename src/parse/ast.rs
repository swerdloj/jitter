pub type AST<'input> = Vec<TopLevel<'input>>;

#[derive(Debug)]
pub enum TopLevel<'input> {
    Function(Function<'input>),
    ConstDeclaration,
    UseStatement,
}

#[derive(Debug)]
pub struct Function<'input> {
    pub name: &'input str,
    pub parameters: Vec<FunctionParameters<'input>>,
    pub return_type: Option<&'input str>,
    pub statements: Vec<Statement<'input>>,
}

#[derive(Debug)]
pub struct FunctionParameters<'input> {
    pub field_name: &'input str,
    pub field_type: &'input str,
}

#[derive(Debug)]
pub enum Statement<'input> {
    Let {
        ident: &'input str,
        mutable: bool,
        type_: Option<&'input str>,
        value: Option<Expression>,
    },

    Expression(Expression),
}

#[derive(Debug)]
pub enum Expression {
    BinaryExpression {
        lhs: Box<Expression>,
        op: BinaryOp,
        rhs: Box<Expression>,
    },

    UnaryExpression {
        op: UnaryOp,
        expr: Box<Expression>,
    },

    Literal(Literal),
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