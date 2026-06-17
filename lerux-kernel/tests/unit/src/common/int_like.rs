//! Tests for the `int_like!` macro defined in the kernel.

#[macro_use]
#[path = "../../../../src/common/int_like.rs"]
mod int_like_def;

#[cfg(test)]
mod tests {
    use core::mem::size_of;
    use core::sync::atomic::AtomicUsize;

    #[test]
    fn usize_like_has_expected_size_and_ordering() {
        int_like!(UsizeLike, usize);
        assert_eq!(size_of::<UsizeLike>(), size_of::<usize>());

        let a = UsizeLike::new(42);
        assert_eq!(a.get(), 42);
        assert!(a > UsizeLike::new(41));
        assert_eq!(UsizeLike::from(43usize).get(), 43);
    }

    #[test]
    fn atomic_usize_like_has_expected_size_and_round_trip() {
        int_like!(UsizeLike2, AtomicUsizeLike, usize, AtomicUsize);
        assert_eq!(size_of::<UsizeLike2>(), size_of::<usize>());
        assert_eq!(size_of::<AtomicUsizeLike>(), size_of::<AtomicUsize>());

        let atomic = AtomicUsizeLike::default();
        assert_eq!(atomic.load(core::sync::atomic::Ordering::Relaxed).get(), 0);

        atomic.store(
            UsizeLike2::new(7),
            core::sync::atomic::Ordering::Relaxed,
        );
        assert_eq!(atomic.load(core::sync::atomic::Ordering::Relaxed).get(), 7);
    }
}
