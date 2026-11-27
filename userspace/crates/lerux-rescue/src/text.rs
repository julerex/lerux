pub(crate) struct Buf<'a> {
    out: &'a mut [u8],
    n: usize,
}

impl<'a> Buf<'a> {
    pub(crate) fn new(out: &'a mut [u8]) -> Self {
        Self { out, n: 0 }
    }

    pub(crate) fn push(&mut self, bytes: &[u8]) -> bool {
        let Some(next) = self.n.checked_add(bytes.len()) else {
            return false;
        };
        if next > self.out.len() {
            return false;
        }
        self.out[self.n..next].copy_from_slice(bytes);
        self.n = next;
        true
    }

    pub(crate) fn u64(&mut self, value: u64) -> bool {
        let mut tmp = [0u8; 20];
        let mut index = tmp.len();
        let mut left = value;
        loop {
            index -= 1;
            let Ok(digit) = u8::try_from(left % 10) else {
                return false;
            };
            tmp[index] = b'0' + digit;
            left /= 10;
            if left == 0 {
                break;
            }
        }
        self.push(&tmp[index..])
    }

    pub(crate) const fn len(&self) -> usize {
        self.n
    }
}

pub(crate) fn show(bytes: &[u8], out: &mut [u8]) -> Option<usize> {
    if bytes.len() > out.len() {
        return None;
    }
    out[..bytes.len()].copy_from_slice(bytes);
    Some(bytes.len())
}
