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

pub trait Visitor {
    type Result;
    fn visit_top(&mut self, s: &TopAST) -> Self::Result;
    fn visit_function(&mut self, s: &FunctionAST) -> Self::Result;
    fn visit_prototype(&mut self, e: &PrototypeAST) -> Self::Result;
    fn visit_expr(&mut self, e: &ExprAST) -> Self::Result;
    fn visit_number_expr(&mut self, e: &NumberExprAST) -> Self::Result;
    fn visit_variable_expr(&mut self, e: &VariableExprAST) -> Self::Result;
    fn visit_binary_expr(&mut self, e: &BinaryExprAST) -> Self::Result;
    fn visit_call_expr(&mut self, e: &CallExprAST) -> Self::Result;
}

pub struct PrintVisitor;

impl Visitor for PrintVisitor {
    type Result = ();
    fn visit_top(&mut self, top_elem: &TopAST) {
        println!("Visit top");
        match top_elem {
            TopAST::Function(func_elem) => self.visit_function(func_elem),
            TopAST::Prototype(proto_elem) => self.visit_prototype(proto_elem),
        }
    }
    fn visit_function(&mut self, func_elem: &FunctionAST) {
        println!("Visit function");
        self.visit_prototype(&func_elem.proto);
        self.visit_expr(&func_elem.body);
    }
    fn visit_prototype(&mut self, _e: &PrototypeAST) {
        println!("Visit prototype");
    }
    fn visit_expr(&mut self, expr_elem: &ExprAST) {
        println!("Visit expr");
        match expr_elem {
            ExprAST::NumberExpr(num_elem) => self.visit_number_expr(num_elem),
            ExprAST::VariableExpr(var_elem) => self.visit_variable_expr(var_elem),
            ExprAST::BinaryExpr(bin_elem) => self.visit_binary_expr(bin_elem),
            ExprAST::CallExpr(call_elem) => self.visit_call_expr(call_elem),
        }
    }
    fn visit_number_expr(&mut self, _num_elem: &NumberExprAST) {
        println!("Visit number expr");
    }
    fn visit_variable_expr(&mut self, _var_elem: &VariableExprAST) {
        println!("Visit variable expr");
    }
    fn visit_binary_expr(&mut self, bin_elem: &BinaryExprAST) {
        println!("Visit binary expr");
        self.visit_expr(&bin_elem.lhs);
        self.visit_expr(&bin_elem.rhs);
    }
    fn visit_call_expr(&mut self, call_elem: &CallExprAST) {
        println!("Visit call expr");
        for expr_elem in &call_elem.args {
            self.visit_expr(expr_elem);
        }
    }
}
