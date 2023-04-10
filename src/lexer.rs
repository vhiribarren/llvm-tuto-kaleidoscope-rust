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

use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq)]
pub enum Token {
    Def,
    Extern,
    Identifier(String),
    Number(f64),
    Op(char),
    If,
    Then,
    Else,
    For,
    In,
    EoF,
}

pub struct Lexer<'a> {
    input_iter: Peekable<Chars<'a>>,
}

impl<'a> Lexer<'a> {
    pub fn new(iter: Chars<'a>) -> Lexer<'a> {
        Self {
            input_iter: iter.peekable(),
        }
    }

    fn consume_whitespaces(&mut self) {
        loop {
            match self.input_iter.peek() {
                Some(c) if c.is_whitespace() => {
                    self.input_iter.next();
                }
                Some(c) if c == &'#' => {
                    self.consume_until_eol();
                }
                Some(_) | None => {
                    return;
                }
            };
        }
    }

    fn consume_until_eol(&mut self) {
        loop {
            match self.input_iter.peek() {
                Some(c) if c == &'\n' => {
                    self.input_iter.next();
                    return;
                }
                Some(_) => {
                    self.input_iter.next();
                }
                None => {
                    return;
                }
            }
        }
    }

    fn consume_alphabetic(&mut self) -> Option<String> {
        let mut result = String::new();
        loop {
            match self.input_iter.peek() {
                None => break,
                Some(c) if c.is_alphanumeric() => {
                    result.push(self.input_iter.next().unwrap());
                    continue;
                }
                Some(_) => break,
            }
        }
        Some(result)
    }

    fn consume_numeric(&mut self) -> Option<f64> {
        let mut val = String::new();
        loop {
            match self.input_iter.peek() {
                Some(&v) if Self::is_numeric(v) => {
                    val.push(v);
                    self.input_iter.next().unwrap();
                }
                Some(_) | None => {
                    break;
                }
            }
        }
        val.parse().ok()
    }

    fn is_numeric(c: char) -> bool {
        matches!(c, '.' | '0'..='9')
    }
}

impl Iterator for Lexer<'_> {
    type Item = Token;

    fn next(&mut self) -> Option<Self::Item> {
        self.consume_whitespaces();
        let token = match self.input_iter.peek() {
            None => return None,
            Some(c) if c.is_numeric() => match self.consume_numeric() {
                None => panic!(),
                Some(v) => Token::Number(v),
            },
            Some(c) if c.is_alphabetic() => match self.consume_alphabetic() {
                None => panic!(),
                Some(val) if val == "def" => Token::Def,
                Some(val) if val == "extern" => Token::Extern,
                Some(val) if val == "if" => Token::If,
                Some(val) if val == "then" => Token::Then,
                Some(val) if val == "else" => Token::Else,
                Some(val) if val == "for" => Token::For,
                Some(val) if val == "in" => Token::In,
                Some(any) => Token::Identifier(any),
            },
            Some(&c) => {
                self.input_iter.next().unwrap();
                Token::Op(c)
            }
        };
        Some(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Token::*;

    #[test]
    fn scan_simple_def() {
        let input = "def";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Def);
    }

    #[test]
    fn scan_simple_extern() {
        let input = "extern";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Extern);
    }

    #[test]
    fn scan_simple_if() {
        let input = "if";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), If);
    }

    #[test]
    fn scan_simple_then() {
        let input = "then";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Then);
    }

    #[test]
    fn scan_simple_else() {
        let input = "else";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Else);
    }

    #[test]
    fn scan_simple_for() {
        let input = "for";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), For);
    }

    #[test]
    fn scan_simple_in() {
        let input = "in";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), In);
    }

    #[test]
    fn scan_simple_number() {
        let input = "42";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Number(42_f64));
    }

    #[test]
    fn scan_simple_identifier() {
        let input = "abcd";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Identifier(String::from("abcd")));
    }

    #[test]
    fn scan_simple_op() {
        let input = "(";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Op('('));
    }

    #[test]
    fn scan_strings() {
        let input = r#"  hello   1.42
           +-    for  def  extern   in    
            if  else then "#;
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Identifier("hello".to_string()));
        assert_eq!(lexer.next().unwrap(), Number(1.42));
        assert_eq!(lexer.next().unwrap(), Op('+'));
        assert_eq!(lexer.next().unwrap(), Op('-'));
        assert_eq!(lexer.next().unwrap(), For);
        assert_eq!(lexer.next().unwrap(), Def);
        assert_eq!(lexer.next().unwrap(), Extern);
        assert_eq!(lexer.next().unwrap(), In);
        assert_eq!(lexer.next().unwrap(), If);
        assert_eq!(lexer.next().unwrap(), Else);
        assert_eq!(lexer.next().unwrap(), Then);
        assert!(lexer.next().is_none());
    }
    #[test]
    fn scan_strings_with_comments() {
        let input = r#"  #comment 1
        hello  #comment 2
        # 1.42
        "#;
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Identifier("hello".to_string()));
        assert!(lexer.next().is_none());
    }
}
