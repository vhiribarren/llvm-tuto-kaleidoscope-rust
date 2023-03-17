#[derive(Debug)]
pub enum TopAST {
    Function(FunctionAST),
    Prototype(PrototypeAST),
}

#[derive(Debug)]
pub enum ExprAST {
    NumberExpr(NumberExprAST),
    VariableExpr(VariableExprAST),
    BinaryExpr(BinaryExprAST),
    CallExpr(CallExprAST),
}

#[derive(Debug)]
pub struct NumberExprAST {
    pub val: f64,
}

#[derive(Debug)]
pub struct VariableExprAST {
    pub name: String,
}

#[derive(Debug)]
pub struct BinaryExprAST {
    pub op: char,
    pub lhs: Box<ExprAST>,
    pub rhs: Box<ExprAST>,
}

#[derive(Debug)]
pub struct CallExprAST {
    pub callee: String,
    pub args: Vec<ExprAST>,
}

#[derive(Debug)]
pub struct PrototypeAST {
    pub name: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct FunctionAST {
    pub proto: PrototypeAST,
    pub body: ExprAST,
}
