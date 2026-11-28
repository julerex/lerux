//! Little-endian and big-endian integer wrappers.

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Le16(pub u16);
impl Le16 {
    #[must_use]
    pub const fn from_native(value: u16) -> Self {
        Self(value.to_le())
    }

    #[must_use]
    pub const fn to_native(self) -> u16 {
        u16::from_le(self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Le32(pub u32);
impl Le32 {
    #[must_use]
    pub const fn from_native(value: u32) -> Self {
        Self(value.to_le())
    }

    #[must_use]
    pub const fn to_native(self) -> u32 {
        u32::from_le(self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Le64(pub u64);
impl Le64 {
    #[must_use]
    pub const fn from_native(value: u64) -> Self {
        Self(value.to_le())
    }

    #[must_use]
    pub const fn to_native(self) -> u64 {
        u64::from_le(self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Be16(pub u16);
impl Be16 {
    #[must_use]
    pub const fn from_native(value: u16) -> Self {
        Self(value.to_be())
    }

    #[must_use]
    pub const fn to_native(self) -> u16 {
        u16::from_be(self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Be32(pub u32);
impl Be32 {
    #[must_use]
    pub const fn from_native(value: u32) -> Self {
        Self(value.to_be())
    }

    #[must_use]
    pub const fn to_native(self) -> u32 {
        u32::from_be(self.0)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Be64(pub u64);
impl Be64 {
    #[must_use]
    pub const fn from_native(value: u64) -> Self {
        Self(value.to_be())
    }

    #[must_use]
    pub const fn to_native(self) -> u64 {
        u64::from_be(self.0)
    }
}
