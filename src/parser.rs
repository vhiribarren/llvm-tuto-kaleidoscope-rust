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

use anyhow::{bail, ensure, Result};
use once_cell::sync::Lazy;

use crate::ast::{
    BinaryExprAST, CallExprAST, ExprAST, ForExprAST, FunctionAST, IfExprAST, KaleoGrammar,
    NumberExprAST, Operator, PrototypeAST, TopAST, VariableExprAST, ANONYM_FUNCTION,
};
use crate::lexer::{Lexer, Token};
use std::collections::HashMap;
use std::iter::Peekable;

pub struct Parser<'a> {
    lexer: Peekable<Lexer<'a>>,
}

impl<'a> Parser<'a> {
    fn get_token_precedence(op: char) -> isize {
        static BIN_OP_PRIORITY: Lazy<HashMap<char, isize>> = Lazy::new(|| {
            let mut m = HashMap::new();
            m.insert('<', 10);
            m.insert('+', 20);
            m.insert('-', 20);
            m.insert('*', 40);
            m
        });
        match BIN_OP_PRIORITY.get(&op) {
            Some(val) => *val,
            None => -1,
        }
    }

    pub fn parse(lexer: Lexer<'a>) -> Result<KaleoGrammar> {
        let parser = &mut Parser {
            lexer: lexer.peekable(),
        };
        parser.parse_top()
    }

    fn consume_and_ensure_token(&mut self, _token: Token) -> Result<()> {
        ensure!(
            matches!(self.consume_token(), _token),
            format!("Was waiting for '{_token:?}' token")
        );
        Ok(())
    }

    fn parse_top(&mut self) -> Result<KaleoGrammar> {
        let mut result = vec![];
        loop {
            match self.peek_token() {
                Token::Def => result.push(TopAST::Function(self.parse_definition()?)),
                Token::Extern => result.push(TopAST::Prototype(self.parse_extern()?)),
                Token::Op(';') => {
                    self.consume_token();
                }
                Token::EoF => return Ok(KaleoGrammar(result)),
                _ => result.push(TopAST::Function(self.parse_top_level_expression()?)),
            };
        }
    }

    fn parse_primary(&mut self) -> Result<ExprAST> {
        match self.peek_token() {
            Token::Identifier(_) => self.parse_identifier_expr(),
            Token::Number(_) => self.parse_number_expr(),
            Token::Op('(') => self.parse_paren_expr(),
            Token::If => self.parse_if_expr(),
            Token::For => self.parse_for_expr(),
            other => bail!("Unknown token {other:?} when expecting an expression"),
        }
    }

    fn consume_token(&mut self) -> Token {
        match self.lexer.next() {
            Some(token) => token,
            None => Token::EoF,
        }
    }

    fn peek_token(&mut self) -> &Token {
        match self.lexer.peek() {
            Some(token) => token,
            None => &Token::EoF,
        }
    }

    fn parse_expression(&mut self) -> Result<ExprAST> {
        let lhs = self.parse_primary()?;
        self.parse_bin_op_rhs(0, lhs)
    }

    fn parse_bin_op_rhs(&mut self, expr_precedence: isize, mut lhs: ExprAST) -> Result<ExprAST> {
        loop {
            let op = match self.peek_token() {
                &Token::Op(op) => op,
                _ => return Ok(lhs),
            };
            let tok_prec = Parser::get_token_precedence(op);
            if tok_prec < expr_precedence {
                return Ok(lhs);
            }
            self.consume_token();
            let mut rhs = self.parse_primary()?;
            if let Token::Op(next_op) = self.peek_token() {
                let next_prec = Parser::get_token_precedence(*next_op);
                if tok_prec < next_prec {
                    rhs = self.parse_bin_op_rhs(tok_prec + 1, rhs)?;
                }
            }
            lhs = ExprAST::BinaryExpr(BinaryExprAST {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            });
        }
    }

