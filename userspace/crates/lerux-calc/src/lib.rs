//! Integer expressions shared by the fetch `calc` command and the maths session.
//!
//! A sum is products separated by `+` and `-`. A product is factors separated
//! by `*` and `/`. A factor is a number, a parenthesised sum, or a factor with
//! a leading `+` or `-`. Spaces are ignored. `+`, `-`, and `*` saturate.
//! Division truncates toward zero and fails when the divisor is zero or the
//! quotient does not fit in `i64`. A trailing character that is not a space fails.
#![cfg_attr(not(test), no_std)]

// The verifier's expansion names `vstd` and `int`. A normal build erases that
// expansion, so the imports are unused unless `verus` is verifying.
// Protection-domain builds leave the feature off.
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use verus_builtin::*;
use verus_builtin_macros::verus;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::arithmetic::div_mod::{lemma_div_pos_is_pos, lemma_fundamental_div_mod, lemma_mod_bound};
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::arithmetic::mul::{
    group_mul_basics, group_mul_is_distributive, lemma_mul_inequality, lemma_mul_is_associative,
    lemma_mul_is_commutative, lemma_mul_is_distributive_add,
    lemma_mul_is_distributive_add_other_way,
};
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::arithmetic::power::{lemma_pow0, lemma_pow1, lemma_pow_adds, pow};
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::arithmetic::power2::pow2;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::layout::signed_int_min_max_values;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::math::abs;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::prelude::*;
#[cfg(feature = "verus")]
#[allow(unused_imports)]
use vstd::slice::spec_slice_len;

/// The input is empty, unbalanced, or otherwise not this grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Error;

/// Evaluate one expression.
///
/// # Errors
///
/// Returns [`Error`] when the input is empty, unbalanced, has a trailing
/// character, divides by zero, or the quotient does not fit in `i64`.
pub fn eval(input: &[u8]) -> Result<i64, Error> {
    let mut parser = Parser { s: input, i: 0 };
    let value = parser.parse_expr()?;
    parser.skip();
    if parser.i != parser.s.len() {
        return Err(Error);
    }
    Ok(value)
}

