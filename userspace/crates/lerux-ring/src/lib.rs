//! Index arithmetic for a single-producer single-consumer ring.
//!
//! The producer is the only writer of `tail`. The consumer is the only writer
//! of `head`. The length is the wrapping difference of those counters. A ring
//! is full when that difference equals its capacity, and empty when the
//! counters are equal. The slot is `cursor % capacity`.
//!
//! [`word16`], [`word32`], and [`word64`] are the counter widths used by the
//! network, serial, block, graphics, and filesystem queues. Verus checks that
//! a push into a ring that is not full increases the length by one, a pop from
//! a ring that is not empty decreases it by one, and the slot is strictly
//! below the capacity. The proofs are erased from the executable functions.
#![cfg_attr(not(test), no_std)]

#[cfg(test)]
extern crate std;

/// One counter width. `$module` is the public module name and `$ty` is the
/// unsigned width of `head` and `tail`.
macro_rules! ring_word {
    ($module:ident, $ty:ident) => {
        pub mod $module {
            // The verifier's expansion names `vstd` and `int`. A normal build
            // erases that expansion, so the imports are unused unless `verus`
            // is verifying. Protection-domain builds leave the feature off.
            #[cfg(feature = "verus")]
            #[allow(unused_imports)]
            use verus_builtin::*;
            use verus_builtin_macros::verus;
            #[cfg(feature = "verus")]
            #[allow(unused_imports)]
            use vstd::prelude::*;

            verus! {

            /// Wrapping distance from `head` up to `tail`.
            pub open spec fn spec_len(tail: $ty, head: $ty) -> $ty {
                tail.wrapping_sub(head)
            }

            /// Next counter, wrapping at the word boundary.
            pub open spec fn spec_advance(cursor: $ty) -> $ty {
                cursor.wrapping_add(1)
            }

            /// Executable length. Equal to [`spec_len`].
            #[inline]
            pub fn len(tail: $ty, head: $ty) -> (out: $ty)
                ensures
                    out == spec_len(tail, head),
            {
                tail.wrapping_sub(head)
            }

            /// True when `tail` and `head` name the same counter value.
            ///
            /// That is the same condition as a length of zero.
            #[inline]
            pub fn is_empty(tail: $ty, head: $ty) -> (empty: bool)
                ensures
                    empty == (tail == head),
                    empty == (spec_len(tail, head) == 0),
            {
                assert(tail == head <==> tail.wrapping_sub(head) == 0) by (bit_vector);
                tail == head
            }

            /// True when the ring already holds `capacity` elements.
            #[inline]
            pub fn is_full(tail: $ty, head: $ty, capacity: $ty) -> (full: bool)
                ensures
                    full == (spec_len(tail, head) == capacity),
            {
                len(tail, head) == capacity
            }

            /// Slot of `cursor`.
            ///
            /// `capacity` must be non-zero. The result is strictly below it.
            #[inline]
            pub fn index(cursor: $ty, capacity: $ty) -> (slot: $ty)
                requires
                    capacity > 0,
                ensures
                    slot < capacity,
                    slot == cursor % capacity,
            {
                cursor % capacity
            }

            /// Next value of a counter. The length lemmas are [`advance_tail`]
            /// and [`advance_head`].
            #[inline]
            pub fn advance(cursor: $ty) -> (next: $ty)
                ensures
                    next == spec_advance(cursor),
            {
                cursor.wrapping_add(1)
            }

            /// Push one element: `tail` moves forward and the length grows by one.
            ///
            /// The precondition is the not-full check. A length strictly below
            /// the capacity has room for one more count, including when `tail`
            /// itself wraps the word. `head` and `capacity` are that
            /// precondition. The executable result is only the next tail.
            #[inline]
            pub fn advance_tail(tail: $ty, head: $ty, capacity: $ty) -> (next: $ty)
                requires
                    capacity > 0,
                    spec_len(tail, head) < capacity,
                ensures
                    next == spec_advance(tail),
                    spec_len(next, head) as int == spec_len(tail, head) as int + 1,
            {
                let next = advance(tail);
                let _ = (head, capacity);
                proof {
                    lemma_advance_tail(tail, head, capacity);
                }
                next
            }

            /// Pop one element: `head` moves forward and the length shrinks by one.
            ///
            /// The precondition is the not-empty check. `tail` is that
            /// precondition. The executable result is only the next head.
            #[inline]
            pub fn advance_head(head: $ty, tail: $ty) -> (next: $ty)
                requires
                    spec_len(tail, head) > 0,
                ensures
                    next == spec_advance(head),
                    spec_len(tail, next) as int + 1 == spec_len(tail, head) as int,
            {
                let next = advance(head);
                let _ = tail;
                proof {
                    lemma_advance_head(head, tail);
                }
                next
            }

            /// A not-full push increases the wrapping length by one.
            pub proof fn lemma_advance_tail(tail: $ty, head: $ty, capacity: $ty)
                requires
                    capacity > 0,
                    tail.wrapping_sub(head) < capacity,
                ensures
                    tail.wrapping_add(1).wrapping_sub(head) as int
                        == tail.wrapping_sub(head) as int + 1,
            {
                assert(
                    tail.wrapping_add(1).wrapping_sub(head) == tail.wrapping_sub(head) + 1
                ) by (bit_vector)
                    requires
                        capacity > 0,
                        tail.wrapping_sub(head) < capacity;
            }

            /// A not-empty pop decreases the wrapping length by one.
            pub proof fn lemma_advance_head(head: $ty, tail: $ty)
                requires
                    tail.wrapping_sub(head) > 0,
                ensures
                    tail.wrapping_sub(head.wrapping_add(1)) as int + 1
                        == tail.wrapping_sub(head) as int,
            {
                assert(
                    tail.wrapping_sub(head.wrapping_add(1)) + 1 == tail.wrapping_sub(head)
                ) by (bit_vector)
                    requires
                        tail.wrapping_sub(head) > 0;
            }

            }
        }
    };
}