    fn parse_number_expr(&mut self) -> Result<ExprAST> {
        match self.consume_token() {
            Token::Number(val) => Ok(ExprAST::NumberExpr(NumberExprAST { val })),
            _ => bail!("Was waiting for a Token::Number"),
        }
    }

    fn parse_paren_expr(&mut self) -> Result<ExprAST> {
        self.consume_token();
        let expr = self.parse_expression();
        match self.consume_token() {
            Token::Op(')') => expr,
            _ => bail!("Was expecting a ')'"),
        }
    }

    fn parse_if_expr(&mut self) -> Result<ExprAST> {
        self.consume_and_ensure_token(Token::If)?;
        let condition = Box::new(self.parse_expression()?);
        self.consume_and_ensure_token(Token::Then)?;
        let then_block = Box::new(self.parse_expression()?);
        self.consume_and_ensure_token(Token::Else)?;
        let else_block = Box::new(self.parse_expression()?);
        Ok(ExprAST::IfExpr(IfExprAST {
            condition,
            then_block,
            else_block,
        }))
    }

    fn parse_for_expr(&mut self) -> Result<ExprAST> {
        self.consume_and_ensure_token(Token::For)?;
        let var_name = match self.consume_token() {
            Token::Identifier(var_name) => var_name,
            other => bail!("Was waiting for an identifier, but received: {other:?}"),
        };
        self.consume_and_ensure_token(Token::Op('='))?;
        let var_start = Box::new(self.parse_expression()?);
        self.consume_and_ensure_token(Token::Op(','))?;
        let var_end = Box::new(self.parse_expression()?);
        let step = match self.peek_token() {
            &Token::Op(',') => {
                self.consume_token();
                Some(Box::new(self.parse_expression()?))
            }
            _ => None,
        };
        self.consume_and_ensure_token(Token::In)?;
        let body = Box::new(self.parse_expression()?);
        Ok(ExprAST::ForExpr(ForExprAST {
            var_name,
            var_start,
            var_end,
            step,
            body,
        }))
    }

    fn parse_identifier_expr(&mut self) -> Result<ExprAST> {
        let name = match self.consume_token() {
            Token::Identifier(id_name) => id_name,
            _ => bail!("Was waiting for a Token::Identifier"),
        };
        if !matches!(self.peek_token(), Token::Op('(')) {
            return Ok(ExprAST::VariableExpr(VariableExprAST { name }));
        }
        self.consume_token();
        let mut args = vec![];
        if !matches!(self.peek_token(), Token::Op(')')) {
            loop {
                args.push(self.parse_expression()?);
                if matches!(self.peek_token(), Token::Op(')')) {
                    break;
                }
                if !matches!(self.peek_token(), Token::Op(',')) {
                    bail!("Expected ')' or ',' in argument list");
                }
                self.consume_token();
            }
        }
        self.consume_token();
        Ok(ExprAST::CallExpr(CallExprAST { callee: name, args }))
    }

    fn parse_prototype(&mut self) -> Result<PrototypeAST> {
        let mut operator = None;
        let name = match self.consume_token() {
            Token::Identifier(name) => name,
            Token::Binary => {
                let op_id = match self.consume_token() {
                    Token::Identifier(op) => op,
                    _ => bail!("Was expecting an Identifier"),
                };
                let mut precedence = 30;
                if let Token::Number(prec_candidate) = self.peek_token() {
                    if *prec_candidate < 1.0 || *prec_candidate > 100.0 {
                        bail!("Invalid precedence: must be 1..100");
                    }
                    precedence = *prec_candidate as usize;
                    self.consume_token();
                }
                operator = Some(Operator::Binary { precedence });
                format!("binary{op_id}")
            }
            _ => bail!("Was waiting a Token::Identifier"),
        };
        self.consume_and_ensure_token(Token::Op('('))?;
        let mut args = vec![];
        loop {
            match self.consume_token() {
                Token::Identifier(id) => args.push(id),
                Token::Op(')') => {
                    match operator {
                        Some(Operator::Binary { .. }) => ensure!(args.len() == 2),
                        Some(Operator::Unary) => ensure!(args.len() == 1),
                        None => (),
                    };
                    return Ok(PrototypeAST {
                        name,
                        args,
                        operator,
                    });
                }
                _ => bail!("Was expecting ')'"),
            }
        }
    }

