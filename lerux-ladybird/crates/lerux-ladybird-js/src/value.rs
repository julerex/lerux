use alloc::string::String;
use core::fmt::{self, Debug, Formatter};

/// A value detached from the heap when [`crate::run`] returns.
///
/// [`Self::Object`] is an ordinary object. This milestone's objects are
/// functions. The heap, and any identity the object had, ends with the run.
#[derive(Clone)]
pub enum Value {
    /// The `undefined` literal, a missing argument, or a script with no value.
    Undefined,
    /// `true` or `false`.
    Boolean(bool),
    /// An IEEE-754 number.
    Number(f64),
    /// A string literal.
    String(String),
    /// An ordinary object, including a function.
    Object,
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) | (Self::Object, Self::Object) => true,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            // Bit equality keeps `-0` distinct and treats identical NaN payloads as equal.
            (Self::Number(left), Self::Number(right)) => left.to_bits() == right.to_bits(),
            (Self::String(left), Self::String(right)) => left == right,
            _ => false,
        }
    }
}

impl Debug for Value {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Undefined => formatter.write_str("undefined"),
            Self::Boolean(bit) => bit.fmt(formatter),
            Self::Number(number) => number.fmt(formatter),
            Self::String(text) => text.fmt(formatter),
            Self::Object => formatter.write_str("object"),
        }
    }
}
