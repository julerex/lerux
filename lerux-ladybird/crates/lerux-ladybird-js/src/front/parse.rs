//! Recursive descent for statements, precedence climbing for expressions.

use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{
    ast::{BinaryOp, Expr, FunctionStmt, IfStmt, LetStmt, Stmt, UnaryOp},
    error::Error,
    lex::{Token, TokenKind},
};

pub(crate) fn script(tokens: &[Token]) -> Result<Vec<Stmt>, Error> {
    let mut parser = Parser { tokens, index: 0 };
    let mut body = Vec::new();
    while parser.kind() != TokenKind::End {
        body.push(parser.statement()?);
    }
    Ok(body)
}

struct Parser<'a> {
    tokens: &'a [Token],
    index: usize,
}

impl Parser<'_> {
    fn statement(&mut self) -> Result<Stmt, Error> {
        match self.kind() {
            TokenKind::Let => self.let_stmt(),
            TokenKind::Function => self.function_stmt(),
            TokenKind::If => self.if_stmt(),
            TokenKind::Return => self.return_stmt(),
            TokenKind::LBrace => self.block(),
            _ => self.expr_stmt(),
        }
    }

    fn let_stmt(&mut self) -> Result<Stmt, Error> {
        self.bump();
        let name = self.ident()?;
        self.expect(TokenKind::Assign)?;
        self.bump();
        let init = self.expression()?;
        self.end_stmt()?;
        Ok(Stmt::Let(Box::new(LetStmt { name, init })))
    }

    fn function_stmt(&mut self) -> Result<Stmt, Error> {
        self.bump();
        let name = self.ident()?;
        let params = self.params()?;
        let body = self.block_body()?;
        Ok(Stmt::Function(Box::new(FunctionStmt {
            name,
            params,
            body,
        })))
    }

    fn if_stmt(&mut self) -> Result<Stmt, Error> {
        self.bump();
        self.expect(TokenKind::LParen)?;
        self.bump();
        let test = self.expression()?;
        self.expect(TokenKind::RParen)?;
        self.bump();
        let consequent = self.arm()?;
        let alternate = self.else_arm()?;
        Ok(Stmt::If(Box::new(IfStmt {
            test,
            consequent,
            alternate,
        })))
    }

    fn arm(&mut self) -> Result<Stmt, Error> {
        let stmt = self.statement()?;
        if is_lexical(&stmt) {
            return Err(Error::syntax());
        }
        Ok(stmt)
    }

    fn else_arm(&mut self) -> Result<Option<Stmt>, Error> {
        if self.kind() != TokenKind::Else {
            return Ok(None);
        }
        self.bump();
        Ok(Some(self.arm()?))
    }

    fn return_stmt(&mut self) -> Result<Stmt, Error> {
        self.bump();
        if self.can_end() {
            self.end_stmt()?;
            return Ok(Stmt::Return(None));
        }
        let expr = self.expression()?;
        self.end_stmt()?;
        Ok(Stmt::Return(Some(expr)))
    }

    fn block(&mut self) -> Result<Stmt, Error> {
        Ok(Stmt::Block(self.block_body()?))
    }

    fn block_body(&mut self) -> Result<Vec<Stmt>, Error> {
        self.expect(TokenKind::LBrace)?;
        self.bump();
        let mut body = Vec::new();
        while self.kind() != TokenKind::RBrace {
            if self.kind() == TokenKind::End {
                return Err(Error::syntax());
            }
            body.push(self.statement()?);
        }
        self.bump();
        Ok(body)
    }

    fn expr_stmt(&mut self) -> Result<Stmt, Error> {
        let expr = self.expression()?;
        self.end_stmt()?;
        Ok(Stmt::Expr(expr))
    }

    fn params(&mut self) -> Result<Vec<String>, Error> {
        self.expect(TokenKind::LParen)?;
        self.bump();
        let mut names = Vec::new();
        if self.kind() != TokenKind::RParen {
            names = self.name_list()?;
        }
        self.expect(TokenKind::RParen)?;
        self.bump();
        Ok(names)
    }

    fn name_list(&mut self) -> Result<Vec<String>, Error> {
        let mut names = Vec::new();
        loop {
            names.push(self.ident()?);
            if self.kind() != TokenKind::Comma {
                return Ok(names);
            }
            self.bump();
        }
    }

    fn expression(&mut self) -> Result<Expr, Error> {
        self.climb(PREC_NONE)
    }

    fn climb(&mut self, min: u8) -> Result<Expr, Error> {
        let mut left = self.unary()?;
        while let Some((op, prec)) = infix(self.kind()) {
            if prec < min {
                break;
            }
            self.bump();
            let right = self.climb(prec + 1)?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, Error> {
        let op = match self.kind() {
            TokenKind::Minus => UnaryOp::Neg,
            TokenKind::Plus => UnaryOp::Pos,
            _ => return self.postfix(),
        };
        self.bump();
        Ok(Expr::Unary(op, Box::new(self.unary()?)))
    }

    fn postfix(&mut self) -> Result<Expr, Error> {
        let mut expr = self.primary()?;
        while self.kind() == TokenKind::LParen {
            expr = Expr::Call(Box::new(expr), self.arguments()?);
        }
        Ok(expr)
    }

    fn arguments(&mut self) -> Result<Vec<Expr>, Error> {
        self.bump();
        let mut args = Vec::new();
        if self.kind() != TokenKind::RParen {
            args = self.expr_list()?;
        }
        self.expect(TokenKind::RParen)?;
        self.bump();
        Ok(args)
    }

    fn expr_list(&mut self) -> Result<Vec<Expr>, Error> {
        let mut args = Vec::new();
        loop {
            args.push(self.expression()?);
            if self.kind() != TokenKind::Comma {
                return Ok(args);
            }
            self.bump();
        }
    }

    fn primary(&mut self) -> Result<Expr, Error> {
        let expr = match self.kind() {
            TokenKind::Number => Expr::Number(self.number()),
            TokenKind::String => Expr::String(self.text()),
            TokenKind::True => Expr::Boolean(true),
            TokenKind::False => Expr::Boolean(false),
            TokenKind::Undefined => Expr::Undefined,
            TokenKind::Ident => Expr::Ident(self.text()),
            TokenKind::LParen => return self.group(),
            _ => return Err(Error::syntax()),
        };
        self.bump();
        Ok(expr)
    }

    fn group(&mut self) -> Result<Expr, Error> {
        self.bump();
        let expr = self.expression()?;
        self.expect(TokenKind::RParen)?;
        self.bump();
        Ok(expr)
    }

    fn ident(&mut self) -> Result<String, Error> {
        if self.kind() != TokenKind::Ident {
            return Err(Error::syntax());
        }
        let text = self.text();
        self.bump();
        Ok(text)
    }

    fn end_stmt(&mut self) -> Result<(), Error> {
        if self.kind() == TokenKind::Semicolon {
            self.bump();
            return Ok(());
        }
        if self.can_end() {
            return Ok(());
        }
        Err(Error::syntax())
    }

    fn can_end(&self) -> bool {
        matches!(self.kind(), TokenKind::RBrace | TokenKind::End)
    }

    fn expect(&self, kind: TokenKind) -> Result<(), Error> {
        if self.kind() == kind {
            return Ok(());
        }
        Err(Error::syntax())
    }

    fn kind(&self) -> TokenKind {
        self.tokens
            .get(self.index)
            .map_or(TokenKind::End, |token| token.kind)
    }

    fn number(&self) -> f64 {
        self.tokens
            .get(self.index)
            .map_or(0.0, |token| token.number)
    }

    fn text(&self) -> String {
        self.tokens
            .get(self.index)
            .map(|token| token.text.clone())
            .unwrap_or_default()
    }

    fn bump(&mut self) {
        if self.index + 1 < self.tokens.len() {
            self.index += 1;
        }
    }
}

fn is_lexical(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Let(_) | Stmt::Function(_))
}

const PREC_NONE: u8 = 0;
const PREC_ADD: u8 = 1;
const PREC_MUL: u8 = 2;

fn infix(kind: TokenKind) -> Option<(BinaryOp, u8)> {
    match kind {
        TokenKind::Plus => Some((BinaryOp::Add, PREC_ADD)),
        TokenKind::Minus => Some((BinaryOp::Sub, PREC_ADD)),
        TokenKind::Star => Some((BinaryOp::Mul, PREC_MUL)),
        TokenKind::Slash => Some((BinaryOp::Div, PREC_MUL)),
        TokenKind::Percent => Some((BinaryOp::Rem, PREC_MUL)),
        _ => None,
    }
}