    fn parse_definition(&mut self) -> Result<FunctionAST> {
        self.consume_and_ensure_token(Token::Def)?;
        let proto = self.parse_prototype()?;
        let expr = self.parse_expression()?;
        Ok(FunctionAST { proto, body: expr })
    }

    fn parse_extern(&mut self) -> Result<PrototypeAST> {
        self.consume_and_ensure_token(Token::Extern)?;
        self.parse_prototype()
    }

    fn parse_top_level_expression(&mut self) -> Result<FunctionAST> {
        let expr = self.parse_expression()?;
        let anonymous_prototype = PrototypeAST {
            name: String::from(ANONYM_FUNCTION),
            args: vec![],
            operator: None,
        };
        Ok(FunctionAST {
            body: expr,
            proto: anonymous_prototype,
        })
    }
}

pub fn generate_ast(input: &str) -> Result<KaleoGrammar> {
    let lexer = Lexer::new(input.chars());
    Parser::parse(lexer)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn scan_input_1() {
        let input = r#"
        extern sin(a);
        "#;
        let ast = generate_ast(input).unwrap();
        let result = KaleoGrammar(vec![TopAST::Prototype(PrototypeAST {
            name: String::from("sin"),
            args: vec![String::from("a")],
            operator: None,
        })]);
        assert_eq!(ast, result);
    }

    #[test]
    fn scan_input_2() {
        let input = r#"
        def foo(x y) x+foo(y, 4.0);
        "#;
        let ast = generate_ast(input).unwrap();
        let result = KaleoGrammar(vec![TopAST::Function(FunctionAST {
            proto: PrototypeAST {
                name: "foo".to_string(),
                args: vec!["x".to_string(), "y".to_string()],
                operator: None,
            },
            body: ExprAST::BinaryExpr(BinaryExprAST {
                op: '+',
                lhs: Box::new(ExprAST::VariableExpr(VariableExprAST {
                    name: "x".to_string(),
                })),
                rhs: Box::new(ExprAST::CallExpr(CallExprAST {
                    callee: "foo".to_string(),
                    args: vec![
                        ExprAST::VariableExpr(VariableExprAST {
                            name: "y".to_string(),
                        }),
                        ExprAST::NumberExpr(NumberExprAST { val: 4.0 }),
                    ],
                })),
            }),
        })]);
        assert_eq!(ast, result);
    }

    #[test]
    fn scan_input_3() {
        let input = r#"
        def foo(x y) x+y y;
        "#;
        let ast = generate_ast(input).unwrap();
        let result = KaleoGrammar(vec![
            TopAST::Function(FunctionAST {
                proto: PrototypeAST {
                    name: "foo".to_string(),
                    args: vec!["x".to_string(), "y".to_string()],
                    operator: None,
                },
                body: ExprAST::BinaryExpr(BinaryExprAST {
                    op: '+',
                    lhs: Box::new(ExprAST::VariableExpr(VariableExprAST {
                        name: "x".to_string(),
                    })),
                    rhs: Box::new(ExprAST::VariableExpr(VariableExprAST {
                        name: "y".to_string(),
                    })),
                }),
            }),
            TopAST::Function(FunctionAST {
                proto: PrototypeAST {
                    name: ANONYM_FUNCTION.to_string(),
                    args: vec![],
                    operator: None,
                },
                body: ExprAST::VariableExpr(VariableExprAST {
                    name: "y".to_string(),
                }),
            }),
        ]);
        assert_eq!(ast, result);
    }

    #[test]
    fn scan_bad_input_1() {
        let input = r#"
        def foo(x y) x+y );
        "#;
        let ast = generate_ast(input);
        assert!(ast.is_err());
    }
}