ring_word!(word16, u16);
ring_word!(word32, u32);
ring_word!(word64, u64);

#[cfg(test)]
mod tests {
    use super::{word16, word32, word64};

    #[test]
    fn full_and_empty_follow_the_wrapping_difference() {
        assert!(word32::is_empty(0, 0));
        assert!(!word32::is_full(0, 0, 16));
        assert!(word32::is_full(16, 0, 16));
        assert!(!word32::is_full(15, 0, 16));
        assert_eq!(word32::len(16, 0), 16);
        assert!(word64::is_full(511, 0, 511));
        assert!(word16::is_empty(7, 7));
        assert_eq!(word16::len(1, 0), 1);
    }

    #[test]
    fn index_is_the_cursor_modulo_the_capacity() {
        assert_eq!(word32::index(20, 16), 4);
        assert!(word32::index(u32::MAX, 16) < 16);
        assert_eq!(word64::index(511, 511), 0);
        assert_eq!(word64::index(510, 511), 510);
        assert_eq!(word16::index(17, 16), 1);
    }

    #[test]
    fn push_across_the_u32_boundary_increases_length_by_one() {
        let capacity = 16;
        let head = u32::MAX - 4;
        let tail = u32::MAX;
        assert_eq!(word32::len(tail, head), 4);
        assert!(!word32::is_full(tail, head, capacity));
        let next = word32::advance_tail(tail, head, capacity);
        assert_eq!(next, 0);
        assert_eq!(word32::len(next, head), 5);
        assert_eq!(word32::index(tail, capacity), u32::MAX % capacity);
    }

    #[test]
    fn pop_across_the_u32_boundary_decreases_length_by_one() {
        let tail = 0;
        let head = u32::MAX;
        assert_eq!(word32::len(tail, head), 1);
        assert!(!word32::is_empty(tail, head));
        let next = word32::advance_head(head, tail);
        assert_eq!(next, 0);
        assert!(word32::is_empty(tail, next));
    }

    #[test]
    fn push_across_the_u16_and_u64_boundaries_increases_length_by_one() {
        let next16 = word16::advance_tail(u16::MAX, u16::MAX - 3, 16);
        assert_eq!(next16, 0);
        assert_eq!(word16::len(next16, u16::MAX - 3), 4);

        let next64 = word64::advance_tail(u64::MAX, u64::MAX - 3, 511);
        assert_eq!(next64, 0);
        assert_eq!(word64::len(next64, u64::MAX - 3), 4);
    }
}
