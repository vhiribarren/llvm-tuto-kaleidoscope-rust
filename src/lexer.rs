use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, PartialEq)]
pub enum Token {
    Def,
    Extern,
    Identifier(String),
    Number(f64),
    Op(char),
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
                None => return None,
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
                None => return None,
                Some(_) => {
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
    use crate::lexer::Token::{Def, Extern, Identifier, Number, Op};

    #[test]
    fn scan_strings() {
        let input = "  hello   1.42   +-   def  extern  ";
        let mut lexer = Lexer::new(input.chars());
        assert_eq!(lexer.next().unwrap(), Identifier("hello".to_string()));
        assert_eq!(lexer.next().unwrap(), Number(1.42));
        assert_eq!(lexer.next().unwrap(), Op('+'));
        assert_eq!(lexer.next().unwrap(), Op('-'));
        assert_eq!(lexer.next().unwrap(), Def);
        assert_eq!(lexer.next().unwrap(), Extern);
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
