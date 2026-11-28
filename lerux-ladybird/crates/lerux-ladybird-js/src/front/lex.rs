//! Lexer over Unicode scalar values.
//!
//! `char` is one scalar value. The lexer does not decode UTF-16 code units.

use alloc::{string::String, vec::Vec};
use core::str::FromStr;

use crate::error::Error;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TokenKind {
    Ident,
    Number,
    String,
    Let,
    Function,
    If,
    Else,
    Return,
    True,
    False,
    Undefined,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Comma,
    Semicolon,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    End,
}

#[derive(Clone, Debug)]
pub(crate) struct Token {
    pub kind: TokenKind,
    pub number: f64,
    pub text: String,
}

impl Token {
    fn punctuation(kind: TokenKind) -> Self {
        Self {
            kind,
            number: 0.0,
            text: String::new(),
        }
    }

    fn ident(text: String) -> Self {
        Self {
            kind: TokenKind::Ident,
            number: 0.0,
            text,
        }
    }

    fn number(number: f64) -> Self {
        Self {
            kind: TokenKind::Number,
            number,
            text: String::new(),
        }
    }

    fn string(text: String) -> Self {
        Self {
            kind: TokenKind::String,
            number: 0.0,
            text,
        }
    }

    fn end() -> Self {
        Self::punctuation(TokenKind::End)
    }
}

pub(crate) fn tokenize(source: &str) -> Result<Vec<Token>, Error> {
    let mut lexer = Lexer { source, index: 0 };
    let mut tokens = Vec::new();
    loop {
        let token = lexer.next_token()?;
        let done = token.kind == TokenKind::End;
        tokens.push(token);
        if done {
            return Ok(tokens);
        }
    }
}

struct Lexer<'a> {
    source: &'a str,
    index: usize,
}