verus! {

/// Bytes a `u64` decimal needs. `i64::unsigned_abs` fits in the same width.
const DECIMAL_CAP: usize = 20;

/// `byte` is an ASCII decimal digit.
pub open spec fn is_ascii_digit(byte: u8) -> bool {
    b'0' <= byte && byte <= b'9'
}

/// Integer value of an ASCII decimal digit.
pub open spec fn digit_value(byte: u8) -> int {
    (byte as int) - (b'0' as int)
}

/// Base-10 value of `digits`, least-significant digit last. Empty is 0.
pub open spec fn parse_digits(digits: Seq<u8>) -> int
    decreases digits.len(),
{
    if digits.len() == 0 {
        0
    } else {
        parse_digits(digits.drop_last()) * 10 + digit_value(digits.last())
    }
}

/// Canonical ASCII decimal: digits only, and no leading zero unless the
/// sequence is the single digit zero.
pub open spec fn decimal_digits(digits: Seq<u8>) -> bool {
    &&& digits.len() > 0
    &&& (digits.len() == 1 || digits[0] != b'0')
    &&& forall|i: int| 0 <= i < digits.len() ==> is_ascii_digit(#[trigger] digits[i])
}

/// Mathematical value of `i64::unsigned_abs`.
///
/// Negating `i64::MIN` overflows `i64`. The absolute value is `2^63`, which
/// fits in `u64` and is what `unsigned_abs` returns.
pub open spec fn spec_unsigned_abs(value: i64) -> int {
    if value < 0 {
        -(value as int)
    } else {
        value as int
    }
}

/// `remaining` still fits in `index` decimal places, including a zero quotient.
spec fn below_pow(remaining: int, index: int) -> bool {
    0 <= remaining < pow(10, index as nat)
}

/// Value reconstructed from the digits already stored at the low end of `tmp`.
spec fn place_holds(value: int, remaining: int, index: int, tmp: Seq<u8>) -> bool {
    pow(10, (DECIMAL_CAP as int - index) as nat) * remaining + parse_digits(
        tmp[index..DECIMAL_CAP as int],
    ) == value
}

/// Bytes of `tmp` from `index` on are ASCII digits.
spec fn suffix_is_digits(index: int, tmp: Seq<u8>) -> bool {
    forall|j: int| index <= j < DECIMAL_CAP as int ==> is_ascii_digit(#[trigger] tmp[j])
}

/// The highest stored digit is not a leading zero, unless it is the only digit.
spec fn no_leading_zero(index: int, remaining: int, tmp: Seq<u8>) -> bool {
    index < DECIMAL_CAP as int ==> (DECIMAL_CAP as int - index == 1 || remaining > 0 || tmp[index]
        != b'0')
}

/// `10^(exponent + 1) == 10^exponent * 10`.
proof fn lemma_pow10_succ(exponent: nat)
    ensures
        pow(10, (exponent + 1) as nat) == pow(10, exponent) * 10,
{
    lemma_pow_adds(10, exponent, 1);
    lemma_pow1(10);
}

/// Dividing a value below `10^exponent` by ten drops one power of ten.
proof fn lemma_quot_shrinks(value: int, exponent: nat)
    requires
        exponent >= 1,
        0 <= value < pow(10, exponent),
    ensures
        0 <= value / 10 < pow(10, (exponent - 1) as nat),
{
    lemma_pow0(10);
    lemma_pow10_succ((exponent - 1) as nat);
    lemma_div_pos_is_pos(value, 10);
    lemma_fundamental_div_mod(value, 10);
    lemma_mod_bound(value, 10);
    let quot = value / 10;
    let scale = pow(10, (exponent - 1) as nat);
    assert(pow(10, exponent) == scale * 10);
    lemma_mul_is_commutative(10, quot);
    assert(value == quot * 10 + (value % 10));
    assert(value % 10 >= 0);
    assert(value >= quot * 10);
    if quot >= scale {
        lemma_mul_inequality(scale, quot, 10);
        assert(scale * 10 <= quot * 10);
        assert(value >= scale * 10);
        assert(false);
    }
    assert(quot < scale);
}

/// Prepending a higher digit multiplies the suffix by ten.
proof fn lemma_parse_cons(digit: u8, rest: Seq<u8>)
    ensures
        parse_digits(seq![digit].add(rest)) == digit_value(digit) * pow(10, rest.len() as nat)
            + parse_digits(rest),
    decreases rest.len(),
{
    let combined = seq![digit].add(rest);
    broadcast use group_mul_basics;

    if rest.len() == 0 {
        lemma_pow0(10);
        assert(combined.len() == 1);
        assert(combined[0] == digit);
        assert(combined.last() == combined[0]);
        assert(combined.drop_last().len() == 0);
        assert(parse_digits(combined.drop_last()) == 0);
        assert(parse_digits(combined) == 0 * 10 + digit_value(combined.last()));
        assert(0 * 10 == 0);
        assert(parse_digits(combined) == digit_value(digit));
        assert(pow(10, 0) == 1);
        assert(digit_value(digit) * pow(10, 0) == digit_value(digit));
    } else {
        Seq::drop_last_distributes_over_add(seq![digit], rest);
        assert(combined.drop_last() == seq![digit].add(rest.drop_last()));
        assert(combined.last() == rest.last());
        lemma_parse_cons(digit, rest.drop_last());
        lemma_pow10_succ((rest.len() - 1) as nat);
        lemma_mul_is_associative(digit_value(digit), pow(10, (rest.len() - 1) as nat), 10);
        lemma_mul_is_commutative(pow(10, (rest.len() - 1) as nat), 10);
        assert(parse_digits(combined) == parse_digits(combined.drop_last()) * 10 + digit_value(
            combined.last(),
        ));
        assert(parse_digits(combined) == digit_value(digit) * pow(10, rest.len() as nat)
            + parse_digits(rest));
    }
}

/// Every `u64` is strictly below `10^20`, so its decimal fits in [`DECIMAL_CAP`].
proof fn lemma_u64_lt_pow10_20(value: u64)
    ensures
        (value as int) < pow(10, DECIMAL_CAP as nat),
{
    reveal(pow);
    assert(pow(10, 20) == 100_000_000_000_000_000_000) by (compute);
    assert(DECIMAL_CAP == 20);
    assert((u64::MAX as int) < 100_000_000_000_000_000_000);
    assert((value as int) <= (u64::MAX as int));
}

/// `1 << 63` is `i64::MIN.unsigned_abs()`.
fn magnitude_of_min() -> (magnitude: u64)
    ensures
        magnitude as int == spec_unsigned_abs(i64::MIN),
{
    let magnitude = 1u64 << 63u64;
    proof {
        signed_int_min_max_values();
        reveal(pow);
        assert(0x8_000_000_000_000_000 == pow2(63)) by (compute);
        assert((1u64 << 63u64) == 0x8_000_000_000_000_000) by (bit_vector);
        assert(magnitude == 1u64 << 63u64);
        assert(i64::BITS == 64);
        assert(abs(i64::MIN as int) == pow2((i64::BITS - 1) as nat));
        assert(spec_unsigned_abs(i64::MIN) == abs(i64::MIN as int) as int);
        assert(magnitude as int == pow2(63) as int);
    }
    magnitude
}

/// Absolute value of a negative `i64` other than [`i64::MIN`].
fn magnitude_of_negative(value: i64) -> (magnitude: u64)
    requires
        value < 0,
        value != i64::MIN,
    ensures
        magnitude as int == spec_unsigned_abs(value),
{
    let neg = -value;
    proof {
        assert((-value) as int == -(value as int)) by (bit_vector)
            requires
                value != i64::MIN,
                value < 0,
        ;
        assert(neg as int == -(value as int));
        assert(0 < neg <= i64::MAX);
        assert(neg as u64 as int == neg as int) by (bit_vector)
            requires
                0 <= neg,
        ;
        assert(spec_unsigned_abs(value) == -(value as int));
    }
    // The cast preserves the value: `neg` is positive and at most `i64::MAX`.
    #[expect(
        clippy::cast_sign_loss,
        reason = "negation of a negative i64 other than MIN fits in u64"
    )]
    let magnitude = neg as u64;
    magnitude
}

/// `i64::unsigned_abs` for a negative value, including [`i64::MIN`].
fn unsigned_magnitude(value: i64) -> (magnitude: u64)
    requires
        value < 0,
    ensures
        magnitude as int == spec_unsigned_abs(value),
{
    if value == i64::MIN {
        magnitude_of_min()
    } else {
        magnitude_of_negative(value)
    }
}

/// Quotient and the ASCII digit of `value % 10`.
fn quot_rem_10(value: u64) -> (out: (u64, u8))
    ensures
        out.0 as int == (value as int) / 10,
        digit_value(out.1) == (value as int) % 10,
        is_ascii_digit(out.1),
{
    let quot = value / 10;
    let rem = value % 10;
    proof {
        assert(value % 10 < 10) by (bit_vector);
        assert((value / 10) as int == (value as int) / 10) by (bit_vector);
        assert((value % 10) as int == (value as int) % 10) by (bit_vector);
        assert(quot as int == (value as int) / 10);
        assert(rem as int == (value as int) % 10);
    }
    let rem_byte = rem as u8;
    proof {
        assert(rem < 256);
        assert(rem_byte as int == rem as int);
        assert(rem_byte <= 9);
        assert((b'0' as int) + (rem_byte as int) < 256) by (bit_vector)
            requires
                rem_byte <= 9,
        ;
    }
    let byte = b'0' + rem_byte;
    proof {
        assert(byte as int == (b'0' as int) + (rem_byte as int));
        assert(digit_value(byte) == rem_byte as int);
        assert(is_ascii_digit(byte));
    }
    (quot, byte)
}

/// One digit step: the new high digit extends the suffix, and the quotient
/// shrinks by one power of ten.
#[verifier::rlimit(25)]
proof fn lemma_write_one(value: int, remaining: int, index: int, tmp: Seq<u8>, byte: u8, next: int)
    requires
        tmp.len() == DECIMAL_CAP,
        0 <= index <= DECIMAL_CAP,
        below_pow(remaining, index),
        remaining > 0 || index == DECIMAL_CAP as int,
        index >= 1 || remaining == 0,
        place_holds(value, remaining, index, tmp),
        suffix_is_digits(index, tmp),
        no_leading_zero(index, remaining, tmp),
        is_ascii_digit(byte),
        digit_value(byte) == remaining % 10,
        next == remaining / 10,
    ensures
        index >= 1,
        index == 1 ==> next == 0,
        below_pow(next, index - 1),
        place_holds(value, next, index - 1, tmp.update(index - 1, byte)),
        suffix_is_digits(index - 1, tmp.update(index - 1, byte)),
        no_leading_zero(index - 1, next, tmp.update(index - 1, byte)),
{
    if remaining == 0 {
        assert(index == DECIMAL_CAP as int);
    }
    assert(index >= 1);
    let place = index - 1;
    let updated = tmp.update(place, byte);
    let old_suffix = tmp[index..DECIMAL_CAP as int];
    let new_suffix = updated[place..DECIMAL_CAP as int];
    let prefixed = seq![byte].add(old_suffix);

    lemma_quot_shrinks(remaining, index as nat);
    lemma_pow0(10);
    assert((place as nat) == (index as nat - 1) as nat);
    assert(below_pow(next, place));
    if index == 1 {
        assert(pow(10, 0) == 1);
        assert(next == 0);
    }
    assert(updated[place] == byte);
    assert forall|i: int| 0 <= i < new_suffix.len() implies #[trigger] new_suffix[i]
        == prefixed[i] by {
        if i == 0 {
            assert(new_suffix[0] == byte);
            assert(prefixed[0] == byte);
        } else {
            assert(place + i != place);
            assert(updated[place + i] == tmp[place + i]);
            assert(new_suffix[i] == updated[place + i]);
            assert(old_suffix[i - 1] == tmp[index + (i - 1)]);
            assert(index + (i - 1) == place + i);
            assert(prefixed[i] == old_suffix[i - 1]);
        }
    };
    assert(new_suffix =~= prefixed);

    let written = (DECIMAL_CAP as int - index) as nat;
    assert(old_suffix.len() == written);
    lemma_parse_cons(byte, old_suffix);
    assert(parse_digits(new_suffix) == digit_value(byte) * pow(10, written) + parse_digits(
        old_suffix,
    ));
    lemma_fundamental_div_mod(remaining, 10);
    lemma_mod_bound(remaining, 10);
    assert(remaining == 10 * next + digit_value(byte));
    lemma_pow10_succ(written);
    let scale = pow(10, written);
    let digit = digit_value(byte);
    lemma_mul_is_distributive_add(scale, 10 * next, digit);
    lemma_mul_is_associative(scale, 10, next);
    lemma_mul_is_commutative(digit, scale);
    lemma_mul_is_commutative(scale, 10);
    assert(scale * remaining == scale * (10 * next + digit));
    assert(scale * (10 * next + digit) == scale * (10 * next) + scale * digit);
    assert(scale * (10 * next) == (scale * 10) * next);
    assert(scale * 10 == pow(10, (written + 1) as nat));
    assert(pow(10, (written + 1) as nat) * next + digit * scale + parse_digits(old_suffix) == scale
        * remaining + parse_digits(old_suffix));
    assert(pow(10, (written + 1) as nat) * next + parse_digits(new_suffix) == scale * remaining
        + parse_digits(old_suffix));
    assert(place_holds(value, remaining, index, tmp));
    assert(scale * remaining + parse_digits(old_suffix) == value);
    assert(pow(10, (written + 1) as nat) * next + parse_digits(new_suffix) == value);
    assert((DECIMAL_CAP as int - place) as nat == (written + 1) as nat);
    assert(place_holds(value, next, place, updated));

    assert forall|j: int| place <= j < DECIMAL_CAP as int implies #[trigger] is_ascii_digit(
        updated[j],
    ) by {
        if j == place {
            assert(updated[j] == byte);
        } else {
            assert(updated[j] == tmp[j]);
            assert(index <= j);
        }
    };
    assert(suffix_is_digits(place, updated));

    if index < DECIMAL_CAP as int && next == 0 {
        assert(remaining > 0);
        assert(remaining == digit);
        assert(0 <= digit < 10);
        assert(digit > 0);
        assert(byte != b'0');
    }
    if index == DECIMAL_CAP as int {
        assert(DECIMAL_CAP as int - place == 1);
    }
    assert(DECIMAL_CAP as int - place == 1 || next > 0 || byte != b'0');
    assert(updated[place] == byte);
    assert(no_leading_zero(place, next, updated));
}

