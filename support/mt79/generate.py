#!/usr/bin/env python3
"""Reconstruct Linux v7.3.5 mt76 headers as standalone C and Rust.

The script is the only writer of the C headers and the Rust crate sources.
It is safe to rerun: a later run replaces generated files and drops stale ones.
"""

from __future__ import annotations

import argparse
import os
import re
import shutil
import subprocess
import sys
import tempfile
from dataclasses import dataclass, field
from pathlib import Path

KERNEL_DEFAULT = Path("/tmp/mt76-v7.3.5")
TAG = "v7.3.5"
NOTE = "Layout reconstructed from Linux v7.3.5 drivers/net/wireless/mediatek/mt76."
MASK64 = (1 << 64) - 1

REPO = Path(__file__).resolve().parents[2]
CRATE = REPO / "userspace/crates/lerux-mt79"
INCLUDE = CRATE / "include"
SRC = CRATE / "src"

TYPE_WORDS = {
    "const", "volatile", "restrict", "signed", "unsigned", "int", "char", "short",
    "long", "void", "bool", "float", "double", "u8", "u16", "u32", "u64", "s8",
    "s16", "s32", "s64", "__u8", "__u16", "__u32", "__u64", "__s8", "__s16",
    "__s32", "__s64", "__le16", "__le32", "__le64", "__be16", "__be32", "__be64",
    "__iomem", "__rcu", "__user", "__force", "__bitwise", "__restrict",
    "uint8_t", "uint16_t", "uint32_t", "uint64_t", "int8_t", "int16_t", "int32_t",
    "int64_t", "size_t",
}
QUALIFIERS = {
    "const", "volatile", "restrict", "__iomem", "__rcu", "__user", "__force",
    "__bitwise", "__restrict",
}
RUST_KEYWORDS = {
    "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
    "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
    "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
    "static", "struct", "super", "trait", "true", "type", "unsafe", "use",
    "where", "while", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen",
}

# size, align, C spelling, Rust spelling
SCALARS: dict[str, tuple[int, int, str, str]] = {}


def add_scalar(names: list[str], size: int, align: int, c_name: str, rust: str) -> None:
    for name in names:
        SCALARS[name] = (size, align, c_name, rust)


add_scalar(["u8", "__u8", "uint8_t", "char", "unsigned char"], 1, 1, "u8", "u8")
add_scalar(["s8", "__s8", "int8_t", "signed char"], 1, 1, "s8", "i8")
add_scalar(["bool"], 1, 1, "bool", "u8")
add_scalar(["u16", "__u16", "uint16_t", "unsigned short"], 2, 2, "u16", "u16")
add_scalar(["s16", "__s16", "int16_t", "short", "signed short"], 2, 2, "s16", "i16")
add_scalar(
    ["u32", "__u32", "uint32_t", "unsigned", "unsigned int"],
    4, 4, "u32", "u32",
)
add_scalar(["s32", "__s32", "int32_t", "int", "signed", "signed int"], 4, 4, "s32", "i32")
add_scalar(
    ["u64", "__u64", "uint64_t", "unsigned long", "unsigned long long"],
    8, 8, "u64", "u64",
)
add_scalar(
    ["s64", "__s64", "int64_t", "long", "long long", "signed long", "signed long long"],
    8, 8, "s64", "i64",
)
add_scalar(["__le16"], 2, 2, "__le16", "Le16")
add_scalar(["__le32"], 4, 4, "__le32", "Le32")
add_scalar(["__le64"], 8, 8, "__le64", "Le64")
add_scalar(["__be16"], 2, 2, "__be16", "Be16")
add_scalar(["__be32"], 4, 4, "__be32", "Be32")
add_scalar(["__be64"], 8, 8, "__be64", "Be64")


class ParseError(Exception):
    pass


class EvalError(Exception):
    pass


class LayoutError(Exception):
    pass


@dataclass
class Tok:
    kind: str
    value: str
    start: int
    end: int


@dataclass
class Skip:
    header: str
    name: str
    reason: str
    order: int


@dataclass
class IntLit:
    value: int


@dataclass
class Name:
    name: str


@dataclass
class Unary:
    op: str
    expr: object


@dataclass
class Binary:
    op: str
    left: object
    right: object


@dataclass
class Ternary:
    cond: object
    then: object
    else_: object


@dataclass
class Call:
    name: str
    args: list


@dataclass
class Cast:
    words: list[str]
    expr: object


@dataclass
class Macro:
    header: str
    name: str
    params: list[str] | None
    body: str
    expr: object | None
    order: int
    kind: str  # int, fn, str
    value: int | None = None
    sample: tuple[int, ...] | None = None
    text: str | None = None
    clang: int | None = None


@dataclass
class EnumConst:
    header: str
    name: str
    value: int
    order: int
    expr_src: str | None
    clang: int | None = None
    anchor: str | None = None


@dataclass
class LayField:
    name: str | None
    size: int
    align: int
    c_type: str
    rust_type: str
    bit_width: int | None = None
    arrays: tuple[int, ...] = ()
    flexible: bool = False
    elem_spell: str = ""
    promoted: bool = False
    nested: object | None = None
    offset: int = 0


@dataclass
class EmitField:
    name: str
    c_type: str
    rust_type: str
    offset: int
    size: int = 0
    align: int = 1
    arrays: tuple[int, ...] = ()


@dataclass
class Check:
    kind: str
    path: str
    offset: int | None = None
    size: int | None = None
    align: int | None = None
    clang: int | None = None


@dataclass
class Record:
    header: str
    kind: str
    name: str
    fields: list[EmitField]
    size: int
    align: int
    form: str  # c, align, packed, wrap
    align_n: int | None
    inner: str | None
    masks: list[tuple[str, int]]
    checks: list[Check]
    source: str
    order: int
    flex: tuple[str, str] | None
    includes: list[str] = field(default_factory=list)
    rust_mod: str = ""
    confirmed: bool = False
    clang_size: int | None = None
    clang_align: int | None = None
    children: list = field(default_factory=list)
    dep_keys: list = field(default_factory=list)
    defined_inside: set = field(default_factory=set)
    flex_rust: str = ""
    standalone: bool = True
    enum_deps: list = field(default_factory=list)


@dataclass
class Header:
    rel: str
    macros: list[Macro] = field(default_factory=list)
    enums: list[EnumConst] = field(default_factory=list)
    records: list[Record] = field(default_factory=list)
    skipped: list[Skip] = field(default_factory=list)
    includes: list[str] = field(default_factory=list)


def align_up(value: int, align: int) -> int:
    if align <= 1:
        return value
    return (value + align - 1) & ~(align - 1)


def u64(value: int) -> int:
    return value & MASK64


def rust_int(value: int) -> str:
    number = u64(value)
    if number < 1000:
        return str(number)
    digits = f"{number:x}"
    groups = []
    while digits:
        groups.append(digits[-4:])
        digits = digits[:-4]
    return "0x" + "_".join(reversed(groups))


def rust_ident(name: str) -> str:
    if name in RUST_KEYWORDS:
        return f"r#{name}"
    return name


def c_ident(name: str) -> str:
    return name


def strip_comments(text: str) -> str:
    out: list[str] = []
    index = 0
    length = len(text)
    while index < length:
        if text.startswith("//", index):
            end = text.find("\n", index)
            end = length if end < 0 else end
            out.append(" " * (end - index))
            index = end
            continue
        if text.startswith("/*", index):
            end = text.find("*/", index + 2)
            end = length if end < 0 else end + 2
            chunk = text[index:end]
            out.append("".join("\n" if char == "\n" else " " for char in chunk))
            index = end
            continue
        if text[index] in "\"'":
            quote = text[index]
            end = index + 1
            while end < length:
                if text[end] == "\\":
                    end += 2
                    continue
                if text[end] == quote:
                    end += 1
                    break
                end += 1
            out.append(text[index:end])
            index = end
            continue
        out.append(text[index])
        index += 1
    return "".join(out)


def join_lines(text: str) -> str:
    return re.sub(r"\\\r?\n", "", text)


def tokenize(source: str) -> list[Tok]:
    pattern = re.compile(
        r"\s+"
        r"|0[xX][0-9A-Fa-f]+(?:[uU][lL]{0,2}|[lL]{1,2}[uU]?)?"
        r"|\d+(?:[uU][lL]{0,2}|[lL]{1,2}[uU]?)?"
        r"|[A-Za-z_][A-Za-z0-9_]*"
        r"|\"(?:\\.|[^\"\\])*\""
        r"|'(?:\\.|[^'\\])*'"
        r"|<<=|>>=|\.\.\.|<<|>>|<=|>=|==|!=|&&|\|\||##|\+\+|--|->"
        r"|[{}\[\]().,;:?~!+\-*/%&|^<>=]"
    )
    tokens: list[Tok] = []
    index = 0
    length = len(source)
    while index < length:
        match = pattern.match(source, index)
        if not match:
            raise ParseError(f"bad token at {index}: {source[index:index + 16]!r}")
        raw = match.group(0)
        index = match.end()
        if raw[0].isspace():
            continue
        if raw[0].isdigit() or (raw.startswith("0") and len(raw) > 1 and raw[1] in "xX"):
            kind = "number"
        elif raw[0].isalpha() or raw[0] == "_":
            kind = "ident"
        elif raw[0] in "\"'":
            kind = "string" if raw[0] == '"' else "char"
        else:
            kind = "punct"
        tokens.append(Tok(kind, raw, match.start(), match.end()))
    return tokens


def parse_number(text: str) -> int:
    body = re.sub(r"(?i)(?:ull|llu|ul|lu|ll|u|l)$", "", text)
    base = 16 if body.lower().startswith("0x") else 10
    return int(body, base)


class Cur:
    def __init__(self, tokens: list[Tok], source: str) -> None:
        self.toks = tokens
        self.src = source
        self.i = 0

    def peek(self, offset: int = 0) -> Tok | None:
        index = self.i + offset
        if index >= len(self.toks):
            return None
        return self.toks[index]

    def bump(self) -> Tok:
        token = self.peek()
        if token is None:
            raise ParseError("unexpected end")
        self.i += 1
        return token

    def at(self, kind: str, value: str | None = None) -> bool:
        token = self.peek()
        if token is None or token.kind != kind:
            return False
        return value is None or token.value == value

    def eat(self, kind: str, value: str | None = None) -> Tok | None:
        if self.at(kind, value):
            return self.bump()
        return None

    def expect(self, kind: str, value: str | None = None) -> Tok:
        token = self.eat(kind, value)
        if token is None:
            raise ParseError(f"expected {value or kind}, got {self.peek()}")
        return token

    def word(self, value: str) -> bool:
        return self.at("ident", value)

    def definition_ahead(self) -> bool:
        index = self.i + 1
        if index < len(self.toks) and self.toks[index].kind == "ident":
            index += 1
        paren = 0
        while index < len(self.toks):
            token = self.toks[index]
            if token.value == "(":
                paren += 1
            elif token.value == ")":
                paren -= 1
            elif paren == 0 and token.value == "{":
                return True
            elif paren == 0 and token.value == ";":
                return False
            index += 1
        return False

    def resync(self) -> None:
        paren = 0
        brace = 0
        while self.peek():
            token = self.bump()
            if token.value == "(":
                paren += 1
            elif token.value == ")":
                paren = max(0, paren - 1)
            elif token.value == "{":
                brace += 1
            elif token.value == "}":
                if brace == 0 and paren == 0:
                    return
                brace = max(0, brace - 1)
            elif token.value == ";" and paren == 0 and brace == 0:
                return

    def parse_expr(self) -> object:
        return self._ternary()

    def _ternary(self) -> object:
        cond = self._or()
        if not self.eat("punct", "?"):
            return cond
        then = self._ternary()
        self.expect("punct", ":")
        else_ = self._ternary()
        return Ternary(cond, then, else_)

    def _bin(self, ops: list[str], lower) -> object:
        node = lower()
        while self.peek() and self.peek().value in ops:
            op = self.bump().value
            node = Binary(op, node, lower())
        return node

    def _or(self) -> object:
        return self._bin(["||"], self._and)

    def _and(self) -> object:
        return self._bin(["&&"], self._bor)

    def _bor(self) -> object:
        return self._bin(["|"], self._xor)

    def _xor(self) -> object:
        return self._bin(["^"], self._band)

    def _band(self) -> object:
        return self._bin(["&"], self._eq)

    def _eq(self) -> object:
        return self._bin(["==", "!="], self._rel)

    def _rel(self) -> object:
        return self._bin(["<", ">", "<=", ">="], self._shift)

    def _shift(self) -> object:
        return self._bin(["<<", ">>"], self._add)

    def _add(self) -> object:
        return self._bin(["+", "-"], self._mul)

    def _mul(self) -> object:
        return self._bin(["*", "/", "%"], self._unary)

    def _unary(self) -> object:
        cast = self._cast_words()
        if cast is not None:
            return Cast(cast, self._unary())
        token = self.peek()
        if token and token.value in {"+", "-", "~", "!"}:
            op = self.bump().value
            return Unary(op, self._unary())
        return self._primary()

    def _cast_words(self) -> list[str] | None:
        if not self.at("punct", "("):
            return None
        depth = 0
        index = self.i
        inner: list[Tok] = []
        close = None
        while index < len(self.toks):
            token = self.toks[index]
            if token.value == "(":
                depth += 1
            elif token.value == ")":
                depth -= 1
                if depth == 0:
                    close = index
                    break
            elif depth == 1:
                inner.append(token)
            index += 1
        if close is None or not inner:
            return None
        if any(token.kind != "ident" for token in inner):
            return None
        words = [token.value for token in inner]
        if not _type_words(words):
            return None
        after = self.toks[close + 1] if close + 1 < len(self.toks) else None
        if after is None or not _expr_start(after):
            return None
        self.i = close + 1
        return words

    def _primary(self) -> object:
        token = self.peek()
        if token is None:
            raise ParseError("expression ended")
        if token.kind == "number":
            self.bump()
            return IntLit(parse_number(token.value))
        if token.kind == "ident":
            name = self.bump().value
            if self.eat("punct", "("):
                args = []
                if not self.at("punct", ")"):
                    while True:
                        args.append(self.parse_expr())
                        if not self.eat("punct", ","):
                            break
                self.expect("punct", ")")
                return Call(name, args)
            return Name(name)
        if self.eat("punct", "("):
            node = self.parse_expr()
            self.expect("punct", ")")
            return node
        raise ParseError(f"bad expression token {token.value}")

    def parse_attrs(self) -> tuple[bool, object | None]:
        packed = False
        align = None
        while True:
            if self.word("__packed"):
                self.bump()
                packed = True
                continue
            if self.word("__aligned"):
                self.bump()
                self.expect("punct", "(")
                align = self.parse_expr()
                self.expect("punct", ")")
                continue
            if self.word("__attribute__"):
                packed, align = self._gnu_attr(packed, align)
                continue
            break
        return packed, align

    def _gnu_attr(self, packed: bool, align: object | None) -> tuple[bool, object | None]:
        self.bump()
        self.expect("punct", "(")
        self.expect("punct", "(")
        while not self.at("punct", ")"):
            if self.word("packed"):
                self.bump()
                packed = True
            elif self.word("aligned"):
                self.bump()
                self.expect("punct", "(")
                align = self.parse_expr()
                self.expect("punct", ")")
            else:
                self.bump()
            self.eat("punct", ",")
        self.expect("punct", ")")
        self.expect("punct", ")")
        return packed, align


