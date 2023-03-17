#[derive(Debug)]
pub enum TopAST {
    FunctionAST(FunctionAST),
    PrototypeAST(PrototypeAST),
}

#[derive(Debug)]
pub enum ExprAST {
    NumberExprAST(NumberExprAST),
    VariableExprAST(VariableExprAST),
    BinaryExprAST(BinaryExprAST),
    CallExprAST(CallExprAST),
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