/// After the store, the loop invariant holds at the new index.
proof fn lemma_stored(
    value: int,
    snap: Seq<u8>,
    idx: int,
    index: int,
    byte: u8,
    next: int,
    remaining: int,
    tmp: Seq<u8>,
)
    requires
        index == idx - 1,
        remaining == next,
        snap.len() == DECIMAL_CAP,
        0 <= index < snap.len(),
        tmp == snap.update(index, byte),
        idx >= 1,
        below_pow(next, idx - 1),
        place_holds(value, next, idx - 1, snap.update(idx - 1, byte)),
        suffix_is_digits(idx - 1, snap.update(idx - 1, byte)),
        no_leading_zero(idx - 1, next, snap.update(idx - 1, byte)),
        idx == 1 ==> next == 0,
    ensures
        decimal_inv(value, remaining, index, tmp),
{
    assert(tmp == snap.update(idx - 1, byte));
    if idx == 1 {
        assert(remaining == 0);
    } else {
        assert(index >= 1);
    }
    assert(below_pow(remaining, index));
    assert(place_holds(value, remaining, index, tmp));
    assert(suffix_is_digits(index, tmp));
    assert(no_leading_zero(index, remaining, tmp));
    assert(decimal_inv(value, remaining, index, tmp));
}

/// The empty high end is the whole value, and every `u64` fits in 20 digits.
proof fn lemma_fill_start(value: u64, tmp: Seq<u8>)
    requires
        tmp.len() == DECIMAL_CAP,
    ensures
        decimal_inv(value as int, value as int, DECIMAL_CAP as int, tmp),
{
    lemma_pow0(10);
    lemma_u64_lt_pow10_20(value);
    broadcast use group_mul_basics;

    assert(0 <= (value as int));
    assert(below_pow(value as int, DECIMAL_CAP as int));
    let empty = tmp[DECIMAL_CAP as int..DECIMAL_CAP as int];
    assert(empty.len() == 0);
    assert(parse_digits(empty) == 0);
    assert(pow(10, 0) * (value as int) == value as int);
    assert(place_holds(value as int, value as int, DECIMAL_CAP as int, tmp));
    assert(suffix_is_digits(DECIMAL_CAP as int, tmp));
    assert(no_leading_zero(DECIMAL_CAP as int, value as int, tmp));
    assert(decimal_inv(value as int, value as int, DECIMAL_CAP as int, tmp));
}

