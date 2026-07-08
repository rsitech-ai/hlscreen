use hls_core::{HlsError, HlsResult};

use crate::dsl::ast::{CmpOp, Expr, Field, SortDirection, SortField, ValueExpr};

pub fn parse_filter(input: &str) -> HlsResult<Expr> {
    let tokens = tokenize(input)?;
    let mut parser = Parser::new(tokens);
    let expr = parser.parse_or()?;
    parser.expect_end()?;
    Ok(expr)
}

pub fn parse_sort(input: &str) -> HlsResult<SortField> {
    let Some((value, direction)) = input.rsplit_once(':') else {
        return Err(HlsError::Parse(
            "sort must use field:asc or field:desc".to_owned(),
        ));
    };
    let direction = match direction.trim() {
        "asc" => SortDirection::Asc,
        "desc" => SortDirection::Desc,
        other => {
            return Err(HlsError::Parse(format!(
                "sort direction must be asc or desc, got '{other}'"
            )));
        }
    };

    let tokens = tokenize(value.trim())?;
    let mut parser = Parser::new(tokens);
    let value = parser.parse_value()?;
    parser.expect_end()?;
    match value {
        ValueExpr::Field(_) | ValueExpr::Abs(_) => Ok(SortField { value, direction }),
        _ => Err(HlsError::Parse(
            "sort value must be a field or abs(field)".to_owned(),
        )),
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Ident(String),
    Number(f64),
    String(String),
    Bool(bool),
    And,
    Or,
    LParen,
    RParen,
    Gt,
    Gte,
    Lt,
    Lte,
    Eq,
    Ne,
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, index: 0 }
    }

    fn parse_or(&mut self) -> HlsResult<Expr> {
        let mut expr = self.parse_and()?;
        while self.consume(&Token::Or) {
            expr = Expr::Or(Box::new(expr), Box::new(self.parse_and()?));
        }
        Ok(expr)
    }

    fn parse_and(&mut self) -> HlsResult<Expr> {
        let mut expr = self.parse_comparison()?;
        while self.consume(&Token::And) {
            expr = Expr::And(Box::new(expr), Box::new(self.parse_comparison()?));
        }
        Ok(expr)
    }

    fn parse_comparison(&mut self) -> HlsResult<Expr> {
        if self.consume(&Token::LParen) {
            let expr = self.parse_or()?;
            self.expect(Token::RParen, "expected ')'")?;
            return Ok(expr);
        }

        let left = self.parse_value()?;
        let op = self.parse_cmp_op()?;
        let right = self.parse_value()?;
        Ok(Expr::Compare { left, op, right })
    }

    fn parse_value(&mut self) -> HlsResult<ValueExpr> {
        match self.next() {
            Some(Token::Ident(ident)) if ident == "abs" => {
                self.expect(Token::LParen, "expected '(' after abs")?;
                let field = self.parse_field_ident()?;
                self.expect(Token::RParen, "expected ')' after abs field")?;
                Ok(ValueExpr::Abs(field))
            }
            Some(Token::Ident(ident)) if self.peek() == Some(&Token::LParen) => {
                Err(HlsError::Config(format!("unknown function '{ident}'")))
            }
            Some(Token::Ident(ident)) => Ok(ValueExpr::Field(Field::parse(&ident)?)),
            Some(Token::Number(value)) => Ok(ValueExpr::Number(value)),
            Some(Token::String(value)) => Ok(ValueExpr::String(value)),
            Some(Token::Bool(value)) => Ok(ValueExpr::Bool(value)),
            Some(other) => Err(HlsError::Parse(format!("expected value, got {other:?}"))),
            None => Err(HlsError::Parse("expected value".to_owned())),
        }
    }

    fn parse_field_ident(&mut self) -> HlsResult<Field> {
        match self.next() {
            Some(Token::Ident(ident)) => Field::parse(&ident),
            Some(other) => Err(HlsError::Parse(format!("expected field, got {other:?}"))),
            None => Err(HlsError::Parse("expected field".to_owned())),
        }
    }

    fn parse_cmp_op(&mut self) -> HlsResult<CmpOp> {
        match self.next() {
            Some(Token::Gt) => Ok(CmpOp::Gt),
            Some(Token::Gte) => Ok(CmpOp::Gte),
            Some(Token::Lt) => Ok(CmpOp::Lt),
            Some(Token::Lte) => Ok(CmpOp::Lte),
            Some(Token::Eq) => Ok(CmpOp::Eq),
            Some(Token::Ne) => Ok(CmpOp::Ne),
            Some(other) => Err(HlsError::Parse(format!(
                "expected comparison operator, got {other:?}"
            ))),
            None => Err(HlsError::Parse("expected comparison operator".to_owned())),
        }
    }

    fn expect(&mut self, token: Token, message: &str) -> HlsResult<()> {
        if self.consume(&token) {
            return Ok(());
        }
        Err(HlsError::Parse(message.to_owned()))
    }

    fn expect_end(&self) -> HlsResult<()> {
        if self.index == self.tokens.len() {
            return Ok(());
        }
        Err(HlsError::Parse(format!(
            "unexpected token {:?}",
            self.tokens[self.index]
        )))
    }

    fn consume(&mut self, token: &Token) -> bool {
        if self.tokens.get(self.index) == Some(token) {
            self.index += 1;
            return true;
        }
        false
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        if token.is_some() {
            self.index += 1;
        }
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }
}

