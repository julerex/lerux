//! Integer expressions shared by the fetch `calc` command and the maths session.
//!
//! A sum is products separated by `+` and `-`. A product is factors separated
//! by `*` and `/`. A factor is a number, a parenthesised sum, or a factor with
//! a leading `+` or `-`. Spaces are ignored. `+`, `-`, and `*` saturate.
//! Division truncates toward zero and fails when the divisor is zero. A
//! trailing character that is not a space fails.

#![cfg_attr(not(test), no_std)]

/// The input is empty, unbalanced, or otherwise not this grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error;

/// Evaluate one expression. An empty input fails.
pub fn eval(input: &[u8]) -> Result<i64, Error> {
    let mut parser = Parser { s: input, i: 0 };
    let value = parser.parse_expr()?;
    parser.skip();
    if parser.i != parser.s.len() {
        return Err(Error);
    }
    Ok(value)
}

/// Write the decimal form of `value` into `buf`.
///
/// Returns the number of bytes written. `buf` has to hold the sign and the digits.
pub fn write_i64(buf: &mut [u8], value: i64) -> usize {
    if value < 0 {
        buf[0] = b'-';
        return 1 + write_u64(&mut buf[1..], value.unsigned_abs());
    }
    write_u64(buf, u64::try_from(value).expect("non-negative"))
}

fn write_u64(buf: &mut [u8], mut value: u64) -> usize {
    let mut tmp = [0u8; 20];
    let mut index = tmp.len();
    loop {
        index -= 1;
        tmp[index] = b'0' + (value % 10) as u8;
        value /= 10;
        if value == 0 {
            break;
        }
    }
    let digits = &tmp[index..];
    buf[..digits.len()].copy_from_slice(digits);
    digits.len()
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn skip(&mut self) {
        while self.i < self.s.len() && self.s[self.i] == b' ' {
            self.i += 1;
        }
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.s.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let byte = self.peek()?;
        self.i += 1;
        Some(byte)
    }

    fn parse_expr(&mut self) -> Result<i64, Error> {
        let mut value = self.parse_term()?;
        loop {
            match self.peek() {
                Some(b'+') => {
                    self.bump();
                    value = value.saturating_add(self.parse_term()?);
                }
                Some(b'-') => {
                    self.bump();
                    value = value.saturating_sub(self.parse_term()?);
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_term(&mut self) -> Result<i64, Error> {
        let mut value = self.parse_factor()?;
        loop {
            match self.peek() {
                Some(b'*') => {
                    self.bump();
                    value = value.saturating_mul(self.parse_factor()?);
                }
                Some(b'/') => {
                    self.bump();
                    let right = self.parse_factor()?;
                    if right == 0 {
                        return Err(Error);
                    }
                    value /= right;
                }
                _ => break,
            }
        }
        Ok(value)
    }

    fn parse_factor(&mut self) -> Result<i64, Error> {
        match self.peek() {
            Some(b'(') => {
                self.bump();
                let value = self.parse_expr()?;
                if self.bump() != Some(b')') {
                    return Err(Error);
                }
                Ok(value)
            }
            Some(b'+') => {
                self.bump();
                self.parse_factor()
            }
            Some(b'-') => {
                self.bump();
                Ok(self.parse_factor()?.saturating_neg())
            }
            Some(b'0'..=b'9') => self.parse_number(),
            _ => Err(Error),
        }
    }

    fn parse_number(&mut self) -> Result<i64, Error> {
        let mut value: i64 = 0;
        let mut saw = false;
        while let Some(byte @ b'0'..=b'9') = self.s.get(self.i).copied() {
            saw = true;
            self.i += 1;
            value = value
                .saturating_mul(10)
                .saturating_add(i64::from(byte - b'0'));
        }
        if saw {
            Ok(value)
        } else {
            Err(Error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{eval, write_i64, Error};

    #[test]
    fn product_of_a_sum() {
        assert_eq!(eval(b"(1+2)*3"), Ok(9));
    }

    #[test]
    fn division() {
        assert_eq!(eval(b"6/2"), Ok(3));
    }

    #[test]
    fn division_by_zero_fails() {
        assert_eq!(eval(b"1/0"), Err(Error));
    }

    #[test]
    fn trailing_junk_fails() {
        assert_eq!(eval(b"1 2"), Err(Error));
        assert_eq!(eval(b""), Err(Error));
    }

    #[test]
    fn decimal_form() {
        let mut buf = [0u8; 8];
        assert_eq!(write_i64(&mut buf, 9), 1);
        assert_eq!(&buf[..1], b"9");
        assert_eq!(write_i64(&mut buf, -12), 3);
        assert_eq!(&buf[..3], b"-12");
    }
}
