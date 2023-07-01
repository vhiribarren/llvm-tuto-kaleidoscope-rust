/*
MIT License

Copyright (c) 2023 Vincent Hiribarren

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

pub const ANONYM_FUNCTION: &str = "__anon_expr";

#[derive(Debug, PartialEq)]
pub struct KaleoGrammar(pub Vec<TopAST>);

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
    IfExpr(IfExprAST),
    ForExpr(ForExprAST),
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

#[derive(Debug, PartialEq, Clone)]
pub struct PrototypeAST {
    pub name: String,
    pub args: Vec<String>,
    pub operator: Option<Operator>,
}

impl PrototypeAST {
    pub fn gen_binary_func_name(op: char) -> String {
        format!("binary{op}")
    }
    pub fn is_binary_op(&self) -> bool {
        matches!(&self.operator, Some(Operator::Binary { .. }))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Operator {
    Unary,
    Binary { op_name: char, precedence: isize },
}

#[derive(Debug, PartialEq)]
pub struct FunctionAST {
    pub proto: PrototypeAST,
    pub body: ExprAST,
}

impl FunctionAST {
    pub fn is_top_function(&self) -> bool {
        self.proto.name == ANONYM_FUNCTION
    }
}

#[derive(Debug, PartialEq)]
pub struct IfExprAST {
    pub condition: Box<ExprAST>,
    pub then_block: Box<ExprAST>,
    pub else_block: Box<ExprAST>,
}

#[derive(Debug, PartialEq)]
pub struct ForExprAST {
    pub var_name: String,
    pub var_start: Box<ExprAST>,
    pub var_end: Box<ExprAST>,
    pub step: Option<Box<ExprAST>>,
    pub body: Box<ExprAST>,
}