/// Once the quotient is zero, the suffix is the canonical decimal of `value`.
proof fn lemma_fill_done(value: u64, index: int, remaining: int, tmp: Seq<u8>)
    requires
        tmp.len() == DECIMAL_CAP,
        0 <= index < DECIMAL_CAP as int,
        remaining == 0,
        place_holds(value as int, remaining, index, tmp),
        suffix_is_digits(index, tmp),
        no_leading_zero(index, remaining, tmp),
    ensures
        decimal_digits(tmp[index..DECIMAL_CAP as int]),
        parse_digits(tmp[index..DECIMAL_CAP as int]) == value as int,
{
    broadcast use group_mul_basics;

    let suffix = tmp[index..DECIMAL_CAP as int];
    let scale = pow(10, (DECIMAL_CAP as int - index) as nat);
    assert(scale * remaining + parse_digits(suffix) == value as int);
    assert(scale * 0 == 0);
    assert(parse_digits(suffix) == value as int);
    assert(suffix.len() == DECIMAL_CAP as int - index);
    assert(suffix.len() >= 1);
    assert(suffix[0] == tmp[index]);
    assert(suffix.len() == 1 || suffix[0] != b'0');
    assert(decimal_digits(suffix));
}

/// One stored digit keeps the prefix, extends the copied run, and leaves the tail.
proof fn lemma_copy_step(
    prev: Seq<u8>,
    dst: Seq<u8>,
    original: Seq<u8>,
    src: Seq<u8>,
    at: int,
    start: int,
    before: int,
    done: int,
)
    requires
        dst == prev.update(at + before, src[start + before]),
        done == before + 1,
        0 <= before < src.len() - start,
        0 <= at,
        at + before < prev.len(),
        dst.len() == prev.len() == original.len(),
        src.len() == DECIMAL_CAP,
        forall|k: int| 0 <= k < at ==> #[trigger] prev[k] == original[k],
        forall|k: int| 0 <= k < before ==> #[trigger] prev[at + k] == src[start + k],
        forall|k: int| at + before <= k < prev.len() ==> #[trigger] prev[k] == original[k],
    ensures
        forall|k: int| 0 <= k < at ==> #[trigger] dst[k] == original[k],
        forall|k: int| 0 <= k < done ==> #[trigger] dst[at + k] == src[start + k],
        forall|k: int| at + done <= k < dst.len() ==> #[trigger] dst[k] == original[k],
{
    assert forall|k: int| 0 <= k < at implies #[trigger] dst[k] == original[k] by {
        assert(k != at + before);
        assert(dst[k] == prev[k]);
    };
    assert forall|k: int| 0 <= k < done implies #[trigger] dst[at + k] == src[start + k] by {
        if k < before {
            assert(at + k != at + before);
            assert(dst[at + k] == prev[at + k]);
        } else {
            assert(k == before);
            assert(dst[at + before] == src[start + before]);
        }
    };
    assert forall|k: int| at + done <= k < dst.len() implies #[trigger] dst[k] == original[k] by {
        assert(k != at + before);
        assert(dst[k] == prev[k]);
    };
}

