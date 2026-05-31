use core::arch::asm;

use rand_chacha::ChaCha20Rng;
use rand_core::{RngCore, SeedableRng};
use redox_scheme::scheme::SchemeSync;
use redox_scheme::{CallerCtx, OpenResult};
use scheme_utils::FpathWriter;
use syscall::data::Stat;
use syscall::schemev2::NewFdFlags;
use syscall::{Error, Result, EBADF, MODE_CHR};

const SEED_BYTES: usize = 32;

fn create_rdrand_seed() -> [u8; SEED_BYTES] {
    let mut rng = [0u8; SEED_BYTES];
    #[cfg(target_arch = "x86_64")]
    {
        if raw_cpuid::CpuId::new()
            .get_feature_info()
            .map(|f| f.has_rdrand())
            .unwrap_or(false)
        {
            for i in 0..SEED_BYTES / 8 {
                let rand: u64;
                unsafe {
                    asm!("rdrand rax", out("rax") rand);
                }
                rng[i * 8..(i * 8 + 8)].copy_from_slice(&rand.to_le_bytes());
            }
            return rng;
        }
    }
    rng
}

pub struct RandScheme {
    prng: ChaCha20Rng,
}

impl RandScheme {
    pub fn new() -> Self {
        RandScheme {
            prng: ChaCha20Rng::from_seed(create_rdrand_seed()),
        }
    }
}

impl SchemeSync for RandScheme {
    fn scheme_root(&mut self) -> Result<usize> {
        Ok(0)
    }

    fn openat(
        &mut self,
        _dirfd: usize,
        _path: &str,
        _flags: usize,
        _fcntl_flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<OpenResult> {
        Ok(OpenResult::ThisScheme {
            number: 0,
            flags: NewFdFlags::empty(),
        })
    }

    fn read(
        &mut self,
        _id: usize,
        buf: &mut [u8],
        _offset: u64,
        _flags: u32,
        _ctx: &CallerCtx,
    ) -> Result<usize> {
        self.prng.fill_bytes(buf);
        Ok(buf.len())
    }

    fn write(&mut self, _id: usize, buf: &[u8], _offset: u64, _flags: u32, _ctx: &CallerCtx) -> Result<usize> {
        let _ = buf;
        Ok(buf.len())
    }

    fn fpath(&mut self, _id: usize, buf: &mut [u8], _ctx: &CallerCtx) -> Result<usize> {
        FpathWriter::with(buf, "rand", |_| Ok(()))
    }

    fn fstat(&mut self, _id: usize, stat: &mut Stat, _ctx: &CallerCtx) -> Result<()> {
        stat.st_mode = MODE_CHR | 0o644;
        Ok(())
    }

    fn on_close(&mut self, _id: usize) {}
}
