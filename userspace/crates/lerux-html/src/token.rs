//! Subset tokenizer. Unknown tags are skipped; `<style>` is raw text.

use alloc::{string::String, vec::Vec};

use crate::TagName;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Token {
    Start {
        name: TagName,
        attrs: Vec<crate::Attr>,
        self_closing: bool,
    },
    End {
        name: TagName,
    },
    Text(String),
}

pub(crate) fn tokenize(input: &str) -> Vec<Token> {
    let mut p = Scanner {
        input: input.as_bytes(),
        pos: 0,
    };
    let mut out = Vec::new();
    while !p.eof() {
        if p.peek() == Some(b'<') {
            if p.starts_ignore_ascii(b"<!--") {
                p.skip_comment();
                continue;
            }
            if p.starts_ignore_ascii(b"<!doctype") {
                p.skip_until(b'>');
                continue;
            }
            if let Some(tok) = p.take_tag() {
                let raw = matches!(&tok, Token::Start { name, .. } if name.is_raw_text());
                out.push(tok);
                if raw {
                    p.take_raw_text(&mut out);
                }
            }
            continue;
        }
        if let Some(text) = p.take_text() {
            out.push(Token::Text(text));
        }
    }
    out
}

struct Scanner<'a> {
    input: &'a [u8],
    pos: usize,
}

impl Scanner<'_> {
    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn rest(&self) -> &[u8] {
        &self.input[self.pos..]
    }

    fn bump(&mut self) -> Option<u8> {
        let b = self.peek()?;
        self.pos += 1;
        Some(b)
    }

    fn starts_ignore_ascii(&self, prefix: &[u8]) -> bool {
        let rest = self.rest();
        rest.len() >= prefix.len() && rest[..prefix.len()].eq_ignore_ascii_case(prefix)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    fn skip_until(&mut self, needle: u8) {
        while let Some(b) = self.bump() {
            if b == needle {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        self.pos = self.pos.saturating_add(4);
        while !self.eof() {
            if self.starts_ignore_ascii(b"-->") {
                self.pos = self.pos.saturating_add(3);
                return;
            }
            self.pos += 1;
        }
    }

    fn take_text(&mut self) -> Option<String> {
        let start = self.pos;
        while !self.eof() && self.peek() != Some(b'<') {
            self.pos += 1;
        }
        if start == self.pos {
            return None;
        }
        Some(String::from_utf8_lossy(&self.input[start..self.pos]).into_owned())
    }

    fn take_tag(&mut self) -> Option<Token> {
        debug_assert_eq!(self.peek(), Some(b'<'));
        let saved = self.pos;
        self.pos += 1;
        let is_end = self.peek() == Some(b'/');
        if is_end {
            self.pos += 1;
        }
        let Some(name) = self.take_name() else {
            self.pos = saved;
            self.pos += 1;
            return Some(Token::Text(String::from("<")));
        };
        let Some(tag) = TagName::from_ascii(&name) else {
            self.skip_tag_tail();
            return None;
        };
        if is_end {
            self.skip_ws();
            if self.peek() == Some(b'>') {
                self.pos += 1;
            }
            return Some(Token::End { name: tag });
        }
        let attrs = self.take_attrs();
        self.skip_ws();
        let self_closing = if self.peek() == Some(b'/') {
            self.pos += 1;
            true
        } else {
            false
        };
        self.skip_ws();
        if self.peek() == Some(b'>') {
            self.pos += 1;
        }
        Some(Token::Start {
            name: tag,
            attrs,
            self_closing: self_closing || tag.is_void(),
        })
    }

    fn take_name(&mut self) -> Option<Vec<u8>> {
        let start = self.pos;
        let first = self.peek()?;
        if !first.is_ascii_alphabetic() {
            return None;
        }
        self.pos += 1;
        while matches!(self.peek(), Some(b) if b.is_ascii_alphanumeric() || b == b'-') {
            self.pos += 1;
        }
        Some(self.input[start..self.pos].to_vec())
    }

    fn take_attrs(&mut self) -> Vec<crate::Attr> {
        let mut attrs = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                None | Some(b'>') | Some(b'/') => break,
                _ => {}
            }
            let Some(name) = self.take_attr_name() else {
                break;
            };
            self.skip_ws();
            let value = if self.peek() == Some(b'=') {
                self.pos += 1;
                self.skip_ws();
                self.take_attr_value()
            } else {
                String::new()
            };
            attrs.push(crate::Attr { name, value });
        }
        attrs
    }

    fn take_attr_name(&mut self) -> Option<String> {
        let start = self.pos;
        while matches!(self.peek(), Some(b) if is_attr_name_char(b)) {
            self.pos += 1;
        }
        if start == self.pos {
            return None;
        }
        let mut s = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();
        s.make_ascii_lowercase();
        Some(s)
    }

    fn take_attr_value(&mut self) -> String {
        match self.peek() {
            Some(q @ (b'"' | b'\'')) => {
                self.pos += 1;
                let start = self.pos;
                while let Some(b) = self.peek() {
                    if b == q {
                        break;
                    }
                    self.pos += 1;
                }
                let v = String::from_utf8_lossy(&self.input[start..self.pos]).into_owned();
                if self.peek() == Some(q) {
                    self.pos += 1;
                }
                v
            }
            _ => {
                let start = self.pos;
                while matches!(self.peek(), Some(b) if !is_ws(b) && b != b'>' && b != b'/') {
                    self.pos += 1;
                }
                String::from_utf8_lossy(&self.input[start..self.pos]).into_owned()
            }
        }
    }

    fn skip_tag_tail(&mut self) {
        let mut quote: Option<u8> = None;
        while let Some(b) = self.bump() {
            match quote {
                Some(q) if b == q => quote = None,
                None if b == b'"' || b == b'\'' => quote = Some(b),
                None if b == b'>' => return,
                _ => {}
            }
        }
    }

    fn take_raw_text(&mut self, out: &mut Vec<Token>) {
        let end_tag = match out.last() {
            Some(Token::Start { name, .. }) if name.is_raw_text() => name.end_tag_bytes(),
            _ => return,
        };
        let start = self.pos;
        while !self.eof() {
            if self.starts_ignore_ascii(end_tag) {
                if start < self.pos {
                    out.push(Token::Text(
                        String::from_utf8_lossy(&self.input[start..self.pos]).into_owned(),
                    ));
                }
                if let Some(tok) = self.take_tag() {
                    out.push(tok);
                }
                return;
            }
            self.pos += 1;
        }
        if start < self.pos {
            out.push(Token::Text(
                String::from_utf8_lossy(&self.input[start..self.pos]).into_owned(),
            ));
        }
    }
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r')
}

fn is_attr_name_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':')
}