/// `cursor` still has room for one destination byte, and the source offset fits.
proof fn lemma_copy_ready(at: usize, cursor: usize, end: usize, count: usize, len: int)
    requires
        at <= cursor < end,
        end == at + count,
        end <= len,
        len <= usize::MAX as int,
    ensures
        at <= cursor,
        (cursor as int) - (at as int) < count as int,
        cursor < len,
{
}

/// `start + done` is the next source index and fits in a `usize`.
proof fn lemma_src_fits(start: usize, done: usize, count: usize)
    requires
        done < count,
        count == DECIMAL_CAP - start,
    ensures
        (start as int) + (done as int) < DECIMAL_CAP as int,
        (start as int) + (done as int) <= usize::MAX as int,
{
    assert((start as int) + (done as int) < (start as int) + (count as int));
    assert((start as int) + (count as int) == DECIMAL_CAP as int);
}

/// The stored byte extends the copied run, and `cursor + 1` stays in range.
proof fn lemma_stored_copy(
    prev: Seq<u8>,
    dst: Seq<u8>,
    original: Seq<u8>,
    src: Seq<u8>,
    at: int,
    start: int,
    before: int,
    cursor: int,
    src_at: int,
    end: int,
)
    requires
        dst == prev.update(cursor, src[src_at]),
        cursor == at + before,
        src_at == start + before,
        0 <= before < src.len() - start,
        0 <= at,
        at + before < prev.len(),
        dst.len() == prev.len() == original.len(),
        src.len() == DECIMAL_CAP,
        cursor < end,
        end <= usize::MAX,
        forall|k: int| 0 <= k < at ==> #[trigger] prev[k] == original[k],
        forall|k: int| 0 <= k < before ==> #[trigger] prev[at + k] == src[start + k],
        forall|k: int| at + before <= k < prev.len() ==> #[trigger] prev[k] == original[k],
    ensures
        cursor + 1 <= end,
        cursor + 1 <= usize::MAX,
        dst.len() == original.len(),
        forall|k: int| 0 <= k < at ==> #[trigger] dst[k] == original[k],
        forall|k: int| 0 <= k < before + 1 ==> #[trigger] dst[at + k] == src[start + k],
        forall|k: int| at + before + 1 <= k < dst.len() ==> #[trigger] dst[k] == original[k],
{
    lemma_copy_step(prev, dst, original, src, at, start, before, before + 1);
    assert(cursor + 1 <= end);
    assert(cursor + 1 <= usize::MAX);
}

/// Loop invariant for [`copy_run`]: prefix, copied run, and untouched tail.
spec fn copy_inv(
    dst: Seq<u8>,
    original: Seq<u8>,
    src: Seq<u8>,
    at: int,
    start: int,
    cursor: int,
    end: int,
    count: int,
) -> bool {
    &&& at <= cursor <= end
    &&& end == at + count
    &&& count == DECIMAL_CAP as int - start
    &&& end <= dst.len()
    &&& dst.len() == original.len()
    &&& dst.len() <= usize::MAX as int
    &&& src.len() == DECIMAL_CAP
    &&& forall|k: int| 0 <= k < at ==> #[trigger] dst[k] == original[k]
    &&& forall|k: int| 0 <= k < cursor - at ==> #[trigger] dst[at + k] == src[start + k]
    &&& forall|k: int| cursor <= k < dst.len() ==> #[trigger] dst[k] == original[k]
}

