//! Protection-domain priority and budget for one LionsOS image.
//!
//! Microkit copies `priority`, `budget`, and `period` from the system
//! description into each scheduling context. The shell prints the same
//! numbers from this table. `period` equals `budget`, which is the
//! round-robin case.

/// Bytes reserved for one protection-domain name.
pub const SCHED_NAME_LEN: usize = 16;

/// One row of the filesystem image.
pub struct Role {
    pub name: &'static str,
    pub priority: u8,
    pub budget_us: u32,
    pub passive: bool,
}

/// Rows in template order. `budget_us` is both the budget and the period.
pub const FS_IMAGE_ROLES: [Role; 8] = [
    Role {
        name: "blk_driver",
        priority: 4,
        budget_us: 4000,
        passive: false,
    },
    Role {
        name: "serial_driver",
        priority: 4,
        budget_us: 4000,
        passive: false,
    },
    Role {
        name: "blk_virt",
        priority: 3,
        budget_us: 3000,
        passive: false,
    },
    Role {
        name: "serial_virt_tx",
        priority: 3,
        budget_us: 3000,
        passive: false,
    },
    Role {
        name: "serial_virt_rx",
        priority: 3,
        budget_us: 3000,
        passive: false,
    },
    Role {
        name: "fatfs",
        priority: 2,
        budget_us: 2000,
        passive: false,
    },
    Role {
        name: "timer_driver",
        priority: 5,
        budget_us: 5000,
        passive: true,
    },
    Role {
        name: "shell",
        priority: 1,
        budget_us: 1000,
        passive: false,
    },
];

/// One stored row. The name is zero-padded.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sched_entry_t {
    pub name: [u8; SCHED_NAME_LEN],
    pub priority: u8,
    pub _pad: [u8; 3],
    pub budget_us: u32,
}

/// Schedule page embedded after the shell's other configuration pages.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct sched_config_t {
    pub count: u8,
    pub _pad: [u8; 7],
    pub entry: [sched_entry_t; FS_IMAGE_ROLES.len()],
}

const _: () = {
    let mut index = 0;
    while index < FS_IMAGE_ROLES.len() {
        let len = FS_IMAGE_ROLES[index].name.len();
        assert!(len > 0 && len < SCHED_NAME_LEN);
        index += 1;
    }
};

/// Schedule page for `qemu_virt_aarch64_fs_sddf`.
pub fn fs_image_schedule() -> sched_config_t {
    let mut config: sched_config_t = unsafe { core::mem::zeroed() };
    config.count = u8::try_from(FS_IMAGE_ROLES.len()).expect("role count fits");
    for (index, role) in FS_IMAGE_ROLES.iter().enumerate() {
        let mut name = [0u8; SCHED_NAME_LEN];
        name[..role.name.len()].copy_from_slice(role.name.as_bytes());
        config.entry[index].name = name;
        config.entry[index].priority = role.priority;
        config.entry[index].budget_us = role.budget_us;
    }
    config
}

/// Copy a schedule page into `dst`.
pub fn sched_to_bytes(value: &sched_config_t, dst: &mut [u8]) {
    assert_eq!(dst.len(), core::mem::size_of::<sched_config_t>());
    // SAFETY: `dst` is exactly one page, and `value` is initialised.
    unsafe {
        core::ptr::copy_nonoverlapping(
            core::ptr::from_ref(value).cast::<u8>(),
            dst.as_mut_ptr(),
            dst.len(),
        );
    }
}

/// Read a schedule page out of bytes that came from [`sched_to_bytes`].
///
/// # Safety
///
/// `bytes` must be a valid `sched_config_t`.
pub unsafe fn sched_from_bytes(bytes: &[u8]) -> sched_config_t {
    assert_eq!(bytes.len(), core::mem::size_of::<sched_config_t>());
    let mut value = core::mem::MaybeUninit::<sched_config_t>::uninit();
    unsafe {
        core::ptr::copy_nonoverlapping(
            bytes.as_ptr(),
            value.as_mut_ptr().cast::<u8>(),
            bytes.len(),
        );
        value.assume_init()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_round_trip_keeps_every_role() {
        let page = fs_image_schedule();
        let mut bytes = vec![0u8; core::mem::size_of::<sched_config_t>()];
        sched_to_bytes(&page, &mut bytes);
        let decoded = unsafe { sched_from_bytes(&bytes) };
        assert_eq!(decoded.count as usize, FS_IMAGE_ROLES.len());
        for (index, role) in FS_IMAGE_ROLES.iter().enumerate() {
            let name = decoded.entry[index]
                .name
                .split(|byte| *byte == 0)
                .next()
                .unwrap();
            assert_eq!(name, role.name.as_bytes());
            assert_eq!(decoded.entry[index].priority, role.priority);
            assert_eq!(decoded.entry[index].budget_us, role.budget_us);
        }
    }
}