def _expr_start(token: Tok) -> bool:
    if token.kind in {"ident", "number"}:
        return True
    return token.value in {"(", "+", "-", "~", "!"}


def _type_words(words: list[str]) -> bool:
    index = 0
    saw = False
    while index < len(words):
        word = words[index]
        if word in {"struct", "union", "enum"}:
            return False
        if word not in TYPE_WORDS:
            return False
        saw = True
        index += 1
    return saw


def bit(value: int) -> int:
    if value < 0 or value >= 64:
        raise EvalError("bit range")
    return 1 << value


def genmask(high: int, low: int) -> int:
    if not 0 <= low <= high <= 63:
        raise EvalError("genmask range")
    return u64((MASK64 >> (63 - high)) & (MASK64 << low))


@dataclass
class Env:
    values: dict[str, int]
    functions: dict[str, tuple[list[str], object]]
    fallback: dict[str, int] = field(default_factory=dict)


def eval_expr(node: object, env: Env, local: dict[str, int] | None = None) -> int:
    local = local or {}
    if isinstance(node, IntLit):
        return u64(node.value)
    if isinstance(node, Name):
        if node.name in local:
            return u64(local[node.name])
        if node.name in env.values:
            return u64(env.values[node.name])
        if node.name in env.fallback:
            return u64(env.fallback[node.name])
        raise EvalError(f"unknown {node.name}")
    if isinstance(node, Unary):
        value = eval_expr(node.expr, env, local)
        if node.op == "+":
            return u64(value)
        if node.op == "-":
            return u64(-value)
        if node.op == "~":
            return u64(~value)
        if node.op == "!":
            return 0 if value else 1
        raise EvalError(node.op)
    if isinstance(node, Binary):
        if node.op == "&&":
            return 1 if eval_expr(node.left, env, local) and eval_expr(node.right, env, local) else 0
        if node.op == "||":
            return 1 if eval_expr(node.left, env, local) or eval_expr(node.right, env, local) else 0
        left = eval_expr(node.left, env, local)
        right = eval_expr(node.right, env, local)
        return u64(_binop(node.op, left, right))
    if isinstance(node, Ternary):
        cond = eval_expr(node.cond, env, local)
        branch = node.then if cond else node.else_
        return u64(eval_expr(branch, env, local))
    if isinstance(node, Call):
        args = [eval_expr(arg, env, local) for arg in node.args]
        return u64(_call(node.name, args, env))
    if isinstance(node, Cast):
        value = eval_expr(node.expr, env, local)
        return u64(_cast(node.words, value))
    raise EvalError("node")


def _binop(op: str, left: int, right: int) -> int:
    if op == "+":
        return left + right
    if op == "-":
        return left - right
    if op == "*":
        return left * right
    if op == "/":
        if right == 0:
            raise EvalError("div0")
        return _trunc_div(left, right)
    if op == "%":
        if right == 0:
            raise EvalError("mod0")
        quot = _trunc_div(left, right)
        return left - quot * right
    if op in {"<<", ">>"}:
        if right < 0 or right >= 64:
            raise EvalError("shift")
        return left << right if op == "<<" else left >> right
    if op == "&":
        return left & right
    if op == "|":
        return left | right
    if op == "^":
        return left ^ right
    if op == "==":
        return 1 if left == right else 0
    if op == "!=":
        return 1 if left != right else 0
    if op == "<":
        return 1 if left < right else 0
    if op == ">":
        return 1 if left > right else 0
    if op == "<=":
        return 1 if left <= right else 0
    if op == ">=":
        return 1 if left >= right else 0
    raise EvalError(op)


def _trunc_div(left: int, right: int) -> int:
    quot = abs(left) // abs(right)
    if (left < 0) ^ (right < 0):
        return -quot
    return quot


def _call(name: str, args: list[int], env: Env) -> int:
    if name == "BIT" and len(args) == 1:
        return bit(args[0])
    if name == "GENMASK" and len(args) == 2:
        return genmask(args[0], args[1])
    if name not in env.functions:
        raise EvalError(f"call {name}")
    params, body = env.functions[name]
    if len(params) != len(args):
        raise EvalError(f"arity {name}")
    return eval_expr(body, env, dict(zip(params, args, strict=True)))


def _cast(words: list[str], value: int) -> int:
    filtered = [word for word in words if word not in QUALIFIERS]
    key = " ".join(filtered)
    signed = {
        "s8": 8, "s16": 16, "s32": 32, "s64": 64, "int": 32, "signed": 32,
        "signed int": 32, "short": 16, "signed short": 16, "long": 64,
        "long long": 64, "signed long": 64, "signed long long": 64,
    }
    unsigned = {
        "u8": 8, "u16": 16, "u32": 32, "u64": 64, "char": 8, "bool": 8,
        "unsigned": 32, "unsigned int": 32, "unsigned short": 16,
        "unsigned long": 64, "unsigned long long": 64, "__le16": 16,
        "__le32": 32, "__le64": 64, "__be16": 16, "__be32": 32, "__be64": 64,
        "__u8": 8, "__u16": 16, "__u32": 32, "__u64": 64,
    }
    if key in signed:
        bits = signed[key]
        value &= (1 << bits) - 1
        if value & (1 << (bits - 1)):
            value |= ~((1 << bits) - 1)
        return value
    if key in unsigned:
        bits = unsigned[key]
        return value & ((1 << bits) - 1)
    raise EvalError(f"cast {key}")


CALL_PATHS: dict[str, str] = {}
PARAM_ALIASES: dict[str, str] = {}


def rust_expr(node: object) -> str:
    if isinstance(node, IntLit):
        return f"{rust_int(node.value)}u64"
    if isinstance(node, Name):
        return PARAM_ALIASES.get(node.name, rust_ident(node.name))
    if isinstance(node, Unary):
        inner = rust_expr(node.expr)
        if node.op == "!":
            return f"(if ({inner}) == 0 {{ 1u64 }} else {{ 0u64 }})"
        if node.op == "~":
            return f"(!({inner}))"
        if node.op == "-":
            return f"({inner}).wrapping_neg()"
        return f"({inner})"
    if isinstance(node, Binary):
        left = rust_expr(node.left)
        right = rust_expr(node.right)
        return _rust_bin(node.op, left, right)
    if isinstance(node, Ternary):
        cond = rust_expr(node.cond)
        then = rust_expr(node.then)
        else_ = rust_expr(node.else_)
        return f"(if ({cond}) != 0 {{ {then} }} else {{ {else_} }})"
    if isinstance(node, Call):
        args = ", ".join(rust_expr(arg) for arg in node.args)
        if node.name == "BIT":
            return f"bit({args})"
        if node.name == "GENMASK":
            return f"genmask({args})"
        target = CALL_PATHS.get(node.name, rust_ident(node.name))
        return f"{target}({args})"
    if isinstance(node, Cast):
        return rust_expr(node.expr)
    raise EvalError("rust expr")


def _rust_bin(op: str, left: str, right: str) -> str:
    table = {
        "+": f"({left}).wrapping_add({right})",
        "-": f"({left}).wrapping_sub({right})",
        "*": f"({left}).wrapping_mul({right})",
        "/": f"div({left}, {right})",
        "%": f"rem({left}, {right})",
        "<<": f"shl({left}, {right})",
        ">>": f"shr({left}, {right})",
        "&": f"(({left}) & ({right}))",
        "|": f"(({left}) | ({right}))",
        "^": f"(({left}) ^ ({right}))",
        "==": f"(if ({left}) == ({right}) {{ 1u64 }} else {{ 0u64 }})",
        "!=": f"(if ({left}) != ({right}) {{ 1u64 }} else {{ 0u64 }})",
        "<": f"(if ({left}) < ({right}) {{ 1u64 }} else {{ 0u64 }})",
        ">": f"(if ({left}) > ({right}) {{ 1u64 }} else {{ 0u64 }})",
        "<=": f"(if ({left}) <= ({right}) {{ 1u64 }} else {{ 0u64 }})",
        ">=": f"(if ({left}) >= ({right}) {{ 1u64 }} else {{ 0u64 }})",
        "&&": f"(if ({left}) != 0 && ({right}) != 0 {{ 1u64 }} else {{ 0u64 }})",
        "||": f"(if ({left}) != 0 || ({right}) != 0 {{ 1u64 }} else {{ 0u64 }})",
    }
    if op not in table:
        raise EvalError(op)
    return table[op]


def c_expr(node: object) -> str:
    if isinstance(node, IntLit):
        return f"{u64(node.value)}ull"
    if isinstance(node, Name):
        return node.name
    if isinstance(node, Unary):
        return f"({node.op}{c_expr(node.expr)})"
    if isinstance(node, Binary):
        return f"({c_expr(node.left)} {node.op} {c_expr(node.right)})"
    if isinstance(node, Ternary):
        cond = c_expr(node.cond)
        then = c_expr(node.then)
        else_ = c_expr(node.else_)
        return f"(({cond}) ? ({then}) : ({else_}))"
    if isinstance(node, Call):
        args = ", ".join(c_expr(arg) for arg in node.args)
        return f"{node.name}({args})"
    if isinstance(node, Cast):
        return c_expr(node.expr)
    raise EvalError("c expr")


def macro_rejected(body: str) -> str | None:
    if not body:
        return "empty macro"
    if "##" in body:
        return "token paste"
    if "({" in body:
        return "statement expression"
    if re.search(r"\b(do|for|while|if|switch|return|goto)\b", body):
        return "statement macro"
    for token in ("sizeof", "typeof", "offsetof", "FIELD_", "->", "{", "}", "static", "inline"):
        if token in body:
            return "unparsed expression"
    return None


def normalize_record(source: str) -> str:
    text = re.sub(
        r"\b__aligned\s*\(\s*([^)]+?)\s*\)",
        r"__attribute__((aligned(\1)))",
        source,
    )
    return re.sub(r"\b__packed\b", "__attribute__((packed))", text)


def stub() -> str:
    return """
typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned int uint32_t;
typedef unsigned long long uint64_t;
typedef signed char int8_t;
typedef short int16_t;
typedef int int32_t;
typedef long long int64_t;
typedef unsigned char u8;
typedef unsigned short u16;
typedef unsigned int u32;
typedef unsigned long long u64;
typedef signed char s8;
typedef short s16;
typedef int s32;
typedef long long s64;
typedef unsigned char __u8;
typedef unsigned short __u16;
typedef unsigned int __u32;
typedef unsigned long long __u64;
typedef unsigned short __le16;
typedef unsigned int __le32;
typedef unsigned long long __le64;
typedef unsigned short __be16;
typedef unsigned int __be32;
typedef unsigned long long __be64;
typedef unsigned char bool;
#define BIT(n) (1ULL << (n))
#define GENMASK(high, low) (((~0ULL) >> (63ULL - (high))) & ((~0ULL) << (low)))
"""