/// Copy `src[start..]` into `dst[at..end]`.
fn copy_run(
    dst: &mut [u8],
    src: &[u8; DECIMAL_CAP],
    at: usize,
    start: usize,
    end: usize,
    count: usize,
) -> (copied: usize)
    requires
        count == DECIMAL_CAP - start,
        end == at + count,
        end <= dst@.len(),
        dst@.len() <= usize::MAX as int,
    ensures
        copied == count,
        final(dst)@.len() == old(dst)@.len(),
        forall|k: int| 0 <= k < at ==> #[trigger] final(dst)@[k] == old(dst)@[k],
        forall|k: int|
            0 <= k < copied ==> #[trigger] final(dst)@[at as int + k] == src@[start as int + k],
        forall|k: int|
            at as int + copied as int <= k < old(dst)@.len() ==> #[trigger] final(dst)@[k] == old(
                dst,
            )@[k],
{
    let mut cursor = at;
    while cursor < end
        invariant
            copy_inv(
                dst@,
                old(dst)@,
                src@,
                at as int,
                start as int,
                cursor as int,
                end as int,
                count as int,
            ),
        decreases (end as int) - (cursor as int),
    {
        proof {
            lemma_copy_ready(at, cursor, end, count, dst@.len() as int);
        }
        let done = cursor - at;
        proof {
            lemma_src_fits(start, done, count);
        }
        let src_at = start + done;
        let ghost prev = dst@;
        dst[cursor] = src[src_at];
        proof {
            lemma_stored_copy(
                prev,
                dst@,
                old(dst)@,
                src@,
                at as int,
                start as int,
                done as int,
                cursor as int,
                src_at as int,
                end as int,
            );
        }
        cursor += 1;
    }
    count
}

/// Copy `src[start..DECIMAL_CAP]` into `dst` starting at `at`.
fn copy_digits(dst: &mut [u8], src: &[u8; DECIMAL_CAP], at: usize, start: usize) -> (copied: usize)
    requires
        start <= DECIMAL_CAP,
        at + (DECIMAL_CAP - start) <= dst@.len(),
    ensures
        copied == DECIMAL_CAP - start,
        final(dst)@.len() == old(dst)@.len(),
        forall|k: int| 0 <= k < at ==> #[trigger] final(dst)@[k] == old(dst)@[k],
        forall|k: int|
            0 <= k < copied ==> #[trigger] final(dst)@[at as int + k] == src@[start as int + k],
        forall|k: int|
            at as int + copied as int <= k < old(dst)@.len() ==> #[trigger] final(dst)@[k] == old(
                dst,
            )@[k],
{
    let count = DECIMAL_CAP - start;
    proof {
        assert(spec_slice_len(dst) == dst@.len());
        assert(dst@.len() <= usize::MAX as int);
        assert((at as int) + (count as int) <= usize::MAX as int);
    }
    let end = at + count;
    copy_run(dst, src, at, start, end, count)
}

/// Loop invariant for [`fill_decimal`].
spec fn decimal_inv(value: int, remaining: int, index: int, tmp: Seq<u8>) -> bool {
    &&& 0 <= index <= DECIMAL_CAP as int
    &&& index >= 1 || remaining == 0
    &&& below_pow(remaining, index)
    &&& place_holds(value, remaining, index, tmp)
    &&& suffix_is_digits(index, tmp)
    &&& no_leading_zero(index, remaining, tmp)
    &&& tmp.len() == DECIMAL_CAP
}

/// Fill `tmp` from the end. Returns the index of the first digit.
fn fill_decimal(tmp: &mut [u8; DECIMAL_CAP], value: u64) -> (index: usize)
    ensures
        index < DECIMAL_CAP,
        decimal_digits(final(tmp)@[index as int..DECIMAL_CAP as int]),
        parse_digits(final(tmp)@[index as int..DECIMAL_CAP as int]) == value as int,
{
    let mut index: usize = DECIMAL_CAP;
    let mut remaining: u64 = value;
    proof {
        lemma_fill_start(value, tmp@);
    }
    while remaining > 0 || index == DECIMAL_CAP
        invariant
            decimal_inv(value as int, remaining as int, index as int, tmp@),
        decreases index,
    {
        let ghost snap = tmp@;
        let ghost idx = index as int;
        let ghost rem = remaining as int;
        let (next, byte) = quot_rem_10(remaining);
        proof {
            lemma_write_one(value as int, rem, idx, snap, byte, next as int);
        }
        index -= 1;
        tmp[index] = byte;
        remaining = next;
        proof {
            assert(index as int == idx - 1);
            assert(remaining as int == next as int);
            assert(tmp@ == snap.update(index as int, byte));
            lemma_stored(
                value as int,
                snap,
                idx,
                index as int,
                byte,
                next as int,
                remaining as int,
                tmp@,
            );
        }
    }
    proof {
        assert(remaining == 0);
        assert(index < DECIMAL_CAP);
        lemma_fill_done(value, index as int, remaining as int, tmp@);
    }
    index
}

/// The copied prefix is the canonical decimal, byte for byte.
proof fn lemma_copied(buf: Seq<u8>, tmp: Seq<u8>, index: int, written: int, value: u64)
    requires
        tmp.len() == DECIMAL_CAP,
        0 <= index < DECIMAL_CAP as int,
        written == DECIMAL_CAP as int - index,
        written <= buf.len(),
        forall|k: int| 0 <= k < written ==> #[trigger] buf[k] == tmp[index + k],
        decimal_digits(tmp[index..DECIMAL_CAP as int]),
        parse_digits(tmp[index..DECIMAL_CAP as int]) == value as int,
    ensures
        1 <= written <= DECIMAL_CAP,
        decimal_digits(buf[0..written]),
        parse_digits(buf[0..written]) == value as int,
{
    let suffix = tmp[index..DECIMAL_CAP as int];
    assert(suffix.len() == written);
    assert forall|k: int| 0 <= k < written implies #[trigger] buf[k] == suffix[k] by {
        assert(suffix[k] == tmp[index + k]);
    };
    assert(buf[0..written] =~= suffix);
}

