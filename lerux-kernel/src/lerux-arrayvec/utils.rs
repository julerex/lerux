use core::marker::PhantomData;
use core::mem::MaybeUninit;

pub(crate) struct MakeMaybeUninit<T, const N: usize>(PhantomData<fn() -> T>);

impl<T, const N: usize> MakeMaybeUninit<T, N> {
    pub(crate) const VALUE: MaybeUninit<T> = MaybeUninit::uninit();

    pub(crate) const ARRAY: [MaybeUninit<T>; N] = [Self::VALUE; N];
}