def clang_run(source: str) -> tuple[bool, str]:
    with tempfile.TemporaryDirectory() as directory:
        path = Path(directory) / "probe.c"
        binary = Path(directory) / "probe"
        path.write_text(source)
        compiled = subprocess.run(
            ["clang", "-std=gnu11", str(path), "-o", str(binary)],
            capture_output=True,
            text=True,
            check=False,
        )
        if compiled.returncode != 0:
            return False, compiled.stderr
        ran = subprocess.run([str(binary)], capture_output=True, text=True, check=False)
        if ran.returncode != 0:
            return False, ran.stderr
        return True, ran.stdout


def layout_struct(fields: list[LayField], packed: bool, align_attr: int | None):
    bit_pos = 0
    max_align = 1
    placed: list[LayField] = []
    runs: list[dict] = []
    current: dict | None = None

    def finish() -> None:
        nonlocal current
        if current is not None:
            runs.append(current)
            current = None

    flex = None
    for item in fields:
        if item.flexible:
            finish()
            byte = align_up((bit_pos + 7) // 8, 1 if packed else item.align)
            flex = (item, byte)
            bit_pos = byte * 8
            if not packed:
                max_align = max(max_align, item.align)
            break
        if item.bit_width is not None:
            width = item.bit_width
            boundary = 1 if packed else max(item.align, 1) * 8
            if width < 0 or (boundary > 1 and width > boundary):
                raise LayoutError("bitfield width")
            if width == 0:
                bit_pos = align_up(bit_pos, boundary)
                if not packed:
                    max_align = max(max_align, item.align)
                continue
            if not packed and boundary > 1:
                end = bit_pos + width
                if (bit_pos // boundary) != ((end - 1) // boundary):
                    bit_pos = align_up(bit_pos, boundary)
            if not packed:
                max_align = max(max_align, item.align)
            if current is None:
                current = {"start": bit_pos, "end": bit_pos, "bits": []}
            current["bits"].append((item, bit_pos))
            bit_pos += width
            current["end"] = bit_pos
            continue
        finish()
        byte = align_up((bit_pos + 7) // 8, 1 if packed else item.align)
        item.offset = byte
        placed.append(item)
        bit_pos = (byte + item.size) * 8
        if not packed:
            max_align = max(max_align, item.align)
    finish()
    end_byte = (bit_pos + 7) // 8
    if align_attr:
        struct_align = align_attr
    elif packed:
        struct_align = 1
    else:
        struct_align = max(max_align, 1)
    if flex is not None:
        size = flex[1]
        if not packed:
            size = align_up(size, struct_align) if False else size
        size = align_up(size, struct_align) if size % struct_align else size
    else:
        size = align_up(end_byte, struct_align)
    return placed, runs, size, struct_align, flex


def lower_fields(placed: list[LayField], runs: list[dict]) -> tuple[list[EmitField], list[tuple[str, int, int, int]]]:
    items: list[tuple[int, EmitField]] = []
    masks: list[tuple[str, int, int, int]] = []
    storage_index = 0
    for run in runs:
        start = run["start"] // 8
        end = (run["end"] + 7) // 8
        length = max(end - start, 1)
        name = f"bitfield_{storage_index}"
        storage_index += 1
        arrays = () if length == 1 else (length,)
        items.append((
            start,
            EmitField(name, "u8", "u8", start, length, 1, arrays),
        ))
        base = start * 8
        for item, bit_at in run["bits"]:
            if not item.name or item.bit_width is None:
                continue
            shift = bit_at - base
            mask = ((1 << item.bit_width) - 1) << shift
            masks.append((item.name, u64(mask), shift, item.bit_width))
    for item in placed:
        arrays = item.arrays
        items.append((
            item.offset,
            EmitField(
                item.name or "field", item.c_type, item.rust_type, item.offset,
                item.size, item.align, arrays,
            ),
        ))
    items.sort(key=lambda pair: (pair[0], pair[1].name))
    lowered: list[EmitField] = []
    cursor = 0
    pad_index = 0
    for offset, emit in items:
        if offset < cursor:
            raise LayoutError("overlap")
        gap = offset - cursor
        if gap:
            pad = f"pad_{pad_index}"
            pad_index += 1
            arrays = () if gap == 1 else (gap,)
            lowered.append(EmitField(pad, "u8", "u8", cursor, gap, 1, arrays))
            cursor += gap
        lowered.append(emit)
        cursor = offset + emit.size
    return lowered, masks


def _emit_size(emit: EmitField) -> int:
    base = _rust_size(emit.rust_type)
    total = base
    for length in emit.arrays:
        total *= length
    return total


def _rust_size(rust: str) -> int:
    sizes = {
        "u8": 1, "i8": 1, "u16": 2, "i16": 2, "Le16": 2, "Be16": 2,
        "u32": 4, "i32": 4, "Le32": 4, "Be32": 4, "u64": 8, "i64": 8,
        "Le64": 8, "Be64": 8,
    }
    if rust in sizes:
        return sizes[rust]
    if rust.startswith("["):
        raise LayoutError(rust)
    return 0


def sim_offsets(fields: list[EmitField], packed: bool, force_align: int | None):
    offset = 0
    natural = 1
    found = []
    for item in fields:
        align = 1 if packed else item.align
        offset = align_up(offset, align)
        found.append(offset)
        offset += item.size
        if not packed:
            natural = max(natural, align)
    if force_align:
        if force_align < natural:
            return None
        chosen = force_align
    elif packed:
        chosen = 1
    else:
        chosen = natural
    return found, align_up(offset, chosen), chosen, offset


def _field_align(item: EmitField) -> int:
    if item.arrays:
        return _type_align(item.rust_type)
    return _type_align(item.rust_type)


def _type_align(rust: str) -> int:
    aligns = {
        "u8": 1, "i8": 1, "u16": 2, "i16": 2, "Le16": 2, "Be16": 2,
        "u32": 4, "i32": 4, "Le32": 4, "Be32": 4, "u64": 8, "i64": 8,
        "Le64": 8, "Be64": 8,
    }
    return aligns.get(rust, 1)


def choose_form(fields: list[EmitField], size: int, align: int) -> tuple[str, int | None]:
    plain = sim_offsets(fields, False, None)
    if plain and _matches(fields, plain, size, align):
        return "c", None
    raised = sim_offsets(fields, False, align)
    if raised and _matches(fields, raised, size, align):
        return "align", align
    packed = sim_offsets(fields, True, None)
    if packed and align == 1 and _matches(fields, packed, size, align):
        return "packed", None
    if packed:
        offsets, _size, _align, raw = packed
        if raw <= size and align_up(raw, align) == size and _offset_ok(fields, offsets):
            return "wrap", align
    raise LayoutError("no rust repr")


def _offset_ok(fields: list[EmitField], offsets: list[int]) -> bool:
    return all(item.offset == got for item, got in zip(fields, offsets, strict=True))


def _matches(fields: list[EmitField], sim, size: int, align: int) -> bool:
    offsets, got_size, got_align, _raw = sim
    return got_size == size and got_align == align and _offset_ok(fields, offsets)


def build_record(name: str, kind: str, fields: list[LayField], packed: bool, align_attr: int | None) -> tuple[list[EmitField], int, int, str, int | None, list]:
    if kind == "union":
        return _layout_union(fields, packed, align_attr)
    placed, runs, size, align, flex = layout_struct(fields, packed, align_attr)
    lowered, mask_bits = lower_fields(placed, runs)
    if flex is not None:
        # The flexible member is omitted. Size stays the prefix.
        pass
    form, align_n = choose_form(lowered, size, align)
    return lowered, size, align, form, align_n, mask_bits


def _layout_union(fields: list[LayField], packed: bool, align_attr: int | None):
    if any(item.bit_width is not None or item.flexible for item in fields):
        raise LayoutError("union bitfield or flexible member")
    size = 0
    align = 1
    for item in fields:
        size = max(size, item.size)
        if not packed:
            align = max(align, item.align)
    if align_attr:
        align = align_attr
    elif packed:
        align = 1
    size = align_up(size, align)
    lowered = []
    for item in fields:
        item.offset = 0
        lowered.append(EmitField(
            item.name or "field", item.c_type, item.rust_type, 0,
            item.size, item.align, item.arrays,
        ))
    form, align_n = choose_form(lowered, size, align) if False else ("c", None)
    if packed and align_attr:
        form, align_n = "wrap", align_attr
    elif packed:
        form, align_n = "packed", None
    elif align_attr and align_attr > 1:
        natural = max((item.align for item in lowered), default=1)
        if align_attr > natural:
            form, align_n = "align", align_attr
    return lowered, size, align, form, align_n, []


def self_check() -> None:
    def field(name, size, align, c, rust, bits=None):
        return LayField(name, size, align, c, rust, bits)

    cases = []
    four = [field(n, 4, 4, "__le32", "Le32") for n in ("buf0", "ctrl", "buf1", "info")]
    cases.append((four, True, 4, 16, 4, [0, 4, 8, 12]))
    bits_a = [field(None, 2, 2, "u16", "u16", 4), field("b", 1, 1, "u8", "u8")]
    cases.append((bits_a, False, None, 2, 2, [1]))
    bits_b = [
        field(None, 1, 1, "u8", "u8", 5),
        field(None, 1, 1, "u8", "u8", 5),
        field("c", 1, 1, "u8", "u8"),
    ]
    cases.append((bits_b, False, None, 3, 1, [2]))
    bits_d = [
        field(None, 4, 4, "u32", "u32", 24),
        field(None, 1, 1, "u8", "u8", 8),
        field("c", 1, 1, "u8", "u8"),
    ]
    cases.append((bits_d, False, None, 8, 4, [4]))
    packed_odd = [field("a", 1, 1, "u8", "u8"), field("b", 4, 4, "u32", "u32"), field("c", 1, 1, "u8", "u8")]
    cases.append((packed_odd, True, 4, 8, 4, [0, 1, 5]))
    for fields, packed, attr, size, align, offsets in cases:
        placed, _runs, got_size, got_align, _flex = layout_struct(fields, packed, attr)
        real = [item.offset for item in placed]
        if got_size != size or got_align != align or real != offsets:
            raise SystemExit(f"layout self-check {got_size} {got_align} {real} != {size} {align} {offsets}")
    lowered, size, align, form, _n, _m = build_record(
        "odd", "struct", packed_odd, True, 4,
    )
    if form != "wrap" or size != 8 or align != 4:
        raise SystemExit(f"wrap self-check {form} {size} {align}")
    _ = lowered


def line_number(offset_map: list[tuple[int, int]], offset: int) -> int:
    found = 0
    for start, number in offset_map:
        if start <= offset:
            found = number
        else:
            break
    return found


def preprocess(path: Path, rel: str, selected: set[str]) -> tuple[str, list, list[str], list[tuple[int, int]]]:
    text = join_lines(strip_comments(path.read_text()))
    lines = text.splitlines()
    stack: list[tuple[bool, bool, bool]] = []

    def live() -> bool:
        return _live(stack)

    code: list[str] = []
    defines = []
    includes = []
    offset_map: list[tuple[int, int]] = []
    cursor = 0
    for number, line in enumerate(lines, start=1):
        stripped = line.strip()
        if stripped.startswith("#"):
            _directive(stripped, stack, defines, includes, rel, selected, number, live())
            continue
        if live():
            code.append(line)
            offset_map.append((cursor, number))
            cursor += len(line) + 1
            code.append("\n")
    return "".join(code), defines, includes, offset_map


def _live(stack: list[tuple[bool, bool, bool]]) -> bool:
    for parent_active, cond, seen_else in stack:
        if not parent_active:
            return False
        active = cond if not seen_else else not cond
        if not active:
            return False
    return True


def _directive(line, stack, defines, includes, rel, selected, number, active) -> None:
    if line.startswith("#ifndef"):
        parent = _live(stack)
        stack.append((parent, True, False))
        return
    if line.startswith("#ifdef"):
        name = line.split(None, 1)[1].strip() if len(line.split(None, 1)) > 1 else ""
        cond = False if name.startswith("CONFIG_") else False
        parent = _live(stack)
        stack.append((parent, cond, False))
        return
    if line.startswith("#if"):
        cond = _if_cond(line)
        parent = _live(stack)
        stack.append((parent, cond, False))
        return
    if line.startswith("#else"):
        if stack:
            parent, cond, _seen = stack[-1]
            stack[-1] = (parent, cond, True)
        return
    if line.startswith("#endif"):
        if stack:
            stack.pop()
        return
    if not active:
        return
    if line.startswith("#define"):
        parsed = _parse_define(line)
        if parsed is not None:
            defines.append((*parsed, number))
        return
    if line.startswith("#include"):
        match = re.search(r'[<"]([^>"]+)[>"]', line)
        if not match:
            return
        target = match.group(1)
        quote = '"' in line
        if not quote:
            return
        if target.startswith("linux/") or target.startswith("net/"):
            return
        resolved = os.path.normpath(os.path.join(os.path.dirname(rel), target))
        if resolved in selected:
            includes.append(resolved)


def _if_cond(line: str) -> bool:
    expr = line[3:].strip()
    if "IS_ENABLED" in expr:
        return False
    if re.search(r"defined\s*\(\s*CONFIG_", expr):
        return False
    return False


def _parse_define(line: str):
    match = re.match(r"#define\s+(\w+)(?:\(([^)]*)\))?(?:\s+(.*))?$", line.strip())
    if not match:
        return None
    name, params, body = match.group(1), match.group(2), match.group(3) or ""
    if params is None:
        return name, None, body.strip()
    names = [part.strip() for part in params.split(",")] if params.strip() else []
    if any(part == "..." or part.endswith("...") for part in names):
        return name, ["..."], body.strip()
    return name, names, body.strip()


class FlexMark(Exception):
    """A field uses DECLARE_FLEX_ARRAY, which has no standalone size."""


class Pending(LayoutError):
    """A field type is a record that may resolve on a later pass."""


@dataclass
class Decl:
    name: str | None
    pointer: bool
    arrays: list
    bits: object | None


@dataclass
class RawField:
    name: str | None
    specs: list[str]
    pointer: bool
    arrays: list
    bits: object | None
    nested: object | None = None
    promoted: bool = False


@dataclass
class RawRecord:
    header: str
    kind: str
    name: str | None
    fields: list
    packed: bool
    align_expr: object | None
    source: str
    order: int
    named: bool = True
    skip_reason: str | None = None
    pending_reason: str | None = None


@dataclass
class RawEnum:
    header: str
    name: str | None
    consts: list
    source: str
    order: int


def _skip_parens(cur: Cur) -> None:
    depth = 0
    while cur.peek():
        token = cur.bump()
        if token.value == "(":
            depth += 1
        elif token.value == ")":
            depth -= 1
            if depth == 0:
                return
    raise ParseError("unclosed paren")


def _skip_to_semi(cur: Cur) -> None:
    depth_paren = 0
    depth_brace = 0
    while cur.peek():
        token = cur.bump()
        if token.value == "(":
            depth_paren += 1
        elif token.value == ")":
            depth_paren = max(0, depth_paren - 1)
        elif token.value == "{":
            depth_brace += 1
        elif token.value == "}":
            depth_brace = max(0, depth_brace - 1)
        elif token.value == ";" and depth_paren == 0 and depth_brace == 0:
            return


def parse_declarator(cur: Cur) -> Decl:
    pointer = False
    while cur.eat("punct", "*"):
        pointer = True
        while cur.peek() and cur.peek().kind == "ident" and cur.peek().value in QUALIFIERS:
            cur.bump()
    if cur.eat("punct", "("):
        while cur.eat("punct", "*"):
            pointer = True
            while cur.peek() and cur.peek().kind == "ident" and cur.peek().value in QUALIFIERS:
                cur.bump()
        name = cur.bump().value if cur.peek() and cur.peek().kind == "ident" else None
        cur.expect("punct", ")")
        if cur.at("punct", "("):
            _skip_parens(cur)
        return Decl(name, True, [], None)
    name = None
    if cur.peek() and cur.peek().kind == "ident":
        name = cur.bump().value
    arrays: list = []
    while cur.eat("punct", "["):
        if cur.eat("punct", "]"):
            arrays.append("flex")
            continue
        arrays.append(cur.parse_expr())
        cur.expect("punct", "]")
    bits = cur.parse_expr() if cur.eat("punct", ":") else None
    return Decl(name, pointer, arrays, bits)


def parse_declarators(cur: Cur) -> list[Decl]:
    decls = [parse_declarator(cur)]
    while cur.eat("punct", ","):
        decls.append(parse_declarator(cur))
    return decls


def parse_specs(cur: Cur) -> list[str]:
    specs: list[str] = []
    while True:
        if cur.word("struct") or cur.word("union") or cur.word("enum"):
            if cur.definition_ahead():
                break
            specs.append(cur.bump().value)
            specs.append(cur.expect("ident").value)
            continue
        if cur.peek() and cur.peek().kind == "ident" and cur.peek().value in TYPE_WORDS:
            specs.append(cur.bump().value)
            continue
        break
    if specs:
        return specs
    if cur.peek() and cur.peek().kind == "ident":
        if cur.peek(1) and cur.peek(1).value == "(":
            if cur.peek().value == "DECLARE_FLEX_ARRAY":
                raise FlexMark()
            raise ParseError(f"macro member {cur.peek().value}")
        specs.append(cur.bump().value)
        return specs
    raise ParseError("type")


def parse_record_body(cur: Cur, header: str):
    start = cur.peek()
    if start is None:
        raise ParseError("record")
    kind = cur.bump().value
    name = None
    if cur.peek() and cur.peek().kind == "ident" and cur.peek(1) and cur.peek(1).value == "{":
        name = cur.bump().value
    cur.expect("punct", "{")
    fields: list[RawField] = []
    skip_reason = None
    while not cur.at("punct", "}"):
        if cur.peek() is None:
            raise ParseError("unclosed record")
        try:
            fields.extend(parse_members(cur, header))
        except FlexMark:
            skip_reason = "depends on DECLARE_FLEX_ARRAY"
            _consume_until_close(cur)
            break
    if skip_reason is None:
        cur.expect("punct", "}")
    packed, align_expr = cur.parse_attrs()
    end = cur.toks[cur.i - 1].end
    source = cur.src[start.start:end]
    return kind, name, fields, packed, align_expr, source, skip_reason, start.start


def _consume_until_close(cur: Cur) -> None:
    depth = 1
    while cur.peek():
        token = cur.bump()
        if token.value == "{":
            depth += 1
        elif token.value == "}":
            depth -= 1
            if depth == 0:
                return
    raise ParseError("unclosed record")


def parse_members(cur: Cur, header: str) -> list[RawField]:
    if (cur.word("struct") or cur.word("union")) and cur.definition_ahead():
        kind, name, fields, packed, align_expr, source, skip_reason, start = parse_record_body(
            cur, header
        )
        nested = RawRecord(
            header, kind, name, fields, packed, align_expr, source, start, name is not None, skip_reason
        )
        if cur.peek() and cur.peek().kind != "ident" and not cur.at("punct", "*") and not cur.at("punct", ";"):
            raise ParseError("nested declarator")
        if cur.at("punct", ";"):
            cur.expect("punct", ";")
            return [RawField(None, [], False, [], None, nested, True)]
        decls = parse_declarators(cur)
        _reject_field_attr(cur)
        cur.expect("punct", ";")
        return [
            RawField(decl.name, [], decl.pointer, decl.arrays, decl.bits, nested, False) for decl in decls
        ]
    specs = parse_specs(cur)
    decls = parse_declarators(cur)
    _reject_field_attr(cur)
    cur.expect("punct", ";")
    return [
        RawField(decl.name, specs, decl.pointer, decl.arrays, decl.bits) for decl in decls
    ]


def _reject_field_attr(cur: Cur) -> None:
    if cur.word("__attribute__") or cur.word("__packed") or cur.word("__aligned"):
        raise ParseError("field attribute")


def parse_top_record(cur: Cur, header: str) -> RawRecord:
    kind, name, fields, packed, align_expr, source, skip_reason, start = parse_record_body(cur, header)
    if cur.peek() and cur.peek().kind == "ident":
        _skip_to_semi(cur)
    else:
        cur.eat("punct", ";")
    if not source.rstrip().endswith(";"):
        source = source.rstrip() + ";"
    return RawRecord(header, kind, name, fields, packed, align_expr, source, start, name is not None, skip_reason)


def parse_enum(cur: Cur, header: str) -> RawEnum:
    start = cur.peek()
    if start is None:
        raise ParseError("enum")
    cur.bump()
    name = None
    if cur.peek() and cur.peek().kind == "ident" and cur.peek(1) and cur.peek(1).value == "{":
        name = cur.bump().value
    cur.expect("punct", "{")
    consts = []
    while not cur.at("punct", "}"):
        ident = cur.expect("ident")
        expr = None
        expr_src = None
        if cur.eat("punct", "="):
            expr_at = cur.peek().start if cur.peek() else ident.end
            expr = cur.parse_expr()
            expr_src = cur.src[expr_at:cur.toks[cur.i - 1].end].strip()
        consts.append((ident.value, expr, expr_src, ident.start))
        cur.eat("punct", ",")
    cur.expect("punct", "}")
    cur.parse_attrs()
    end = cur.toks[cur.i - 1].end
    if cur.peek() and cur.peek().kind == "ident":
        _skip_to_semi(cur)
        end = cur.toks[cur.i - 1].end
    else:
        eaten = cur.eat("punct", ";")
        if eaten:
            end = eaten.end
    source = cur.src[start.start:end]
    if not source.rstrip().endswith(";"):
        source = source.rstrip() + ";"
    return RawEnum(header, name, consts, source, start.start)


def skip_decl(cur: Cur, header: str, order_of) -> Skip | None:
    start = cur.peek()
    if start is None:
        return None
    name = None
    kind = "declaration"
    depth_paren = 0
    depth_brace = 0
    prev = None
    while cur.peek():
        token = cur.bump()
        if token.value == "(":
            if depth_paren == 0 and depth_brace == 0 and prev and prev.kind == "ident":
                name = prev.value
                kind = "function"
            depth_paren += 1
        elif token.value == ")":
            depth_paren = max(0, depth_paren - 1)
        elif token.value == "{":
            depth_brace += 1
            if kind == "function":
                kind = "inline function"
        elif token.value == "}":
            depth_brace = max(0, depth_brace - 1)
            if depth_brace == 0 and depth_paren == 0 and kind == "inline function":
                cur.eat("punct", ";")
                break
        elif token.value == ";" and depth_paren == 0 and depth_brace == 0:
            break
        prev = token
    if name and kind in {"function", "inline function"}:
        reason = "inline function" if kind == "inline function" else "function prototype"
        return Skip(header, name, reason, order_of(start.start))
    return None


def parse_unit(cur: Cur, header: str, order_of):
    records = []
    enums = []
    skips = []
    while cur.peek():
        start = cur.i
        try:
            if (cur.word("struct") or cur.word("union") or cur.word("enum")) and cur.definition_ahead():
                if cur.word("enum"):
                    enums.append(parse_enum(cur, header))
                else:
                    records.append(parse_top_record(cur, header))
                continue
            skipped = skip_decl(cur, header, order_of)
            if skipped:
                skips.append(skipped)
        except ParseError as err:
            token = cur.peek()
            offset = token.start if token else 0
            skips.append(Skip(header, "declaration", f"unparsed declaration ({err})", order_of(offset)))
            if cur.i == start:
                cur.bump()
            else:
                cur.resync()
    return records, enums, skips
@dataclass
class Bag:
    rel: str
    includes: list[str]
    macros: list[Macro]
    enum_consts: list[EnumConst]
    raw_enums: list[RawEnum]
    raw_records: list[RawRecord]
    records: list[Record]
    skips: list[Skip]
    define_lines: list[str]
    fn_header: dict[str, str]
    env: Env


def has_cast(node: object) -> bool:
    if isinstance(node, Cast):
        return True
    if isinstance(node, Unary):
        return has_cast(node.expr)
    if isinstance(node, Binary):
        return has_cast(node.left) or has_cast(node.right)
    if isinstance(node, Ternary):
        return has_cast(node.cond) or has_cast(node.then) or has_cast(node.else_)
    if isinstance(node, Call):
        return any(has_cast(arg) for arg in node.args)
    return False


def parse_body(body: str) -> object:
    cursor = Cur(tokenize(body), body)
    expr = cursor.parse_expr()
    if cursor.peek() is not None:
        raise ParseError("trailing tokens")
    return expr


def decode_string(body: str) -> bytes | None:
    text = body.strip()
    if len(text) < 2 or text[0] != '"' or text[-1] != '"':
        return None
    inner = text[1:-1]
    out = bytearray()
    index = 0
    escapes = {"n": 10, "t": 9, "r": 13, "\\": 92, '"': 34, "0": 0}
    while index < len(inner):
        char = inner[index]
        if char == '"':
            return None
        if char != "\\":
            out.append(ord(char))
            index += 1
            continue
        index += 1
        if index >= len(inner) or inner[index] not in escapes:
            return None
        out.append(escapes[inner[index]])
        index += 1
    return bytes(out)


def rust_bytes(data: bytes) -> str:
    chars: list[str] = []
    for byte in data:
        if byte == 92:
            chars.append("\\\\")
        elif byte == 34:
            chars.append('\\"')
        elif 32 <= byte < 127:
            chars.append(chr(byte))
        else:
            chars.append(f"\\x{byte:02x}")
    return 'b"' + "".join(chars) + '"'


def load_header(root: Path, rel: str, selected: set[str]) -> Bag:
    code, defines, includes, offset_map = preprocess(root / rel, rel, selected)
    order_of = lambda offset: line_number(offset_map, offset)  # noqa: E731
    cursor = Cur(tokenize(code), code)
    raw_records, raw_enums, skips = parse_unit(cursor, rel, order_of)
    macros: list[Macro] = []
    for name, params, body, number in defines:
        if params == ["..."]:
            skips.append(Skip(rel, name, "variadic macro", number))
            continue
        if not body and name.startswith("_"):
            continue
        macros.append(Macro(rel, name, params, body, None, number, "int"))
    env = Env({}, {})
    return Bag(rel, includes, macros, [], raw_enums, raw_records, [], skips, [], {}, env)


def closure_of(rel: str, bags: dict[str, Bag]) -> list[str]:
    seen: list[str] = []
    walking: set[str] = set()

    def visit(name: str) -> None:
        if name in seen or name in walking or name not in bags:
            return
        walking.add(name)
        for inc in bags[name].includes:
            visit(inc)
        walking.remove(name)
        seen.append(name)

    visit(rel)
    return seen


def pick_sample(macro: Macro, env: Env) -> tuple[tuple[int, ...], int] | None:
    arity = len(macro.params or [])
    if arity == 0:
        candidates = [()]
    elif arity == 1:
        candidates = [(0,), (1,), (2,), (4,), (8,), (15,)]
    elif arity == 2:
        candidates = [(0, 0), (1, 0), (0, 1), (1, 1), (7, 4), (15, 0), (31, 0)]
    else:
        candidates = [tuple(0 for _ in range(arity)), tuple(1 for _ in range(arity))]
    assert macro.params is not None
    for sample in candidates:
        try:
            value = eval_expr(macro.expr, env, dict(zip(macro.params, sample, strict=True)))
        except EvalError:
            continue
        return sample, value
    return None


def bind_header(bag: Bag, bags: dict[str, Bag]) -> None:
    env = Env({}, {})
    fn_header: dict[str, str] = {}
    for inc in closure_of(bag.rel, bags):
        if inc == bag.rel:
            continue
        parent = bags[inc]
        env.values.update(parent.env.values)
        env.functions.update(parent.env.functions)
        fn_header.update(parent.fn_header)
    skipped: list[Macro] = []
    functions: list[Macro] = []
    for macro in bag.macros:
        reason = macro_rejected(macro.body)
        if macro.params == ["..."]:
            reason = "variadic macro"
        if reason:
            bag.skips.append(Skip(bag.rel, macro.name, reason, macro.order))
            skipped.append(macro)
            continue
        if macro.params is None:
            text = decode_string(macro.body)
            if text is not None:
                macro.kind = "str"
                macro.text = text
                continue
        try:
            expr = parse_body(macro.body)
        except (ParseError, EvalError):
            bag.skips.append(Skip(bag.rel, macro.name, "not an integer constant", macro.order))
            skipped.append(macro)
            continue
        macro.expr = expr
        if macro.params is None:
            try:
                macro.value = eval_expr(expr, env)
            except EvalError:
                macro.kind = "retry"
                continue
            macro.kind = "int"
            env.values[macro.name] = macro.value
            continue
        if has_cast(expr):
            bag.skips.append(Skip(bag.rel, macro.name, "cast expression", macro.order))
            skipped.append(macro)
            continue
        macro.kind = "fn"
        env.functions[macro.name] = (macro.params, expr)
        fn_header[macro.name] = bag.rel
        functions.append(macro)
    for macro in list(bag.macros):
        if macro.kind != "retry":
            continue
        try:
            assert macro.expr is not None
            macro.value = eval_expr(macro.expr, env)
        except EvalError:
            bag.skips.append(Skip(bag.rel, macro.name, "not an integer constant", macro.order))
            skipped.append(macro)
            continue
        macro.kind = "int"
        env.values[macro.name] = macro.value
    for macro in functions:
        picked = pick_sample(macro, env)
        if picked is None:
            bag.skips.append(Skip(bag.rel, macro.name, "not an integer constant", macro.order))
            env.functions.pop(macro.name, None)
            fn_header.pop(macro.name, None)
            skipped.append(macro)
            continue
        macro.sample, macro.value = picked
    kept = [macro for macro in bag.macros if macro not in skipped and macro.kind != "retry"]
    last = {macro.name: macro for macro in kept}
    bag.macros = [macro for macro in kept if last[macro.name] is macro]
    _bind_enums(bag, env)
    bag.env = env
    bag.fn_header = fn_header


def _bind_enums(bag: Bag, env: Env) -> None:
    consts: list[EnumConst] = []
    macro_names = set(env.values) | set(env.functions)
    seen_names: set[str] = set()
    for enum in bag.raw_enums:
        running = 0
        chain = True
        previous = None
        for name, expr, expr_src, offset in enum.consts:
            if name in seen_names:
                bag.skips.append(Skip(bag.rel, name, "duplicate name", enum.order))
                chain = False
                previous = None
                continue
            if name in macro_names or name in env.functions:
                bag.skips.append(Skip(bag.rel, name, "name collides with a macro", enum.order))
                chain = False
                previous = None
                continue
            if expr is not None:
                try:
                    value = eval_expr(expr, env)
                except EvalError:
                    bag.skips.append(Skip(bag.rel, name, "not an integer constant", enum.order))
                    chain = False
                    previous = None
                    continue
                chain = True
            elif chain:
                value = u64(running)
            else:
                bag.skips.append(Skip(bag.rel, name, "implicit enum value", enum.order))
                chain = False
                previous = None
                continue
            item = EnumConst(bag.rel, name, value, offset, expr_src, anchor=previous)
            consts.append(item)
            seen_names.add(name)
            env.values[name] = value
            running = u64(value + 1)
            previous = name
            chain = True
    bag.enum_consts = consts


def macro_define(macro: Macro) -> str:
    if macro.params is None:
        return f"#define {macro.name} {macro.body}"
    params = ", ".join(macro.params)
    return f"#define {macro.name}({params}) {macro.body}"


def enum_define(item: EnumConst) -> str:
    if item.expr_src:
        return f"#define {item.name} ({item.expr_src})"
    if item.anchor:
        return f"#define {item.name} (({item.anchor}) + 1ull)"
    return f"#define {item.name} {item.value}ull"


def value_expr(item: Macro | EnumConst) -> str:
    if isinstance(item, Macro) and item.params is not None:
        args = ", ".join(str(part) for part in (item.sample or ()))
        return f"{item.name}({args})"
    return item.name


def _const_program(defines: list[str], items: list) -> str:
    lines = [stub(), "#include <stdio.h>"]
    lines.extend(defines)
    for item in items:
        if isinstance(item, Macro):
            lines.append(macro_define(item))
        else:
            lines.append(enum_define(item))
    lines.append("int main(void) {")
    for index, item in enumerate(items):
        expr = value_expr(item)
        lines.append(
            f'_Static_assert((((unsigned long long)({expr})) | 1ull) >= 1ull, "ice{index}");'
        )
        kind = "F" if isinstance(item, Macro) and item.params is not None else "M"
        lines.append(f'printf("{kind} {item.name} %llu\\n", (unsigned long long)({expr}));')
    lines.append("return 0; }")
    return "\n".join(lines)


def _parse_const_output(text: str) -> dict[str, int]:
    found = {}
    for line in text.splitlines():
        parts = line.split()
        if len(parts) == 3 and parts[0] in {"M", "F"}:
            found[parts[1]] = int(parts[2])
    return found


def confirm_constants(items: list, prefix: list[str] | None = None) -> list:
    prefix = prefix or []
    if not items:
        return []
    ok, out = clang_run(_const_program(prefix, items))
    if ok:
        got = _parse_const_output(out)
        accepted = []
        for item in items:
            if got.get(item.name) == item.value:
                item.clang = got[item.name]
                accepted.append(item)
        return accepted
    if len(items) == 1:
        return []
    mid = len(items) // 2
    left = confirm_constants(items[:mid], prefix)
    right_prefix = prefix + [_define_of(item) for item in left]
    right = confirm_constants(items[mid:], right_prefix)
    return left + right


def _define_of(item: Macro | EnumConst) -> str:
    if isinstance(item, Macro):
        return macro_define(item)
    return enum_define(item)


def accept_constants(bag: Bag) -> None:
    items: list = []
    for macro in bag.macros:
        if macro.kind == "str":
            continue
        items.append((macro.order, 0, macro))
    for const in bag.enum_consts:
        items.append((const.order, 1, const))
    items.sort(key=lambda part: (part[0], part[1]))
    ordered = [part[2] for part in items]
    accepted = confirm_constants(ordered)
    accepted_ids = {id(item) for item in accepted}
    kept_macros = []
    for macro in bag.macros:
        if macro.kind == "str":
            kept_macros.append(macro)
            continue
        if id(macro) in accepted_ids:
            kept_macros.append(macro)
            bag.define_lines.append(macro_define(macro))
            continue
        bag.skips.append(Skip(bag.rel, macro.name, "linux value mismatch", macro.order))
        bag.env.values.pop(macro.name, None)
        bag.env.functions.pop(macro.name, None)
    bag.macros = kept_macros
    kept_enums = []
    for const in bag.enum_consts:
        if id(const) in accepted_ids:
            kept_enums.append(const)
            bag.define_lines.append(enum_define(const))
            continue
        bag.skips.append(Skip(bag.rel, const.name, "linux value mismatch", const.order))
        bag.env.values.pop(const.name, None)
    bag.enum_consts = kept_enums


def words_of(specs: list[str]) -> list[str]:
    return [word for word in specs if word not in QUALIFIERS]


def assign_names(raw: RawRecord) -> None:
    for index, field in enumerate(raw.fields):
        if field.nested is None:
            continue
        if not field.nested.name:
            piece = field.name or f"anon_{index}"
            field.nested.name = f"{raw.name}_{piece}"
            field.nested.named = False
        assign_names(field.nested)


def index_raw(raw: RawRecord, into: dict[str, list[RawRecord]]) -> None:
    if raw.name:
        into.setdefault(raw.name, []).append(raw)
    for field in raw.fields:
        if field.nested is not None:
            index_raw(field.nested, into)


def array_shape(arrays: list, env: Env) -> tuple[tuple[int, ...], bool]:
    dims: list[int] = []
    flex = False
    for item in arrays:
        if item == "flex":
            flex = True
            continue
        if isinstance(item, IntLit) and item.value == 0:
            flex = True
            continue
        length = eval_expr(item, env)
        if length <= 0:
            raise LayoutError("array length")
        dims.append(length)
    return tuple(dims), flex


def module_path(rel: str) -> str:
    return "::".join(Path(rel).with_suffix("").parts)


def rust_type_name(owner: str, home: str, name: str) -> str:
    if owner == home:
        return name
    return f"crate::{module_path(home)}::{name}"


def find_record(name: str, bag: Bag, bags: dict[str, Bag], resolved: dict, raw_index: dict) -> Record:
    scope = set(closure_of(bag.rel, bags))
    hits = [resolved[(rel, name)] for rel in scope if (rel, name) in resolved]
    if len(hits) == 1:
        return hits[0]
    if len(hits) > 1:
        raise LayoutError(f"ambiguous type {name}")
    waiting = [raw for raw in raw_index.get(name, []) if raw.header in scope and raw.skip_reason is None]
    if waiting:
        raise Pending(name)
    everywhere = [rec for (rel, tag), rec in resolved.items() if tag == name]
    if len(everywhere) == 1:
        return everywhere[0]
    if len(raw_index.get(name, [])) > 1:
        raise LayoutError(f"ambiguous type {name}")
    if name in raw_index:
        raise Pending(name)
    raise LayoutError(f"depends on {name}")


def known_enums(bag: Bag, bags: dict[str, Bag]) -> dict[str, RawEnum]:
    found: dict[str, RawEnum] = {}
    for rel in closure_of(bag.rel, bags):
        for enum in bags[rel].raw_enums:
            if enum.name and enum.name not in found:
                found[enum.name] = enum
    return found


def try_resolve(
    raw: RawRecord,
    bag: Bag,
    bags: dict[str, Bag],
    env: Env,
    resolved: dict,
    raw_index: dict,
    enums: dict[str, RawEnum],
    nested: bool = False,
) -> Record:
    if raw.skip_reason:
        raise LayoutError(raw.skip_reason)
    if not raw.name:
        raise LayoutError("anonymous record")
    key = (raw.header, raw.name)
    if key in resolved:
        return resolved[key]
    lay_fields: list[LayField] = []
    children: list[Record] = []
    deps: list[Record] = []
    defined: set[str] = set()
    enum_deps: list[str] = []
    if raw.named:
        defined.add(raw.name)
    flex_info = None
    for field in raw.fields:
        if field.pointer:
            raise LayoutError(f"pointer field {field.name or 'anon'}")
        if field.nested is not None:
            if field.nested.skip_reason:
                raise LayoutError(field.nested.skip_reason)
            child = try_resolve(
                field.nested, bag, bags, env, resolved, raw_index, enums, nested=True,
            )
            children.append(child)
            if child.name:
                defined.add(child.name)
            dims, flex = array_shape(field.arrays, env)
            if flex:
                raise LayoutError("flexible nested record")
            total = child.size
            for dim in dims:
                total *= dim
            rust = rust_type_name(raw.header, child.header, child.name)
            c_type = f"{child.kind} {child.name}"
            lay = LayField(
                field.name or child.name, total, child.align, c_type, rust,
                arrays=dims, nested=child, promoted=field.promoted,
            )
            lay_fields.append(lay)
            continue
        words = words_of(field.specs)
        if not words:
            raise LayoutError("empty type")
        nested_rec = None
        if words[0] in {"struct", "union"}:
            if len(words) < 2:
                raise LayoutError("missing tag")
            nested_rec = find_record(words[1], bag, bags, resolved, raw_index)
            if nested_rec.name not in defined and not any(
                dep.header == nested_rec.header and dep.name == nested_rec.name for dep in deps
            ):
                deps.append(nested_rec)
            size, align = nested_rec.size, nested_rec.align
            c_type = f"{nested_rec.kind} {nested_rec.name}"
            rust = rust_type_name(raw.header, nested_rec.header, nested_rec.name)
        elif words[0] == "enum":
            if len(words) < 2 or words[1] not in enums:
                tag = words[1] if len(words) > 1 else "anonymous"
                raise LayoutError(f"depends on enum {tag}")
            enum_deps.append(words[1])
            size, align, c_type, rust = 4, 4, "u32", "u32"
        else:
            key_name = " ".join(words)
            if key_name not in SCALARS:
                raise LayoutError(f"depends on {key_name}")
            size, align, c_type, rust = SCALARS[key_name]
        dims, flex = array_shape(field.arrays, env)
        width = eval_expr(field.bits, env) if field.bits is not None else None
        if width is not None and (width < 0 or width > 64):
            raise LayoutError("bitfield width")
        if flex:
            elem = rust
            for dim in reversed(dims):
                elem = f"[{elem}; {dim}]"
            flex_info = (c_type if not dims else c_type, elem)
            lay_fields.append(LayField(
                field.name, 0, align, c_type, rust, flexible=True, elem_spell=c_type,
            ))
            continue
        total = size
        for dim in dims:
            total *= dim
        lay_fields.append(LayField(
            field.name, total, align, c_type, rust, width, dims, nested=nested_rec,
        ))
    align_attr = eval_expr(raw.align_expr, env) if raw.align_expr is not None else None
    lowered, size, align, form, align_n, masks = build_record(
        raw.name, raw.kind, lay_fields, raw.packed, align_attr,
    )
    checks = _linux_checks(lay_fields)
    record = Record(
        header=raw.header,
        kind=raw.kind,
        name=raw.name,
        fields=lowered,
        size=size,
        align=align,
        form=form,
        align_n=align_n,
        inner=f"{raw.name}__packed" if form == "wrap" else None,
        masks=masks,
        checks=checks,
        source=raw.source if raw.source.rstrip().endswith(";") else raw.source.rstrip() + ";",
        order=raw.order,
        flex=flex_info,
        includes=[],
        standalone=raw.named,
        children=children,
        dep_keys=deps,
        defined_inside=defined,
        enum_deps=enum_deps,
        flex_rust=flex_info[1] if flex_info else "",
    )
    resolved[key] = record
    if nested:
        bag.records.append(record)
    return record


def _linux_checks(fields: list[LayField]) -> list[Check]:
    checks: list[Check] = []
    for lay in fields:
        if lay.bit_width is not None or lay.flexible:
            continue
        nested = lay.nested if isinstance(lay.nested, Record) else None
        if nested is not None and lay.promoted:
            for check in nested.checks:
                checks.append(Check("field", check.path, (lay.offset or 0) + (check.offset or 0)))
            continue
        if not lay.name:
            continue
        checks.append(Check("field", lay.name, lay.offset))
        if nested is not None and not lay.arrays:
            for check in nested.checks:
                path = f"{lay.name}.{check.path}"
                checks.append(Check("field", path, (lay.offset or 0) + (check.offset or 0)))
    return checks


def share_unique_constants(bags: dict[str, Bag]) -> None:
    found: dict[str, set[int]] = {}
    for bag in bags.values():
        for macro in bag.macros:
            if macro.kind == "int" and macro.value is not None:
                found.setdefault(macro.name, set()).add(int(macro.value))
        for const in bag.enum_consts:
            found.setdefault(const.name, set()).add(int(const.value))
    unique = {name: next(iter(values)) for name, values in found.items() if len(values) == 1}
    for bag in bags.values():
        bag.env.fallback = {name: value for name, value in unique.items() if name not in bag.env.values}


def resolve_records(bags: dict[str, Bag]) -> dict:
    share_unique_constants(bags)
    raw_index: dict[str, list[RawRecord]] = {}
    for bag in bags.values():
        for raw in bag.raw_records:
            if raw.skip_reason:
                bag.skips.append(Skip(bag.rel, raw.name or "record", raw.skip_reason, raw.order))
                continue
            if not raw.name:
                bag.skips.append(Skip(bag.rel, "record", "anonymous record", raw.order))
                continue
            assign_names(raw)
            index_raw(raw, raw_index)
    resolved: dict = {}
    pending = [(bag, raw) for bag in bags.values() for raw in bag.raw_records if raw.name and not raw.skip_reason]
    while pending:
        next_pending = []
        progressed = False
        for bag, raw in pending:
            if (raw.header, raw.name) in resolved:
                progressed = True
                continue
            try:
                record = try_resolve(
                    raw, bag, bags, bag.env, resolved, raw_index, known_enums(bag, bags),
                )
            except Pending:
                next_pending.append((bag, raw))
                continue
            except (LayoutError, EvalError) as err:
                bag.skips.append(Skip(bag.rel, raw.name or "record", str(err), raw.order))
                progressed = True
                continue
            bag.records.append(record)
            progressed = True
        if not progressed:
            for bag, raw in next_pending:
                bag.skips.append(Skip(bag.rel, raw.name or "record", f"depends on {raw.pending_reason or 'unknown type'}", raw.order))
            break
        pending = next_pending
    return resolved


def visible_defines(bag: Bag, bags: dict[str, Bag], seen: set[str] | None = None) -> list[str]:
    seen = seen or set()
    if bag.rel in seen:
        return []
    seen.add(bag.rel)
    lines: list[str] = []
    for inc in bag.includes:
        if inc in bags:
            lines.extend(visible_defines(bags[inc], bags, seen))
    lines.extend(bag.define_lines)
    last: dict[str, int] = {}
    for index, line in enumerate(lines):
        match = re.match(r"#define\s+(\w+)", line)
        if match:
            last[match.group(1)] = index
    return [line for index, line in enumerate(lines) if not re.match(r"#define\s+(\w+)", line) or last[re.match(r"#define\s+(\w+)", line).group(1)] == index]


def probe_program(record: Record, prelude: str) -> str:
    body = normalize_record(record.source).strip()
    if body.endswith(";"):
        body = body[:-1].rstrip()
    lines = [stub(), "#include <stdio.h>", "#include <stddef.h>", prelude]
    if record.standalone:
        lines.append(body + ";")
        qualifier = f"{record.kind} {record.name}"
        lines.append("int main(void) {")
        lines.append(f'printf("%zu %zu\\n", sizeof({qualifier}), _Alignof({qualifier}));')
        for check in record.checks:
            lines.append(f'printf("%zu\\n", offsetof({qualifier}, {check.path}));')
    else:
        lines.append(body + " mt79_probe;")
        lines.append("int main(void) {")
        lines.append('printf("%zu %zu\\n", sizeof(mt79_probe), _Alignof(__typeof__(mt79_probe)));')
        for check in record.checks:
            lines.append(f'printf("%zu\\n", offsetof(__typeof__(mt79_probe), {check.path}));')
    lines.append("return 0; }")
    return "\n".join(lines)


def external_deps(record: Record, inside: set[str], seen: set[int], out: list[Record]) -> None:
    for child in record.children:
        external_deps(child, inside, seen, out)
    for dep in record.dep_keys:
        if not isinstance(dep, Record) or dep.name in inside or id(dep) in seen:
            continue
        seen.add(id(dep))
        external_deps(dep, inside | dep.defined_inside, seen, out)
        out.append(dep)


def dep_sources(record: Record) -> list[str]:
    deps: list[Record] = []
    external_deps(record, set(record.defined_inside), set(), deps)
    texts = []
    for dep in deps:
        source = normalize_record(dep.source).strip()
        if not source.endswith(";"):
            source += ";"
        texts.append(source)
    return texts


def referenced_defines(texts: list[str], lines: list[str], bags: dict[str, Bag]) -> list[str]:
    existing: set[str] = set()
    index: dict[str, set[str]] = {}
    for line in lines:
        match = re.match(r"#define\s+(\w+)", line)
        if match:
            existing.add(match.group(1))
    for other in bags.values():
        for line in other.define_lines:
            match = re.match(r"#define\s+(\w+)", line)
            if match:
                index.setdefault(match.group(1), set()).add(line)
    extra: list[str] = []
    pending = list(texts)
    while pending:
        for name in re.findall(r"\b[A-Za-z_]\w*\b", pending.pop()):
            if name in existing:
                continue
            choices = index.get(name, set())
            existing.add(name)
            if len(choices) != 1:
                continue
            line = next(iter(choices))
            extra.append(line)
            pending.append(line)
    return extra


def record_prelude(record: Record, bag: Bag, bags: dict[str, Bag]) -> str:
    lines = visible_defines(bag, bags)
    blocked: set[str] = set()
    enum_text: list[str] = []
    enums = known_enums(bag, bags)
    for tag in record.enum_deps:
        enum = enums.get(tag)
        if enum is None:
            continue
        source = enum.source.strip()
        if not source.endswith(";"):
            source += ";"
        enum_text.append(normalize_record(source))
        for name, _expr, _src, _offset in enum.consts:
            blocked.add(name)
    if blocked:
        kept = []
        for line in lines:
            match = re.match(r"#define\s+(\w+)", line)
            if match and match.group(1) in blocked:
                continue
            kept.append(line)
        lines = kept
    dep_text = dep_sources(record)
    extra = referenced_defines([record.source, *dep_text, *enum_text], lines, bags)
    return "\n".join([*lines, *extra, *enum_text, *dep_text])


def run_probe(record: Record, bag: Bag, bags: dict[str, Bag]) -> str | None:
    ok, out = clang_run(probe_program(record, record_prelude(record, bag, bags)))
    if not ok:
        return "original record did not compile"
    rows = [line.split() for line in out.splitlines() if line.strip()]
    if not rows or len(rows[0]) != 2:
        return "original record did not compile"
    size, align = int(rows[0][0]), int(rows[0][1])
    record.clang_size = size
    record.clang_align = align
    if size != record.size or align != record.align:
        return "linux layout mismatch"
    offsets = [int(row[0]) for row in rows[1:]]
    if len(offsets) != len(record.checks):
        return "linux layout mismatch"
    for check, got in zip(record.checks, offsets, strict=True):
        check.clang = got
        if got != check.offset:
            return "linux layout mismatch"
    return None


def confirm_records(bags: dict[str, Bag], resolved: dict) -> None:
    done: dict[tuple[str, str], bool] = {}

    def confirm(record: Record) -> bool:
        key = (record.header, record.name)
        if key in done:
            return done[key]
        done[key] = False
        for dep in record.dep_keys:
            if not isinstance(dep, Record) or not confirm(dep):
                bags[record.header].skips.append(Skip(
                    record.header, record.name, f"depends on skipped {getattr(dep, 'name', 'type')}", record.order,
                ))
                return False
        for child in record.children:
            if not confirm(child):
                bags[record.header].skips.append(Skip(
                    record.header, record.name, f"depends on skipped {child.name}", record.order,
                ))
                return False
        reason = run_probe(record, bags[record.header], bags)
        if reason:
            bags[record.header].skips.append(Skip(record.header, record.name, reason, record.order))
            return False
        record.confirmed = True
        done[key] = True
        return True

    for bag in bags.values():
        kept = []
        for record in bag.records:
            if confirm(record):
                kept.append(record)
        bag.records = kept


def gate(bags: dict[str, Bag]) -> None:
    bad = 0
    for bag in bags.values():
        for macro in bag.macros:
            if macro.kind == "str":
                continue
            if macro.clang != macro.value:
                bad += 1
        for const in bag.enum_consts:
            if const.clang != const.value:
                bad += 1
        for record in _iter_records(bag.records):
            if not record.confirmed or record.clang_size != record.size or record.clang_align != record.align:
                bad += 1
            for check in record.checks:
                if check.clang != check.offset:
                    bad += 1
    if bad:
        raise SystemExit(f"linux mismatches {bad}")


def _iter_records(records: list[Record]):
    seen: set[int] = set()

    def walk(record: Record):
        if id(record) in seen:
            return
        seen.add(id(record))
        if record.confirmed:
            yield record
        for child in record.children:
            yield from walk(child)

    for record in records:
        yield from walk(record)


def c_decl(field: EmitField) -> str:
    suffix = "".join(f"[{length}]" for length in field.arrays)
    return f"    {field.c_type} {field.name}{suffix};"


def rust_decl(field: EmitField) -> str:
    rust = field.rust_type
    for length in reversed(field.arrays):
        rust = f"[{rust}; {length}]"
    return f"    pub {rust_ident(field.name)}: {rust},"


def c_attr(record: Record) -> str:
    if record.form == "wrap":
        return f" __attribute__((packed, aligned({record.align_n})))"
    if record.form == "packed":
        return " __attribute__((packed))"
    if record.form == "align":
        return f" __attribute__((aligned({record.align_n})))"
    return ""


def emit_c_record(record: Record) -> list[str]:
    lines = [f"{record.kind} {record.name} {{"]
    for field in record.fields:
        lines.append(c_decl(field))
    lines.append("}" + c_attr(record) + ";")
    return lines


def emit_rust_record(record: Record) -> list[str]:
    lines: list[str] = []
    derive = "#[derive(Clone, Copy)]"
    if record.form == "wrap":
        lines.extend(["#[repr(C, packed)]", derive, f"pub {record.kind} {record.inner} {{"])
        for field in record.fields:
            lines.append(rust_decl(field))
        lines.append("}")
        lines.extend([
            f"#[repr(C, align({record.align_n}))]",
            derive,
            f"pub struct {record.name}(pub {record.inner});",
        ])
    else:
        attr = {
            "c": "#[repr(C)]",
            "packed": "#[repr(C, packed)]",
            "align": f"#[repr(C, align({record.align_n}))]",
        }[record.form]
        lines.extend([attr, derive, f"pub {record.kind} {record.name} {{"])
        for field in record.fields:
            lines.append(rust_decl(field))
        lines.append("}")
    if record.kind == "union":
        lines.insert(0, "#[allow(missing_debug_implementations)] // union fields share one store")
    if record.flex_rust:
        lines.append(f"pub const {record.name}_FLEX: usize = core::mem::size_of::<{record.flex_rust}>();")
    return lines


def mask_items(record: Record, taken: set[str]) -> list[tuple[str, int]]:
    items = []
    for name, mask, shift, _width in record.masks:
        base = f"{record.name}_{name}"
        for suffix, value in (("_MASK", mask), ("_SHIFT", shift)):
            item = base + suffix
            while item in taken:
                item += "_BIT"
            taken.add(item)
            items.append((item, int(value)))
    return items


def names_in(node: object) -> set[str]:
    found: set[str] = set()

    def walk(item: object) -> None:
        if isinstance(item, Name):
            found.add(item.name)
        elif isinstance(item, (Unary, Cast)):
            walk(item.expr)
        elif isinstance(item, Binary):
            walk(item.left)
            walk(item.right)
        elif isinstance(item, Ternary):
            walk(item.cond)
            walk(item.then)
            walk(item.else_)
        elif isinstance(item, Call):
            for arg in item.args:
                walk(arg)

    walk(node)
    return found


def param_aliases(params: list[str], expr: object) -> dict[str, str]:
    used = names_in(expr)
    aliases: dict[str, str] = {}
    seen: set[str] = set()
    for name in params:
        if name in used:
            rust = rust_ident(name.lstrip("_") or "arg")
        else:
            rust = rust_ident(name if name.startswith("_") else f"_{name}")
        base = rust
        index = 2
        while rust in seen:
            rust = f"{base}_{index}"
            index += 1
        seen.add(rust)
        aliases[name] = rust
    return aliases


def strip_wrapping_parens(text: str) -> str:
    if len(text) < 2 or text[0] != "(" or text[-1] != ")":
        return text
    depth = 0
    for index, char in enumerate(text):
        if char == "(":
            depth += 1
        elif char == ")":
            depth -= 1
            if depth == 0 and index != len(text) - 1:
                return text
    return text[1:-1]


def render_fn(macro: Macro) -> str:
    global PARAM_ALIASES
    params = macro.params or []
    aliases = param_aliases(params, macro.expr)
    PARAM_ALIASES = aliases
    body = strip_wrapping_parens(rust_expr(macro.expr))
    PARAM_ALIASES = {}
    signature = ", ".join(f"{aliases[name]}: u64" for name in params)
    name = rust_ident(macro.name)
    return f"#[must_use]\n#[rustfmt::skip]\npub const fn {name}({signature}) -> u64 {{ {body} }}\n"


def header_guard(rel: str) -> str:
    token = re.sub(r"[^A-Za-z0-9]", "_", rel).upper()
    return f"MT79_{token}"


def relative_include(source: str, target: str) -> str:
    start = Path(source).parent
    return Path(os.path.relpath(target, start)).as_posix()


def types_header() -> str:
    return f"""/* SPDX-License-Identifier: GPL-2.0-only */
/* {NOTE} */
#ifndef MT79_TYPES_H
#define MT79_TYPES_H
typedef unsigned char uint8_t;
typedef unsigned short uint16_t;
typedef unsigned int uint32_t;
typedef unsigned long long uint64_t;
typedef signed char int8_t;
typedef short int16_t;
typedef int int32_t;
typedef long long int64_t;
typedef uint8_t u8;
typedef uint16_t u16;
typedef uint32_t u32;
typedef uint64_t u64;
typedef int8_t s8;
typedef int16_t s16;
typedef int32_t s32;
typedef int64_t s64;
typedef uint8_t bool;
typedef uint16_t __le16;
typedef uint32_t __le32;
typedef uint64_t __le64;
typedef uint16_t __be16;
typedef uint32_t __be32;
typedef uint64_t __be64;
#define BIT(n) (1ULL << (n))
#define GENMASK(high, low) (((~0ULL) >> (63ULL - (high))) & ((~0ULL) << (low)))
#endif
"""


def endian_rust() -> str:
    parts = ["//! Little-endian and big-endian integer wrappers.", ""]
    specs = [
        ("Le16", "u16", "to_le", "from_le"),
        ("Le32", "u32", "to_le", "from_le"),
        ("Le64", "u64", "to_le", "from_le"),
        ("Be16", "u16", "to_be", "from_be"),
        ("Be32", "u32", "to_be", "from_be"),
        ("Be64", "u64", "to_be", "from_be"),
    ]
    for name, inner, to_wire, from_wire in specs:
        parts.append("#[repr(transparent)]")
        parts.append("#[derive(Clone, Copy, Debug, Eq, PartialEq)]")
        parts.append(f"pub struct {name}(pub {inner});")
        parts.append(f"impl {name} {{")
        parts.append("    #[must_use]")
        parts.append(f"    pub const fn from_native(value: {inner}) -> Self {{")
        parts.append(f"        Self(value.{to_wire}())")
        parts.append("    }")
        parts.append("")
        parts.append("    #[must_use]")
        parts.append(f"    pub const fn to_native(self) -> {inner} {{")
        parts.append(f"        {inner}::{from_wire}(self.0)")
        parts.append("    }")
        parts.append("}")
        parts.append("")
    return "\n".join(parts)


def bits_rust() -> str:
    return """//! Unsigned helpers used by transcribed function-like macros.
#![allow(clippy::cast_possible_truncation)] // shift counts are narrowed for wrapping shifts
#![allow(clippy::integer_division)] // unsigned division matches the C macros

#[must_use]
pub const fn bit(amount: u64) -> u64 {
    1u64.wrapping_shl(amount as u32)
}

#[must_use]
pub const fn genmask(high: u64, low: u64) -> u64 {
    let top = u64::MAX.wrapping_shr((63u64 - high) as u32);
    let bottom = u64::MAX.wrapping_shl(low as u32);
    top & bottom
}

#[must_use]
pub const fn shl(value: u64, amount: u64) -> u64 {
    value.wrapping_shl(amount as u32)
}

#[must_use]
pub const fn shr(value: u64, amount: u64) -> u64 {
    value.wrapping_shr(amount as u32)
}

/// Divides `left` by `right`.
///
/// # Panics
///
/// Panics when `right` is zero.
#[must_use]
pub const fn div(left: u64, right: u64) -> u64 {
    left / right
}

/// Remainder of `left` divided by `right`.
///
/// # Panics
///
/// Panics when `right` is zero.
#[must_use]
pub const fn rem(left: u64, right: u64) -> u64 {
    left % right
}
"""


def call_paths_for(bag: Bag) -> dict[str, str]:
    paths = {}
    for name, home in bag.fn_header.items():
        if home == bag.rel:
            paths[name] = rust_ident(name)
        else:
            paths[name] = f"crate::{module_path(home)}::{rust_ident(name)}"
    return paths


def emit_header_c(bag: Bag) -> str:
    includes = ["mt79_types.h"]
    for record in _iter_records(bag.records):
        for dep in record.dep_keys:
            if isinstance(dep, Record) and dep.header != bag.rel and dep.header not in includes:
                includes.append(dep.header)
    lines = [
        "/* SPDX-License-Identifier: GPL-2.0-only */",
        f"/* {NOTE} */",
        f"#ifndef {header_guard(bag.rel)}",
        f"#define {header_guard(bag.rel)}",
    ]
    for inc in includes:
        target = "mt79_types.h" if inc == "mt79_types.h" else inc
        lines.append(f'#include "{relative_include(bag.rel, target)}"')
    lines.append("")
    taken = {macro.name for macro in bag.macros} | {const.name for const in bag.enum_consts}
    emitted: set[int] = set()

    def emit_record(record: Record) -> None:
        if id(record) in emitted or not record.confirmed:
            return
        for dep in record.dep_keys:
            if isinstance(dep, Record) and dep.header == bag.rel:
                emit_record(dep)
        for child in record.children:
            emit_record(child)
        for name, value in mask_items(record, taken):
            lines.append(f"#define {name} {value}ull")
        lines.extend(emit_c_record(record))
        lines.append("")
        emitted.add(id(record))

    events: list[tuple[int, int, str, object]] = []
    for macro in bag.macros:
        events.append((macro.order, 0, "macro", macro))
    for const in bag.enum_consts:
        events.append((const.order, 1, "enum", const))
    for record in bag.records:
        events.append((record.order, 2, "record", record))
    for _order, _kind, kind, item in sorted(events, key=lambda part: (part[0], part[1])):
        if kind == "macro":
            assert isinstance(item, Macro)
            if item.kind == "str":
                lines.append(f"#define {item.name} {item.body}")
            elif item.kind == "fn":
                params = ", ".join(item.params or [])
                lines.append(f"#define {item.name}({params}) {c_expr(item.expr)}")
            else:
                lines.append(f"#define {item.name} {item.value}ull")
            continue
        if kind == "enum":
            assert isinstance(item, EnumConst)
            lines.append(f"#define {item.name} {item.value}ull")
            continue
        assert isinstance(item, Record)
        emit_record(item)
    lines.append(f"#endif /* {header_guard(bag.rel)} */")
    lines.append("")
    return "\n".join(lines)


def emit_header_rust(bag: Bag) -> str:
    global CALL_PATHS
    CALL_PATHS = call_paths_for(bag)
    lines = [
        f"//! `{bag.rel}` layouts reconstructed from Linux v7.3.5 mt76.",
        "#[allow(unused_imports)] // fields name endian wrappers directly",
        "use crate::endian::{Be16, Be32, Be64, Le16, Le32, Le64};",
        "#[allow(unused_imports)] // function-like macros call the unsigned helpers",
        "use crate::bits::{bit, div, genmask, rem, shl, shr};",
        "",
    ]
    taken = {macro.name for macro in bag.macros} | {const.name for const in bag.enum_consts}
    emitted: set[int] = set()

    def emit_record(record: Record) -> None:
        if id(record) in emitted or not record.confirmed:
            return
        for dep in record.dep_keys:
            if isinstance(dep, Record) and dep.header == bag.rel:
                emit_record(dep)
        for child in record.children:
            emit_record(child)
        for name, value in mask_items(record, taken):
            lines.append(f"pub const {rust_ident(name)}: u64 = {rust_int(value)};")
        lines.extend(emit_rust_record(record))
        lines.append("")
        emitted.add(id(record))

    events: list[tuple[int, int, str, object]] = []
    for macro in bag.macros:
        events.append((macro.order, 0, "macro", macro))
    for const in bag.enum_consts:
        events.append((const.order, 1, "enum", const))
    for record in bag.records:
        events.append((record.order, 2, "record", record))
    for _order, _kind, kind, item in sorted(events, key=lambda part: (part[0], part[1])):
        if kind == "macro":
            assert isinstance(item, Macro)
            if item.kind == "str" and item.text is not None:
                lines.append(f"pub const {rust_ident(item.name)}: &[u8] = {rust_bytes(item.text)};")
            elif item.kind == "fn":
                lines.append(render_fn(item))
            else:
                lines.append(f"pub const {rust_ident(item.name)}: u64 = {rust_int(item.value or 0)};")
            continue
        if kind == "enum":
            assert isinstance(item, EnumConst)
            lines.append(f"pub const {rust_ident(item.name)}: u64 = {rust_int(item.value)};")
            continue
        assert isinstance(item, Record)
        emit_record(item)
    return "\n".join(lines) + "\n"


def rust_raw(text: str) -> str:
    marks = 0
    while f'"{"#" * marks}' in text:
        marks += 1
    pad = "#" * marks
    return f'r{pad}"{text}"{pad}'


def group_source(bag: Bag) -> tuple[str, str]:
    """C probe source and the matching Rust rows for one header."""
    c_lines = [
        f'#include "{bag.rel}"',
        "#include <stdio.h>",
        "#include <stddef.h>",
        "int main(void) {",
    ]
    rust_ints: list[str] = []
    rust_strs: list[str] = []
    rust_recs: list[str] = []
    string_lines: list[str] = []
    record_lines: list[str] = []
    prefix = f"crate::{module_path(bag.rel)}"
    for macro in bag.macros:
        if macro.kind == "str":
            string_lines.append(f'printf("S %zu ", sizeof({macro.name}) - 1);')
            string_lines.append(f"fwrite({macro.name}, 1, sizeof({macro.name}) - 1, stdout);")
            string_lines.append(r'printf("\n");')
            rust_strs.append(f"        {prefix}::{rust_ident(macro.name)},")
            continue
        if macro.kind == "fn":
            args = ", ".join(str(part) for part in (macro.sample or ()))
            expr = f"{macro.name}({args})"
            rust_call = f"{prefix}::{rust_ident(macro.name)}({args})"
        else:
            expr = macro.name
            rust_call = f"{prefix}::{rust_ident(macro.name)}"
        c_lines.append(f'printf("I %s %llu\\n", "{macro.name}", (unsigned long long)({expr}));')
        rust_ints.append(f'        IntRow {{ name: "{macro.name}", value: {rust_call} }},')
    for const in bag.enum_consts:
        c_lines.append(f'printf("I %s %llu\\n", "{const.name}", (unsigned long long)({const.name}));')
        rust_ints.append(
            f'        IntRow {{ name: "{const.name}", value: {prefix}::{rust_ident(const.name)} }},'
        )
    taken = {macro.name for macro in bag.macros} | {const.name for const in bag.enum_consts}
    for record in _iter_records(bag.records):
        for name, _value in mask_items(record, taken):
            c_lines.append(f'printf("I %s %llu\\n", "{name}", (unsigned long long)({name}));')
            rust_ints.append(f'        IntRow {{ name: "{name}", value: {prefix}::{rust_ident(name)} }},')
        qualifier = f"{record.kind} {record.name}"
        record_lines.append(f'printf("R %zu %zu\\n", sizeof({qualifier}), _Alignof({qualifier}));')
        fields = []
        rust_name = f"{prefix}::{record.name}"
        for field in record.fields:
            record_lines.append(f'printf("O %zu\\n", offsetof({qualifier}, {field.name}));')
            path = f"0.{rust_ident(field.name)}" if record.form == "wrap" else rust_ident(field.name)
            fields.append(f"            FieldRow {{ offset: core::mem::offset_of!({rust_name}, {path}) }},")
        field_body = "\n".join(fields)
        rust_recs.append(
            "        RecRow {\n"
            f"            size: core::mem::size_of::<{rust_name}>(),\n"
            f"            align: core::mem::align_of::<{rust_name}>(),\n"
            f"            fields: &[\n{field_body}\n            ],\n"
            "        },"
        )
    c_lines.extend(string_lines)
    c_lines.extend(record_lines)
    c_lines.append("return 0; }")
    rust = "\n".join([
        "    Group {",
        f"        source: {rust_raw(chr(10).join(c_lines))},",
        "        ints: &[",
        *rust_ints,
        "        ],",
        "        strs: &[",
        *rust_strs,
        "        ],",
        "        recs: &[",
        *rust_recs,
        "        ],",
        "    },",
    ])
    return "\n".join(c_lines), rust


def prove_rust(groups: list[str]) -> str:
    body = "\n".join(groups)
    template = """//! The generated C headers and the Rust layouts are one ABI.
struct IntRow {{
    name: &'static str,
    value: u64,
}}

struct FieldRow {{
    offset: usize,
}}

struct RecRow {{
    size: usize,
    align: usize,
    fields: &'static [FieldRow],
}}

struct Group {{
    source: &'static str,
    ints: &'static [IntRow],
    strs: &'static [&'static [u8]],
    recs: &'static [RecRow],
}}

static GROUPS: &[Group] = &[
@@BODY@@
];

#[test]
fn generated_c_matches_rust() {{
    for group in GROUPS {{
        check_group(group);
    }}
}}

fn check_group(group: &Group) {{
    let output = compile(group.source);
    let lines: Vec<&str> = output.lines().collect();
    let mut cursor = 0;
    cursor = check_ints(group, &lines, cursor);
    cursor = check_strs(group, &lines, cursor);
    check_recs(group, &lines, cursor);
}}

fn check_ints(group: &Group, lines: &[&str], mut cursor: usize) -> usize {{
    for row in group.ints {{
        let parts: Vec<&str> = lines[cursor].split_whitespace().collect();
        assert_eq!(parts.first().copied(), Some("I"), "{{}}", row.name);
        let got: u64 = parts[2].parse().expect("integer");
        assert_eq!(got, row.value, "{{}}", row.name);
        cursor += 1;
    }}
    cursor
}}

fn check_strs(group: &Group, lines: &[&str], mut cursor: usize) -> usize {{
    for row in group.strs {{
        let line = lines[cursor];
        let rest = line.strip_prefix("S ").expect("string row");
        let (len_text, text) = rest.split_once(' ').unwrap_or((rest, ""));
        let len: usize = len_text.parse().expect("length");
        assert_eq!(len, row.len(), "string length");
        assert_eq!(text.as_bytes(), *row, "string bytes");
        cursor += 1;
    }}
    cursor
}}

fn check_recs(group: &Group, lines: &[&str], mut cursor: usize) {{
    for row in group.recs {{
        let parts: Vec<&str> = lines[cursor].split_whitespace().collect();
        cursor += 1;
        assert_eq!(parts.first().copied(), Some("R"));
        let size: usize = parts[1].parse().expect("size");
        let align: usize = parts[2].parse().expect("align");
        assert_eq!(size, row.size, "record size");
        assert_eq!(align, row.align, "record align");
        for field in row.fields {{
            let got: usize = lines[cursor].split_whitespace().nth(1).expect("offset").parse().expect("offset");
            assert_eq!(got, field.offset, "field offset");
            cursor += 1;
        }}
    }}
}}

fn compile(source: &str) -> String {{
    let dir = std::env::temp_dir().join("lerux-mt79-proof");
    std::fs::create_dir_all(&dir).expect("temp dir");
    let path = dir.join("probe.c");
    std::fs::write(&path, source).expect("probe source");
    let binary = dir.join("probe");
    let include = concat!(env!("CARGO_MANIFEST_DIR"), "/include");
    let compiled = std::process::Command::new("gcc")
        .args(["-std=gnu11", "-I", include])
        .arg(&path)
        .arg("-o")
        .arg(&binary)
        .output()
        .expect("gcc");
    assert!(
        compiled.status.success(),
        "{{}}",
        String::from_utf8_lossy(&compiled.stderr),
    );
    let ran = std::process::Command::new(&binary).output().expect("probe");
    assert!(ran.status.success(), "{{}}", String::from_utf8_lossy(&ran.stderr));
    String::from_utf8(ran.stdout).expect("utf8")
}}
"""
    return template.replace("{{", "{").replace("}}", "}").replace("@@BODY@@", body)


def lib_rust(rels: list[str], names: list[tuple[str, str]]) -> str:
    lines = [
        "#![cfg_attr(not(test), no_std)]",
        "//! Standalone mt76 layouts reconstructed from Linux v7.3.5.",
        "//! The MIT Swift driver does not link this crate.",
        "",
        "#![allow(non_camel_case_types)] // C record names keep their kernel spelling",
        "#![allow(non_snake_case)] // C fields and function-like macros keep their spelling",
        "#![allow(non_upper_case_globals)] // C macros keep their spelling",
        "#![allow(missing_debug_implementations)] // wire records are copied, not formatted",
        "#![allow(clippy::module_inception)] // the chip header repeats its directory name",
        "#![allow(clippy::pub_underscore_fields)] // kernel fields keep a leading underscore",
        "",
        "mod bits;",
        "mod endian;",
        "pub use bits::{bit, div, genmask, rem, shl, shr};",
        "pub use endian::{Be16, Be32, Be64, Le16, Le32, Le64};",
        "",
    ]
    tops = []
    for rel in rels:
        top = Path(rel).parts[0].removesuffix(".h")
        if top not in tops:
            tops.append(top)
    for top in tops:
        lines.append(f"pub mod {top};")
    lines.append("")
    reserved = {
        "bit", "div", "genmask", "rem", "shl", "shr",
        "Be16", "Be32", "Be64", "Le16", "Le32", "Le64",
    }
    counts: dict[str, int] = {}
    for name, _path in names:
        counts[name] = counts.get(name, 0) + 1
    for name, path in names:
        if counts[name] == 1 and name not in reserved:
            lines.append(f"pub use {path}::{rust_ident(name)};")
    lines.append("")
    lines.append("#[cfg(test)]")
    lines.append("mod prove;")
    lines.append("")
    return "\n".join(lines)


def mod_rust(directory: str, rels: list[str]) -> str:
    names = []
    for rel in rels:
        parts = Path(rel).parts
        if len(parts) == 2 and parts[0] == directory:
            names.append(Path(parts[1]).stem)
    lines = [f"//! `{directory}` headers reconstructed from Linux v7.3.5 mt76.", ""]
    for name in sorted(set(names)):
        lines.append(f"pub mod {name};")
    lines.append("")
    return "\n".join(lines)


def collect_names(bag: Bag) -> list[tuple[str, str]]:
    path = module_path(bag.rel)
    found = []
    for macro in bag.macros:
        found.append((macro.name, path))
    for const in bag.enum_consts:
        found.append((const.name, path))
    taken = {name for name, _path in found}
    for record in _iter_records(bag.records):
        found.append((record.name, path))
        if record.inner:
            found.append((record.inner, path))
        for name, _value in mask_items(record, taken):
            found.append((name, path))
        if record.flex_rust:
            found.append((f"{record.name}_FLEX", path))
    return found


def write_outputs(files: dict[Path, str]) -> None:
    expected = set()
    for path, text in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        temporary = path.with_name(path.name + ".tmp")
        temporary.write_text(text)
        temporary.replace(path)
        expected.add(path.resolve())
    for root in (INCLUDE, SRC):
        if not root.exists():
            continue
        for path in root.rglob("*"):
            if path.is_file() and path.suffix in {".h", ".rs"} and path.resolve() not in expected:
                path.unlink()


def proof_text(bags: dict[str, Bag]) -> str:
    macros = sum(1 for bag in bags.values() for macro in bag.macros)
    enums = sum(len(bag.enum_consts) for bag in bags.values())
    records = sum(1 for bag in bags.values() for _rec in _iter_records(bag.records))
    skips = [skip for bag in bags.values() for skip in bag.skips]
    lines = [
        f"linux tag {TAG}",
        f"macros emitted {macros}",
        f"enum constants emitted {enums}",
        f"records emitted {records}",
        f"skipped {len(skips)}",
        "linux mismatches 0",
    ]
    for skip in sorted(skips, key=lambda item: (item.header, item.order, item.name, item.reason)):
        lines.append(f"skipped {skip.header} {skip.name} {skip.reason}")
    lines.append("")
    return "\n".join(lines)


def selected_headers(root: Path, only: list[str] | None) -> list[str]:
    found = sorted(path.relative_to(root).as_posix() for path in root.rglob("*.h"))
    if not only:
        return found
    missing = [name for name in only if name not in found]
    if missing:
        raise SystemExit(f"missing headers: {missing}")
    return list(only)


def generate(root: Path, rels: list[str]) -> None:
    selected = set(rels)
    bags = {rel: load_header(root, rel, selected) for rel in rels}
    for rel in closure_order(rels, bags):
        print(f"bind {rel}", flush=True)
        bind_header(bags[rel], bags)
        accept_constants(bags[rel])
    print("resolve records", flush=True)
    resolved = resolve_records(bags)
    print(f"probe {len(resolved)} records", flush=True)
    confirm_records(bags, resolved)
    gate(bags)
    if "dma.h" in bags:
        names = {record.name for record in _iter_records(bags["dma.h"].records)}
        if "mt76_desc" not in names:
            raise SystemExit("mt76_desc was not emitted")
    files: dict[Path, str] = {}
    files[INCLUDE / "mt79_types.h"] = types_header()
    files[SRC / "endian.rs"] = endian_rust()
    files[SRC / "bits.rs"] = bits_rust()
    groups = []
    names: list[tuple[str, str]] = []
    for rel in rels:
        bag = bags[rel]
        files[INCLUDE / rel] = emit_header_c(bag)
        files[SRC / Path(rel).with_suffix(".rs")] = emit_header_rust(bag)
        _c_source, rust_group = group_source(bag)
        if bag.macros or bag.enum_consts or bag.records:
            groups.append(rust_group)
        names.extend(collect_names(bag))
    directories = sorted({Path(rel).parts[0] for rel in rels if len(Path(rel).parts) == 2})
    for directory in directories:
        files[SRC / directory / "mod.rs"] = mod_rust(directory, rels)
    files[SRC / "lib.rs"] = lib_rust(rels, names)
    files[SRC / "prove.rs"] = prove_rust(groups)
    files[REPO / "support/mt79/proof.txt"] = proof_text(bags)
    write_outputs(files)
    formatted = subprocess.run(
        ["cargo", "fmt", "-p", "lerux-mt79"],
        cwd=REPO,
        check=False,
    )
    if formatted.returncode != 0:
        raise SystemExit("rustfmt failed")
    proof = (REPO / "support/mt79/proof.txt").read_text().splitlines()
    print("\n".join(proof[:6]), flush=True)


def closure_order(rels: list[str], bags: dict[str, Bag]) -> list[str]:
    seen: list[str] = []
    walking: set[str] = set()

    def visit(rel: str) -> None:
        if rel in seen or rel in walking:
            return
        walking.add(rel)
        for inc in bags[rel].includes:
            visit(inc)
        walking.remove(rel)
        seen.append(rel)

    for rel in rels:
        visit(rel)
    return seen


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--kernel", type=Path, default=KERNEL_DEFAULT)
    parser.add_argument("--headers", nargs="*")
    args = parser.parse_args()
    self_check()
    generate(args.kernel, selected_headers(args.kernel, args.headers))


if __name__ == "__main__":
    main()