/// Decimal digits of `value`, least-significant digit last, with no leading zero.
fn write_u64(buf: &mut [u8], value: u64) -> (written: usize)
    requires
        buf@.len() >= DECIMAL_CAP,
    ensures
        1 <= written <= DECIMAL_CAP,
        written <= old(buf)@.len(),
        final(buf)@.len() == old(buf)@.len(),
        decimal_digits(final(buf)@[0..written as int]),
        parse_digits(final(buf)@[0..written as int]) == value as int,
        forall|i: int| written as int <= i < old(buf)@.len() ==> final(buf)@[i] == old(buf)@[i],
{
    let mut tmp = [0u8;DECIMAL_CAP];
    let index = fill_decimal(&mut tmp, value);
    proof {
        assert(index < DECIMAL_CAP);
        assert(DECIMAL_CAP - index <= buf@.len());
    }
    let written = copy_digits(buf, &tmp, 0, index);
    proof {
        assert(written as int == DECIMAL_CAP as int - index as int);
        assert(written as int <= buf@.len());
        assert forall|k: int| 0 <= k < written as int implies #[trigger] buf@[k] == tmp@[(
        index as int) + k] by {
            assert(buf@[(0 as int) + k] == tmp@[(index as int) + k]);
            assert((0 as int) + k == k);
        };
        lemma_copied(buf@, tmp@, index as int, written as int, value);
    }
    written
}

/// The sign byte plus the copied digits is the decimal of the magnitude.
proof fn lemma_signed_prefix(
    original: Seq<u8>,
    signed: Seq<u8>,
    buf: Seq<u8>,
    tmp: Seq<u8>,
    index: int,
    digits: int,
    magnitude: int,
    out: int,
)
    requires
        original.len() == signed.len() == buf.len(),
        tmp.len() == DECIMAL_CAP,
        0 <= index < DECIMAL_CAP as int,
        digits == DECIMAL_CAP as int - index,
        out == 1 + digits,
        1 <= out <= buf.len(),
        signed.len() > 0,
        signed[0] == b'-',
        forall|i: int| 1 <= i < signed.len() ==> #[trigger] signed[i] == original[i],
        forall|k: int| 0 <= k < 1 ==> #[trigger] buf[k] == signed[k],
        forall|k: int| 0 <= k < digits ==> #[trigger] buf[1 + k] == tmp[index + k],
        forall|k: int| 1 + digits <= k < buf.len() ==> #[trigger] buf[k] == signed[k],
        decimal_digits(tmp[index..DECIMAL_CAP as int]),
        parse_digits(tmp[index..DECIMAL_CAP as int]) == magnitude,
    ensures
        buf[0] == b'-',
        decimal_digits(buf[1..out]),
        parse_digits(buf[1..out]) == magnitude,
        forall|i: int| out <= i < original.len() ==> buf[i] == original[i],
{
    let suffix = tmp[index..DECIMAL_CAP as int];
    assert(suffix.len() == digits);
    assert(0 <= 1 <= out <= buf.len());
    assert(buf[1..out].len() == suffix.len());
    assert forall|k: int| 0 <= k < digits implies #[trigger] buf[1..out][k] == suffix[k] by {
        assert(buf[1..out][k] == buf[1 + k]);
        assert(buf[1 + k] == tmp[index + k]);
        assert(suffix[k] == tmp[index + k]);
    };
    assert(buf[1..out] =~= suffix);
    assert forall|i: int| out <= i < original.len() implies #[trigger] buf[i] == original[i] by {
        assert(i < buf.len());
        assert(1 + digits <= i);
        assert(buf[i] == signed[i]);
        assert(signed[i] == original[i]);
    };
}

/// Sign plus the digits of `spec_unsigned_abs`.
fn write_negative(buf: &mut [u8], value: i64) -> (out: usize)
    requires
        value < 0,
        buf@.len() >= DECIMAL_CAP + 1,
    ensures
        2 <= out <= DECIMAL_CAP + 1,
        out <= old(buf)@.len(),
        final(buf)@.len() == old(buf)@.len(),
        final(buf)@[0] == b'-',
        decimal_digits(final(buf)@[1..out as int]),
        parse_digits(final(buf)@[1..out as int]) == spec_unsigned_abs(value),
        forall|i: int| out as int <= i < old(buf)@.len() ==> final(buf)@[i] == old(buf)@[i],
{
    let magnitude = unsigned_magnitude(value);
    let mut tmp = [0u8;DECIMAL_CAP];
    let index = fill_decimal(&mut tmp, magnitude);
    buf[0] = b'-';
    proof {
        assert(buf@.len() == old(buf)@.len());
        assert(buf@ == old(buf)@.update(0, b'-'));
        assert(buf@[0] == b'-');
        assert forall|i: int| 1 <= i < buf@.len() implies #[trigger] buf@[i] == old(buf)@[i] by {
            assert(buf@[i] == old(buf)@.update(0, b'-')[i]);
        };
        assert(1 + (DECIMAL_CAP - index) <= buf@.len());
    }
    let ghost signed = buf@;
    let digits = copy_digits(buf, &tmp, 1, index);
    proof {
        assert(digits <= DECIMAL_CAP);
        assert((1 as int) + (digits as int) <= usize::MAX as int);
        assert(buf@.len() == signed.len() == old(buf)@.len());
    }
    let out = 1 + digits;
    proof {
        assert(out as int == 1 + (digits as int));
        assert(1 <= out as int <= buf@.len());
        assert(magnitude as int == spec_unsigned_abs(value));
        lemma_signed_prefix(
            old(buf)@,
            signed,
            buf@,
            tmp@,
            index as int,
            digits as int,
            magnitude as int,
            out as int,
        );
    }
    out
}

