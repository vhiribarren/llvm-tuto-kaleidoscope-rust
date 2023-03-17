#[derive(Debug, PartialEq)]
pub enum TopAST {
    Function(FunctionAST),
    Prototype(PrototypeAST),
}

#[derive(Debug, PartialEq)]
pub enum ExprAST {
    NumberExpr(NumberExprAST),
    VariableExpr(VariableExprAST),
    BinaryExpr(BinaryExprAST),
    CallExpr(CallExprAST),
}

#[derive(Debug, PartialEq)]
pub struct NumberExprAST {
    pub val: f64,
}

#[derive(Debug, PartialEq)]
pub struct VariableExprAST {
    pub name: String,
}

#[derive(Debug, PartialEq)]
pub struct BinaryExprAST {
    pub op: char,
    pub lhs: Box<ExprAST>,
    pub rhs: Box<ExprAST>,
}

#[derive(Debug, PartialEq)]
pub struct CallExprAST {
    pub callee: String,
    pub args: Vec<ExprAST>,
}

#[derive(Debug, PartialEq)]
pub struct PrototypeAST {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug, PartialEq)]
pub struct FunctionAST {
    pub proto: PrototypeAST,
    pub body: ExprAST,
}
