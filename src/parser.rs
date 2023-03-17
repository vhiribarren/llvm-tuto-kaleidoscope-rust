use anyhow::{bail, ensure, Result};
use once_cell::sync::Lazy;

use crate::ast::{
    BinaryExprAST, CallExprAST, ExprAST, FunctionAST, NumberExprAST, PrototypeAST, TopAST,
    VariableExprAST,
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

    pub fn parse(lexer: Lexer<'a>) -> Result<Vec<TopAST>> {
        let parser = &mut Parser {
            lexer: lexer.peekable(),
        };
        parser.parse_top()
    }

    fn parse_top(&mut self) -> Result<Vec<TopAST>> {
        let mut result = vec![];
        loop {
            match self.peek_token() {
                Token::Def => result.push(TopAST::Function(self.parse_definition()?)),
                Token::Extern => result.push(TopAST::Prototype(self.parse_extern()?)),
                Token::Op(';') => {
                    self.consume_token();
                }
                Token::EoF => return Ok(result),
                _ => result.push(TopAST::Function(self.parse_top_level_expression()?)),
            };
        }
    }

    fn parse_primary(&mut self) -> Result<ExprAST> {
        match self.peek_token() {
            Token::Identifier(_) => self.parse_identifier_expr(),
            Token::Number(_) => self.parse_number_expr(),
            Token::Op('(') => self.parse_paren_expr(),
            _ => bail!("Unknown token when expecting an expression"),
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

    fn parse_identifier_expr(&mut self) -> Result<ExprAST> {
        let name = match self.consume_token() {
            Token::Identifier(id_name) => id_name,
            _ => bail!("Was waiting for a Token::Identifier"),
        };
        if !matches!(self.peek_token(), Token::Op('(')) {
            return Ok(ExprAST::VariableExpr(VariableExprAST { name }));
        }
        let mut args = vec![];
        if !matches!(self.consume_token(), Token::Op(')')) {
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
        let identifier_name = match self.consume_token() {
            Token::Identifier(identifier_name) => identifier_name,
            _ => bail!("Was waiting a Token::Identifier"),
        };
        ensure!(
            matches!(self.consume_token(), Token::Op('(')),
            "Was waiting for '('"
        );
        let mut arg_names = vec![];
        loop {
            match self.consume_token() {
                Token::Identifier(id) => arg_names.push(id),
                Token::Op(')') => {
                    return Ok(PrototypeAST {
                        name: identifier_name,
                        args: arg_names,
                    })
                }
                _ => bail!("Was expecting ')'"),
            }
        }
    }

    fn parse_definition(&mut self) -> Result<FunctionAST> {
        ensure!(
            matches!(self.consume_token(), Token::Def),
            "Was waiting for Token::Def"
        );
        let proto = self.parse_prototype()?;
        let expr = self.parse_expression()?;
        Ok(FunctionAST { proto, body: expr })
    }

    fn parse_extern(&mut self) -> Result<PrototypeAST> {
        ensure!(
            matches!(self.consume_token(), Token::Extern),
            "Was waiting for Token::Extern"
        );
        self.parse_prototype()
    }

    fn parse_top_level_expression(&mut self) -> Result<FunctionAST> {
        let expr = self.parse_expression()?;
        let anonymous_prototype = PrototypeAST {
            name: String::from(""),
            args: vec![],
        };
        Ok(FunctionAST {
            body: expr,
            proto: anonymous_prototype,
        })
    }
}

pub fn generate_ast(input: &str) -> Result<Vec<TopAST>> {
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
        let result = vec![TopAST::Prototype(PrototypeAST {
            name: String::from("sin"),
            args: vec![String::from("a")],
        })];
        assert_eq!(ast, result);
    }

    #[test]
    fn scan_input_2() {
        let input = r#"
        def foo(x y) x+foo(y, 4.0);
        "#;
        let ast = generate_ast(input).unwrap();
        let result = vec![TopAST::Function(FunctionAST {
            proto: PrototypeAST {
                name: "foo".to_string(),
                args: vec!["x".to_string(), "y".to_string()],
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
        })];
        assert_eq!(ast, result);
    }

    #[test]
    fn scan_input_3() {
        let input = r#"
        def foo(x y) x+y y;
        "#;
        let ast = generate_ast(input).unwrap();
        let result = vec![
            TopAST::Function(FunctionAST {
                proto: PrototypeAST {
                    name: "foo".to_string(),
                    args: vec!["x".to_string(), "y".to_string()],
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
                    name: "".to_string(),
                    args: vec![],
                },
                body: ExprAST::VariableExpr(VariableExprAST {
                    name: "y".to_string(),
                }),
            }),
        ];
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