impl Lexer<'_> {
    fn next_token(&mut self) -> Result<Token, Error> {
        self.skip_trivia()?;
        let Some(ch) = self.peek() else {
            return Ok(Token::end());
        };
        if let Some(token) = self.punctuation(ch) {
            return Ok(token);
        }
        if ch.is_ascii_digit() || self.leading_dot(ch) {
            return self.number();
        }
        if ch == '"' || ch == '\'' {
            return self.string(ch);
        }
        if is_ident_start(ch) {
            return self.ident();
        }
        Err(Error::syntax())
    }

    fn punctuation(&mut self, ch: char) -> Option<Token> {
        let kind = match ch {
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            ',' => TokenKind::Comma,
            ';' => TokenKind::Semicolon,
            '=' => TokenKind::Assign,
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            _ => return None,
        };
        self.bump();
        Some(Token::punctuation(kind))
    }

    fn leading_dot(&self, ch: char) -> bool {
        ch == '.' && self.peek_second().is_some_and(|next| next.is_ascii_digit())
    }

    fn number(&mut self) -> Result<Token, Error> {
        let start = self.index;
        if self.peek() != Some('.') {
            self.digits()?;
        }
        self.fraction()?;
        self.exponent()?;
        self.reject_ident_tail()?;
        let lexeme = self.slice(start)?;
        Ok(Token::number(parse_number(lexeme)?))
    }

    fn fraction(&mut self) -> Result<(), Error> {
        if self.peek() != Some('.') {
            return Ok(());
        }
        self.bump();
        if self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.digits()?;
        }
        Ok(())
    }

    fn exponent(&mut self) -> Result<(), Error> {
        if !matches!(self.peek(), Some('e' | 'E')) {
            return Ok(());
        }
        self.bump();
        if matches!(self.peek(), Some('+' | '-')) {
            self.bump();
        }
        self.digits()
    }

    fn digits(&mut self) -> Result<(), Error> {
        if !self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            return Err(Error::syntax());
        }
        while self.peek().is_some_and(|ch| ch.is_ascii_digit()) {
            self.bump();
        }
        Ok(())
    }

    fn reject_ident_tail(&self) -> Result<(), Error> {
        if self.peek().is_some_and(is_ident_continue) {
            return Err(Error::syntax());
        }
        Ok(())
    }

    fn string(&mut self, quote: char) -> Result<Token, Error> {
        self.bump();
        let mut text = String::new();
        loop {
            match self.bump() {
                Some(ch) if ch == quote => return Ok(Token::string(text)),
                Some('\\') => text.push(self.escape()?),
                Some('\n' | '\r') | None => return Err(Error::syntax()),
                Some(ch) => text.push(ch),
            }
        }
    }

    fn escape(&mut self) -> Result<char, Error> {
        match self.bump() {
            Some('n') => Ok('\n'),
            Some('t') => Ok('\t'),
            Some('r') => Ok('\r'),
            Some('\\') => Ok('\\'),
            Some('\'') => Ok('\''),
            Some('"') => Ok('"'),
            Some('u') => self.unicode_escape(),
            _ => Err(Error::syntax()),
        }
    }

    fn unicode_escape(&mut self) -> Result<char, Error> {
        let mut code = 0_u32;
        for _ in 0..4 {
            let Some(ch) = self.bump() else {
                return Err(Error::syntax());
            };
            let Some(digit) = ch.to_digit(16) else {
                return Err(Error::syntax());
            };
            code = code * 16 + digit;
        }
        char::from_u32(code).ok_or(Error::syntax())
    }

    fn ident(&mut self) -> Result<Token, Error> {
        let start = self.index;
        self.bump();
        while self.peek().is_some_and(is_ident_continue) {
            self.bump();
        }
        let text = self.slice(start)?;
        if let Some(kind) = keyword(text) {
            return Ok(Token::punctuation(kind));
        }
        Ok(Token::ident(String::from(text)))
    }

    fn skip_trivia(&mut self) -> Result<(), Error> {
        loop {
            match self.peek() {
                Some(ch) if ch.is_whitespace() => {
                    self.bump();
                }
                Some('/') if self.peek_second() == Some('/') => self.skip_line(),
                Some('/') if self.peek_second() == Some('*') => self.skip_block()?,
                _ => return Ok(()),
            }
        }
    }

    fn skip_line(&mut self) {
        self.bump();
        self.bump();
        while let Some(ch) = self.bump() {
            if ch == '\n' {
                break;
            }
        }
    }

    fn skip_block(&mut self) -> Result<(), Error> {
        self.bump();
        self.bump();
        loop {
            match self.bump() {
                Some('*') if self.peek() == Some('/') => {
                    self.bump();
                    return Ok(());
                }
                Some(_) => {}
                None => return Err(Error::syntax()),
            }
        }
    }

    fn slice(&self, start: usize) -> Result<&str, Error> {
        self.source.get(start..self.index).ok_or(Error::syntax())
    }

    fn peek(&self) -> Option<char> {
        self.source[self.index..].chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        self.source[self.index..].chars().nth(1)
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.index += ch.len_utf8();
        Some(ch)
    }
}

fn keyword(text: &str) -> Option<TokenKind> {
    match text {
        "let" => Some(TokenKind::Let),
        "function" => Some(TokenKind::Function),
        "if" => Some(TokenKind::If),
        "else" => Some(TokenKind::Else),
        "return" => Some(TokenKind::Return),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "undefined" => Some(TokenKind::Undefined),
        _ => None,
    }
}

fn is_ident_start(ch: char) -> bool {
    ch == '$' || ch == '_' || ch.is_alphabetic()
}

fn is_ident_continue(ch: char) -> bool {
    ch == '$' || ch == '_' || ch.is_alphanumeric()
}

fn parse_number(lexeme: &str) -> Result<f64, Error> {
    let mut owned = String::new();
    let text = if lexeme.starts_with('.') {
        owned.push('0');
        owned.push_str(lexeme);
        owned.as_str()
    } else {
        lexeme
    };
    f64::from_str(text).map_err(|_| Error::syntax())
}
