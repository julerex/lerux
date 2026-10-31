use alloc::{string::String, vec::Vec};

use super::{
    declaration, Combinator, CompoundSelector, Declaration, ParseError, Selector, SimpleSelector,
    StyleRule, StyleSheet,
};

pub(super) fn sheet(input: &str) -> Result<StyleSheet, ParseError> {
    let mut parser = Parser { input, pos: 0 };
    let mut rules = Vec::new();
    loop {
        parser.skip_trivia()?;
        if parser.peek().is_none() {
            break;
        }
        if parser.peek() == Some('@') {
            return Err(ParseError::new());
        }
        rules.push(parser.parse_rule()?);
    }
    Ok(StyleSheet { rules })
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn parse_rule(&mut self) -> Result<StyleRule, ParseError> {
        let selectors = self.parse_selector_list()?;
        self.expect('{')?;
        let declarations = self.parse_declarations()?;
        self.expect('}')?;
        Ok(StyleRule {
            selectors,
            declarations,
        })
    }

    fn parse_selector_list(&mut self) -> Result<Vec<Selector>, ParseError> {
        let mut selectors = Vec::new();
        loop {
            self.skip_trivia()?;
            selectors.push(self.parse_selector()?);
            self.skip_trivia()?;
            match self.peek() {
                Some(',') => {
                    self.bump();
                }
                Some('{') => break,
                _ => return Err(ParseError::new()),
            }
        }
        Ok(selectors)
    }

    fn parse_selector(&mut self) -> Result<Selector, ParseError> {
        let mut compounds = Vec::new();
        compounds.push(self.parse_compound(Combinator::None)?);
        loop {
            let saw_whitespace = self.skip_trivia()?;
            match self.peek() {
                Some('{') | Some(',') | None => break,
                Some('>') => {
                    self.bump();
                    self.skip_trivia()?;
                    compounds.push(self.parse_compound(Combinator::Child)?);
                }
                Some(_) if saw_whitespace => {
                    compounds.push(self.parse_compound(Combinator::Descendant)?);
                }
                _ => return Err(ParseError::new()),
            }
        }
        Ok(Selector { compounds })
    }

    fn parse_compound(&mut self, combinator: Combinator) -> Result<CompoundSelector, ParseError> {
        let mut simples = Vec::new();
        if self.starts_ident() {
            let mut name = self.parse_ident()?;
            name.make_ascii_lowercase();
            simples.push(SimpleSelector::Type(name.into_boxed_str()));
        }
        loop {
            self.skip_comments()?;
            match self.peek() {
                Some('.') => {
                    self.bump();
                    self.skip_comments()?;
                    simples.push(SimpleSelector::Class(self.parse_ident()?.into_boxed_str()));
                }
                Some('#') => {
                    self.bump();
                    self.skip_comments()?;
                    simples.push(SimpleSelector::Id(self.parse_ident()?.into_boxed_str()));
                }
                _ => break,
            }
        }
        if simples.is_empty() {
            return Err(ParseError::new());
        }
        Ok(CompoundSelector {
            combinator,
            simples,
        })
    }

    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ParseError> {
        let mut declarations = Vec::new();
        loop {
            self.skip_trivia()?;
            match self.peek() {
                Some('}') => break,
                Some(';') => {
                    self.bump();
                }
                None => return Err(ParseError::new()),
                _ => {
                    let name = self.parse_ident()?;
                    self.skip_trivia()?;
                    self.expect(':')?;
                    let (value, has_var) = self.parse_value()?;
                    if self.peek() == Some(';') {
                        self.bump();
                    }
                    declarations.push(declaration(name, value, has_var));
                }
            }
        }
        Ok(declarations)
    }

    fn parse_value(&mut self) -> Result<(String, bool), ParseError> {
        let mut text = String::new();
        let mut has_var = false;
        loop {
            if self.consume_comment()? {
                continue;
            }
            match self.peek() {
                None => return Err(ParseError::new()),
                Some(';' | '}') => break,
                Some('"' | '\'') => text.push_str(&self.parse_string()?),
                Some(ch) if is_ident_start(ch) => {
                    let ident = self.parse_ident()?;
                    let saved = self.pos;
                    self.skip_trivia()?;
                    if ident.eq_ignore_ascii_case("var") && self.peek() == Some('(') {
                        has_var = true;
                    }
                    self.pos = saved;
                    text.push_str(&ident);
                }
                Some(ch) => {
                    self.bump();
                    text.push(ch);
                }
            }
        }
        Ok((String::from(trim_css(&text)), has_var))
    }

    fn parse_string(&mut self) -> Result<String, ParseError> {
        let Some(quote) = self.bump() else {
            return Err(ParseError::new());
        };
        let mut text = String::new();
        text.push(quote);
        loop {
            match self.bump() {
                None => return Err(ParseError::new()),
                Some('\\') => {
                    text.push('\\');
                    let Some(escaped) = self.bump() else {
                        return Err(ParseError::new());
                    };
                    text.push(escaped);
                }
                Some(ch) if ch == quote => {
                    text.push(ch);
                    return Ok(text);
                }
                Some(ch) => text.push(ch),
            }
        }
    }

    fn parse_ident(&mut self) -> Result<String, ParseError> {
        let start = self.pos;
        if self.rest().starts_with("--") {
            self.pos += 2;
            if !self.peek().is_some_and(is_nmchar) {
                return Err(ParseError::new());
            }
            while self.peek().is_some_and(is_nmchar) {
                self.bump();
            }
            return Ok(String::from(&self.input[start..self.pos]));
        }
        if self.peek() == Some('-') {
            self.bump();
        }
        if !self.peek().is_some_and(is_nmstart) {
            return Err(ParseError::new());
        }
        self.bump();
        while self.peek().is_some_and(is_nmchar) {
            self.bump();
        }
        Ok(String::from(&self.input[start..self.pos]))
    }

    fn starts_ident(&self) -> bool {
        let mut chars = self.rest().chars();
        match chars.next() {
            Some(ch) if is_nmstart(ch) => true,
            Some('-') => match chars.next() {
                Some('-') => chars.next().is_some_and(is_nmchar),
                Some(ch) => is_nmstart(ch),
                None => false,
            },
            _ => false,
        }
    }

    fn skip_comments(&mut self) -> Result<(), ParseError> {
        while self.consume_comment()? {}
        Ok(())
    }

    /// `true` when a whitespace character was skipped. Comments do not count.
    fn skip_trivia(&mut self) -> Result<bool, ParseError> {
        let mut saw_whitespace = false;
        loop {
            if self.consume_comment()? {
                continue;
            }
            if self.peek().is_some_and(is_css_whitespace) {
                self.bump();
                saw_whitespace = true;
                continue;
            }
            break;
        }
        Ok(saw_whitespace)
    }

    fn consume_comment(&mut self) -> Result<bool, ParseError> {
        if !self.rest().starts_with("/*") {
            return Ok(false);
        }
        self.pos += 2;
        let Some(end) = self.rest().find("*/") else {
            return Err(ParseError::new());
        };
        self.pos += end + 2;
        Ok(true)
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        if self.bump() == Some(expected) {
            Ok(())
        } else {
            Err(ParseError::new())
        }
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }
}

fn is_ident_start(ch: char) -> bool {
    is_nmstart(ch) || ch == '-'
}

fn is_nmstart(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_'
}

fn is_nmchar(ch: char) -> bool {
    is_nmstart(ch) || ch.is_ascii_digit() || ch == '-'
}

fn is_css_whitespace(ch: char) -> bool {
    matches!(ch, ' ' | '\t' | '\n' | '\r' | '\u{000c}')
}

fn trim_css(value: &str) -> &str {
    value.trim_matches(is_css_whitespace)
}