fn tokenize(input: &str) -> HlsResult<Vec<Token>> {
    let chars: Vec<char> = input.chars().collect();
    let mut index = 0;
    let mut tokens = Vec::new();

    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }

        match ch {
            '(' => {
                tokens.push(Token::LParen);
                index += 1;
            }
            ')' => {
                tokens.push(Token::RParen);
                index += 1;
            }
            '"' => {
                let (value, next_index) = read_string(&chars, index + 1)?;
                tokens.push(Token::String(value));
                index = next_index;
            }
            '>' if chars.get(index + 1) == Some(&'=') => {
                tokens.push(Token::Gte);
                index += 2;
            }
            '>' => {
                tokens.push(Token::Gt);
                index += 1;
            }
            '<' if chars.get(index + 1) == Some(&'=') => {
                tokens.push(Token::Lte);
                index += 2;
            }
            '<' => {
                tokens.push(Token::Lt);
                index += 1;
            }
            '=' if chars.get(index + 1) == Some(&'=') => {
                tokens.push(Token::Eq);
                index += 2;
            }
            '!' if chars.get(index + 1) == Some(&'=') => {
                tokens.push(Token::Ne);
                index += 2;
            }
            '-' | '0'..='9' => {
                let (value, next_index) = read_number(&chars, index)?;
                tokens.push(Token::Number(value));
                index = next_index;
            }
            '_' | 'a'..='z' | 'A'..='Z' => {
                let (ident, next_index) = read_ident(&chars, index);
                tokens.push(match ident.as_str() {
                    "and" => Token::And,
                    "or" => Token::Or,
                    "true" => Token::Bool(true),
                    "false" => Token::Bool(false),
                    _ => Token::Ident(ident),
                });
                index = next_index;
            }
            other => {
                return Err(HlsError::Parse(format!(
                    "unexpected character '{other}' in filter"
                )));
            }
        }
    }

    Ok(tokens)
}

fn read_string(chars: &[char], mut index: usize) -> HlsResult<(String, usize)> {
    let mut value = String::new();
    while index < chars.len() {
        match chars[index] {
            '"' => return Ok((value, index + 1)),
            '\\' => {
                index += 1;
                let Some(escaped) = chars.get(index) else {
                    return Err(HlsError::Parse("unterminated string escape".to_owned()));
                };
                value.push(*escaped);
                index += 1;
            }
            ch => {
                value.push(ch);
                index += 1;
            }
        }
    }
    Err(HlsError::Parse("unterminated string literal".to_owned()))
}

fn read_number(chars: &[char], start: usize) -> HlsResult<(f64, usize)> {
    let mut index = start;
    while matches!(chars.get(index), Some('-' | '.' | '0'..='9')) {
        index += 1;
    }
    let raw: String = chars[start..index].iter().collect();
    let value = raw
        .parse::<f64>()
        .map_err(|err| HlsError::Parse(format!("invalid number '{raw}': {err}")))?;
    Ok((value, index))
}

fn read_ident(chars: &[char], start: usize) -> (String, usize) {
    let mut index = start;
    while matches!(
        chars.get(index),
        Some('_' | 'a'..='z' | 'A'..='Z' | '0'..='9')
    ) {
        index += 1;
    }
    (chars[start..index].iter().collect(), index)
}
