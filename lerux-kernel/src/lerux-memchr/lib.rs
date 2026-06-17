#![no_std]
#[inline] pub fn memchr(needle: u8, haystack: &[u8]) -> Option<usize> { haystack.iter().position(|&b| b == needle) }
#[inline] pub fn memrchr(needle: u8, haystack: &[u8]) -> Option<usize> { haystack.iter().rposition(|&b| b == needle) }
#[inline] pub fn memchr2(n1: u8, n2: u8, h: &[u8]) -> Option<usize> { h.iter().position(|&b| b==n1 || b==n2) }
#[inline] pub fn memrchr2(n1: u8, n2: u8, h: &[u8]) -> Option<usize> { h.iter().rposition(|&b| b==n1 || b==n2) }
#[inline] pub fn memchr3(n1:u8,n2:u8,n3:u8,h:&[u8])->Option<usize>{h.iter().position(|&b|b==n1||b==n2||b==n3)}
#[inline] pub fn memrchr3(n1:u8,n2:u8,n3:u8,h:&[u8])->Option<usize>{h.iter().rposition(|&b|b==n1||b==n2||b==n3)}
pub struct Memchr<'a>(core::marker::PhantomData<&'a u8>);
pub struct Memchr2<'a>(core::marker::PhantomData<&'a u8>);
pub struct Memchr3<'a>(core::marker::PhantomData<&'a u8>);
#[inline] pub fn memchr_iter(needle: u8, haystack: &[u8]) -> impl Iterator<Item=usize> { haystack.iter().enumerate().filter_map(move|(i,&b)| if b==needle{Some(i)}else{None}) }
pub mod arch { pub mod generic {} pub mod all {} pub mod x86_64 {} }
pub mod memmem { #[inline] pub fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> { haystack.windows(needle.len()).position(|w| w == needle) } }
pub use self::memchr as memchr1;