/// Digits of a non-negative value, with no sign byte.
fn write_nonneg(buf: &mut [u8], value: i64) -> (out: usize)
    requires
        value >= 0,
        buf@.len() >= DECIMAL_CAP,
    ensures
        1 <= out <= DECIMAL_CAP,
        out <= old(buf)@.len(),
        final(buf)@.len() == old(buf)@.len(),
        decimal_digits(final(buf)@[0..out as int]),
        parse_digits(final(buf)@[0..out as int]) == spec_unsigned_abs(value),
        forall|i: int| 0 <= i < out ==> final(buf)@[i] != b'-',
        forall|i: int| out as int <= i < old(buf)@.len() ==> final(buf)@[i] == old(buf)@[i],
{
    proof {
        assert(value as u64 as int == value as int) by (bit_vector)
            requires
                0 <= value,
        ;
    }
    // Same bits as the `i64` when the sign bit is clear.
    #[expect(clippy::cast_sign_loss, reason = "value is non-negative")]
    let magnitude = value as u64;
    let out = write_u64(buf, magnitude);
    proof {
        assert(magnitude as int == value as int);
        assert(spec_unsigned_abs(value) == value as int);
        assert(buf@.len() == old(buf)@.len());
        assert(out as int <= buf@.len());
        assert forall|i: int| 0 <= i < out && i < buf@.len() implies #[trigger] buf@[i] != b'-' by {
            let digits = buf@[0..out as int];
            assert(decimal_digits(digits));
            assert(is_ascii_digit(digits[i]));
            assert(digits[i] == buf@[i]);
            assert(b'-' < b'0');
        };
    }
    out
}

/// Write the decimal form of `value` into `buf`.
///
/// Returns the number of bytes written. `buf` has to hold the sign and the digits.
/// A negative value takes a sign plus at most 20 digits. A non-negative value
/// takes at most 20 digits and no sign.
///
/// # Panics
///
/// Panics if `buf` is too small for the sign and the digits.
pub fn write_i64(buf: &mut [u8], value: i64) -> (out: usize)
    requires
        value < 0 ==> buf@.len() >= 21,
        value >= 0 ==> buf@.len() >= 20,
    ensures
        out <= old(buf)@.len(),
        final(buf)@.len() == old(buf)@.len(),
        forall|i: int| out as int <= i < old(buf)@.len() ==> final(buf)@[i] == old(buf)@[i],
        value < 0 ==> {
            &&& 2 <= out <= 21
            &&& final(buf)@[0] == b'-'
            &&& decimal_digits(final(buf)@[1..out as int])
            &&& parse_digits(final(buf)@[1..out as int]) == spec_unsigned_abs(value)
        },
        value >= 0 ==> {
            &&& 1 <= out <= 20
            &&& decimal_digits(final(buf)@[0..out as int])
            &&& parse_digits(final(buf)@[0..out as int]) == spec_unsigned_abs(value)
            &&& forall|i: int| 0 <= i < out ==> final(buf)@[i] != b'-'
        },
{
    if value < 0 {
        write_negative(buf, value)
    } else {
        write_nonneg(buf, value)
    }
}

} // verus!
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
                    // `checked_div` is `None` for a zero divisor and for
                    // `i64::MIN / -1`, whose quotient is not an `i64`.
                    value = value.checked_div(right).ok_or(Error)?;
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
        assert_eq!(eval(b"-5/2"), Ok(-2));
    }

    #[test]
    fn division_by_zero_fails() {
        assert_eq!(eval(b"1/0"), Err(Error));
    }

    #[test]
    fn division_overflow_fails() {
        // `(-9223372036854775807)-1` is `i64::MIN`. Dividing that by `-1`
        // is not an `i64`. The same path that rejects a zero divisor rejects
        // it, instead of panicking inside the maths cothread.
        assert_eq!(eval(b"((-9223372036854775807)-1)/1"), Ok(i64::MIN));
        assert_eq!(eval(b"((-9223372036854775807)-1)/-1"), Err(Error));
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

    #[test]
    fn decimal_edges_match_unsigned_abs() {
        let mut buf = [0u8; 21];
        assert_eq!(write_i64(&mut buf, 0), 1);
        assert_eq!(&buf[..1], b"0");
        assert_eq!(write_i64(&mut buf, i64::MAX), 19);
        assert_eq!(&buf[..19], b"9223372036854775807");
        assert_eq!(write_i64(&mut buf, i64::MIN), 20);
        assert_eq!(&buf[..20], b"-9223372036854775808");
        assert_eq!(i64::MIN.unsigned_abs(), 1u64 << 63);
    }
}
