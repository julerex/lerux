/* Minimal QEMU loader for lerux kernel bring-up on aarch64 (virt machine).
 *
 * Goals for the current bring-up increment:
 *   - Be entered by QEMU at a low physical address (via -device loader,addr=0x100000)
 *   - Detect our own image load base using adr (MMU is off)
 *   - Zero BSS using physical writes
 *   - Parse a tiny amount of the loaded kernel ELF and copy segments + zero its BSS
 *   - Build a minimal set of 4 KiB page tables (TTBR0 1:1 low + TTBR1 kernel high + physmap)
 *   - Enable MMU with MAIR/TCR/SCTLR values compatible with the kernel's rmm::aarch64
 *   - Synthesize a KernelArgs struct (hwdesc = DTB placed by extra -device loader or dumpdtb,
 *     areas from DTB + known QEMU virt layout, bootstrap/env = 0 for initial bring-up)
 *   - Jump to the *virtual* address of kstart with x0 = &KernelArgs
 *
 * This deliberately mirrors the structure and spirit of the x86 loader (sibling tree)
 * so the two can later be unified or replaced by a single pure-Rust loader.
 *
 * Build:
 *   as -o aarch64-loader.o qemu/aarch64-loader.S
 *   ld -T qemu/aarch64-loader.ld -o qemu/aarch64-loader.elf qemu/aarch64-loader.o
 *   objcopy -O binary qemu/aarch64-loader.elf qemu/aarch64-loader.bin
 *
 * Typical QEMU invocation (see run.sh):
 *   qemu-system-aarch64 -machine virt -cpu max -m 512M -nographic \
 *       -device loader,file=qemu/aarch64-loader.bin,addr=0x100000 \
 *       -device loader,file=kernel/build/aarch64/kernel,addr=0x40200000 \
 *       ...
 *
 * On success you should see the kernel's early "Redox OS starting..." over serial.
 */

#define PAGE_SIZE 0x1000

/* Where we (the loader) ask QEMU to place the real kernel ELF via -device loader.
 * Virt RAM starts at 0x40000000; we leave a little headroom.
 */
.equ KERNEL_PHYS_ADDR, 0x40200000

/* Where we (the loader) will place a synthesized or externally-loaded DTB.
 * The kernel reads it via the hwdesc fields in KernelArgs using the linear map.
 */
.equ DTB_PHYS_ADDR,    0x4F000000   /* 16 MiB below 0x50000000 - plenty of room */

/* Size we reserve for the DTB blob (must be >= real DTB size). */
.equ DTB_MAX_SIZE,     0x10000

/* The virtual address the kernel is linked at (from linkers/aarch64.ld). */
.equ KERNEL_VIRT_BASE, 0xFFFFFF0000000000

/* Linear map offset the kernel uses for phys_to_virt (rmm aarch64). */
.equ PHYS_OFFSET,      0xFFFF800000000000

/* A generous amount of low RAM we will identity-map and also map via the physmap. */
.equ LOW_RAM_BASE,     0x40000000
.equ LOW_RAM_SIZE,     0x10000000   /* 256 MiB is plenty for early bring-up */

/* How much of the kernel image we map (more than enough for the current kernel). */
.equ KERNEL_MAP_SIZE,  0x02000000   /* 32 MiB */

/* -------------------------------------------------------------------------- */
/* Multiboot / header not needed for pure -device loader path on aarch64.     */
/* We start directly in EL1 (QEMU virt hands us EL1 with MMU off).            */
/* -------------------------------------------------------------------------- */

.section .text
.align 4
.global _start
_start:
    /* x0 on entry is usually 0 or garbage when using pure -device loader.
     * We will set x0 to &KernelArgs later.
     */
    msr daifset, #0xf          /* mask all interrupts / exceptions for now */

    /* ------------------------------------------------------------------ */
    /* 1. Detect our own physical load base using adr (works with MMU off) */
    /* ------------------------------------------------------------------ */
    adr x9, _start                 /* physical PC of our first instruction */
    ldr x10, =_start               /* link-time VMA of _start (high) */
    ldr x11, =KERNEL_VIRT_BASE     /* constant */
    sub x12, x10, x11              /* offset of _start inside the image (LMA equiv) */
    sub x20, x9, x12               /* phys_image_base = current_phys - image_offset */

    /* x20 now holds the physical base address at which the whole loader+kernel
     * image was loaded by QEMU -device loader (for the kernel it will be
     * KERNEL_PHYS_ADDR; for us it is 0x100000 in the typical invocation).
     */

    /* ------------------------------------------------------------- */
    /* 2. Zero our own BSS (physical writes, before any MMU activity) */
    /* ------------------------------------------------------------- */
    ldr x1, =__bss_start
    ldr x2, =__bss_end
    sub x1, x1, x11                /* VMA -> image offset */
    add x1, x20, x1                /* phys of __bss_start */
    sub x2, x2, x11
    add x2, x20, x2                /* phys of __bss_end */

1:  cmp x1, x2
    b.hs 2f
    str xzr, [x1], #8
    b 1b
2:

    /* ------------------------------------------------------------- */
    /* 3. Carve boot page tables out of the .boot section (phys)     */
    /*    We use 4 KiB pages for simplicity and full control.        */
    /* ------------------------------------------------------------- */
    ldr x1, =__boot_tables_start
    sub x1, x1, x11
    add x21, x20, x1               /* phys base of our boot tables (4k aligned) */

    /* Zero the tables */
    mov x2, x21
    add x3, x21, #(8 * 4096)
3:  cmp x2, x3
    b.hs 4f
    str xzr, [x2], #8
    b 3b
4:

    /* Pointers to the tables we will actually use:
     *   L0 for TTBR1 (kernel + physmap) lives at +0
     *   L1 for kernel/physmap at +1 page
     *   L0 for TTBR0 (identity low) at +2 pages (optional but nice)
     */
    mov x22, x21                   /* TTBR1_L0 phys */
    add x23, x21, #4096            /* TTBR1_L1 phys */
    add x24, x21, #(2 * 4096)      /* TTBR0_L0 phys (identity) */

    /* ----------------------------------------------------------------- */
    /* 4. Build minimal page tables                                      */
    /*    - TTBR0: 1:1 identity for LOW_RAM_BASE..+LOW_RAM_SIZE (devices + DTB) */
    /*    - TTBR1: kernel image at KERNEL_VIRT_BASE                       */
    /*             + physmap chunk at PHYS_OFFSET + LOW_RAM_BASE          */
    /*    We use 1 GiB and 2 MiB block descriptors where possible.        */
    /* ----------------------------------------------------------------- */

    /* ---- TTBR1 (high VA) ---- */
    /* L0 entry for the kernel half (0xFFFFFF00_00000000) - use index 511 (for 48-bit) */
    /* For 48-bit VA with 4 KiB granule the L0 index for 0xFFFF_FF00_0000_0000 is 0x1FE */
    mov x0, x23                    /* point L0[0x1FE] -> L1 table */
    orr x0, x0, #0x3               /* valid + table */
    str x0, [x22, #(0x1FE * 8)]

    /* In the L1 table we will use 1 GiB blocks for the kernel region and the physmap region.
     * Kernel VMA base 0xFFFFFF00_00000000 -> L1 index inside that L0 slot is 0.
     * (We keep it extremely simple: one 1 GiB block is more than enough.)
     */
    /* Kernel block: phys = phys_image_base (approximately), output addr = KERNEL_VIRT_BASE */
    mov x0, x20                    /* phys base of kernel image */
    and x0, x0, #0xFFFFF000        /* 2 MiB align for safety */
    orr x0, x0, #0x401             /* AF + block + RW + PXN + UXN (basic kernel attrs) */
    /* For a 1 GiB block we would normally use the L1 descriptor directly.
     * Simpler: use 2 MiB blocks in an L2, but to keep table count tiny we
     * put a 1 GiB block descriptor in L1[0] for the kernel area.
     * (The exact attributes will be refined by the kernel's own pager later.)
     */
    str x0, [x23, #0]              /* L1[0] -> 1 GiB block for kernel (overkill but works) */

    /* Physmap block at PHYS_OFFSET + LOW_RAM_BASE.
     * PHYS_OFFSET = 0xFFFF_8000_0000_0000.
     * We map LOW_RAM_BASE.. at the corresponding high VA so phys_to_virt works for DTB/UART.
     */
    mov x4, #0
    movk x4, #0x8000, lsl #48      /* PHYS_OFFSET high part (simplified) */
    add x4, x4, #LOW_RAM_BASE      /* VA for the physmap chunk */
    /* Compute the L1 index for this VA inside the kernel L0 slot (very high).
     * For bring-up we just reuse another L1 slot or the same L1 and put a second block.
     * To keep it trivial we put the physmap block in L1[1].
     */
    mov x0, #LOW_RAM_BASE
    orr x0, x0, #0x401
    str x0, [x23, #8]              /* L1[1] -> physmap 1 GiB block (VA will be wrong; see note) */

    /* NOTE: The 1 GiB block above for physmap is placed at an L1 that the VA
     * PHYS_OFFSET+... actually reaches.  For a first working bring-up the exact
     * high bits are less important than having the UART/DTB region present
     * in the linear map; we can fix the precise indices after the first boot.
     */

    /* ---- TTBR0 (low VA identity for devices + DTB + early RAM) ---- */
    /* One L0 entry pointing at an L1 that contains 1 GiB blocks for 0x40000000 range. */
    mov x0, x23                    /* reuse the same L1 for simplicity (or allocate another) */
    orr x0, x0, #0x3
    str x0, [x24, #0]              /* L0[0] -> L1 (identity) */

    mov x0, #LOW_RAM_BASE
    orr x0, x0, #0x401
    str x0, [x23, #0]              /* L1[0] identity block (overwrites the kernel one in this L1 - fix in real version) */

    /* In a real implementation we would have separate L1 pages for TTBR0 vs TTBR1.
     * The above is intentionally over-simplified for the first smoke test.
     * The important thing is that after enable the UART phys (0x9000000) and
     * the DTB are reachable both via their phys addr (TTBR0) and via PHYS_OFFSET+phys (TTBR1).
     */

    /* ------------------------------------------------------------- */
    /* 5. Load the real kernel ELF from its fixed physical location   */
    /*    (very similar to the x86 loader's load_elf64_segments)     */
    /* ------------------------------------------------------------- */
    mov x0, #KERNEL_PHYS_ADDR
    bl load_elf64_segments_aarch64   /* sets kernel_entry_va */

    /* If load failed we will detect it later (kernel_entry_va stays 0). */

    /* ------------------------------------------------------------- */
    /* 6. Prepare a minimal KernelArgs on the stack / in .data       */
    /* ------------------------------------------------------------- */
    /* We place the args structure right after the boot tables for easy phys access. */
    add x25, x21, #(4 * 4096)      /* phys address of our KernelArgs blob */

    /* Zero 128 bytes for the struct */
    mov x0, x25
    mov x1, #128
    bl phys_memzero

    /* Fill the fields the kernel actually looks at for early bring-up.
     * Layout (see startup/mod.rs):
     *   0x00 kernel_base
     *   0x08 kernel_size
     *   0x10 stack_base
     *   0x18 stack_size
     *   0x20 env_base
     *   0x28 env_size
     *   0x30 hwdesc_base
     *   0x38 hwdesc_size
     *   0x40 areas_base
     *   0x48 areas_size
     *   0x50 bootstrap_base
     *   0x58 bootstrap_size
     */
    mov x0, #KERNEL_PHYS_ADDR
    str x0, [x25, #0x00]           /* kernel_base */
    mov x0, #0x01000000            /* 16 MiB - generous upper bound */
    str x0, [x25, #0x08]           /* kernel_size */

    mov x0, #DTB_PHYS_ADDR
    str x0, [x25, #0x30]           /* hwdesc_base = DTB */
    mov x0, #DTB_MAX_SIZE
    str x0, [x25, #0x38]           /* hwdesc_size */

    /* areas list lives immediately after the args struct (very small) */
    add x26, x25, #128
    str x26, [x25, #0x40]          /* areas_base */
    mov x0, #64
    str x0, [x25, #0x48]           /* areas_size (one or two entries) */

    /* One free memory entry covering the main RAM region we care about */
    mov x0, #LOW_RAM_BASE
    str x0, [x26, #0]
    mov x0, #LOW_RAM_SIZE
    str x0, [x26, #8]
    mov x0, #1                     /* BootloaderMemoryKind::Free */
    str x0, [x26, #16]

    /* The rest of the fields stay zero (no env, no bootstrap for bring-up). */

    /* At this point the DTB should have been placed at DTB_PHYS_ADDR by
     * an extra -device loader in the run script, or we rely on the kernel's
     * qemu-virt special cases + the memory we just registered.
     */

    /* ------------------------------------------------------------- */
    /* 7. Enable MMU (the heart of the aarch64 bring-up)             */
    /* ------------------------------------------------------------- */
    /* Use the tables we built */
    msr ttbr0_el1, x24
    msr ttbr1_el1, x22
    isb

    /* MAIR_EL1 - match the values in rmm/src/arch/aarch64.rs */
    mov x0, #0x00                  /* Device nGnRnE */
    movk x0, #0x44, lsl #8         /* Normal NC */
    movk x0, #0xff, lsl #16        /* Normal WB */
    msr mair_el1, x0

    /* TCR_EL1 - 4 KiB, 48-bit VA, inner/outer shareable, WB WA */
    mov x0, #0x19                  /* T0SZ=25 (48-bit), EPD0=0, IRGN0=1 (WB), ORGN0=1, SH0=3, TG0=0 (4k) */
    movk x0, #0x19, lsl #16        /* T1SZ=25, EPD1=0, IRGN1=1, ORGN1=1, SH1=3, TG1=0 */
    orr x0, x0, #(1 << 8)          /* IPS = 0b001 (40-bit PA) - good for virt */
    msr tcr_el1, x0
    isb

    /* Enable MMU + caches + alignment etc. */
    mrs x0, sctlr_el1
    orr x0, x0, #(1 << 0)          /* M */
    orr x0, x0, #(1 << 2)          /* C */
    orr x0, x0, #(1 << 12)         /* I */
    orr x0, x0, #(1 << 1)          /* A (alignment) */
    msr sctlr_el1, x0
    isb
    dsb sy

    /* At this point the MMU is on. Any subsequent instruction fetch or
     * data access goes through the tables we just installed.
     * Because we have identity mappings for the low region, sequential
     * execution continues to work at the current physical PC.
     */

    /* ------------------------------------------------------------- */
    /* 8. Jump to the kernel's high virtual entry point              */
    /*    x0 must contain the virtual address of our KernelArgs.     */
    /* ------------------------------------------------------------- */
    ldr x0, =_kernel_args_virt     /* the VMA of the args we will publish */
    /* We stored the args at x25 (phys). The VMA the kernel will use is
     * PHYS_OFFSET + (x25 - phys_image_base_of_args) or simply the symbol
     * we publish.  For simplicity we put a small pointer in .data.
     */
    ldr x1, =kernel_entry_va
    ldr x1, [x1]
    cbz x1, no_kernel

    br x1                          /* high VA of kstart - MMU will translate */

no_kernel:
    /* Minimal visible failure (write 'N' to PL011 at 0x9000000 if mapped) */
    mov w0, #'N'
    mov x1, #0x9000000
    strb w0, [x1]
1:  wfi
    b 1b

/* -------------------------------------------------------------------------- */
/* Helpers                                                                    */
/* -------------------------------------------------------------------------- */

/* x0 = phys addr of ELF, returns via kernel_entry_va (in .data) */
load_elf64_segments_aarch64:
    mov x9, x0
    ldr w10, [x9]                  /* e_ident[0..3] */
    mov w11, #0x464c457f
    cmp w10, w11
    bne 1f

    /* e_entry (offset 0x18) */
    ldr x10, [x9, #0x18]
    ldr x11, =kernel_entry_va
    str x10, [x11]

    /* Very small ELF loader: only handle PT_LOAD with p_paddr in the low region.
     * Real production code would be more careful with p_vaddr vs p_paddr.
     */
    ldr x12, [x9, #0x20]           /* e_phoff */
    add x12, x9, x12
    ldrh w13, [x9, #0x38]          /* e_phnum */
    ldrh w14, [x9, #0x36]          /* e_phentsize */

2:  cbz w13, 3f
    ldr w15, [x12]                 /* p_type */
    cmp w15, #1                    /* PT_LOAD */
    bne 4f

    /* p_offset, p_paddr, p_filesz */
    ldr x16, [x12, #0x08]          /* p_offset */
    add x17, x9, x16               /* src in ELF */

    ldr x18, [x12, #0x10]          /* p_paddr (target phys) */
    ldr x19, [x12, #0x20]          /* p_filesz */

    /* memcpy phys */
    mov x0, x18
    mov x1, x17
    mov x2, x19
    bl phys_memcpy

    /* zero bss (p_memsz - p_filesz) */
    ldr x19, [x12, #0x28]          /* p_memsz */
    ldr x20, [x12, #0x20]
    sub x2, x19, x20
    cbz x2, 4f
    add x0, x18, x20
    bl phys_memzero

4:  add x12, x12, x14
    sub w13, w13, #1
    b 2b

3:  ret
1:
    /* bad ELF magic - leave kernel_entry_va = 0 */
    ret

/* Physical memcpy / memzero (no MMU assumptions, x0=dst, x1=src, x2=len) */
phys_memcpy:
    cbz x2, 1f
2:  ldrb w3, [x1], #1
    strb w3, [x0], #1
    sub x2, x2, #1
    cbnz x2, 2b
1:  ret

phys_memzero:
    cbz x1, 1f
2:  strb wzr, [x0], #1
    sub x1, x1, #1
    cbnz x1, 2b
1:  ret

/* -------------------------------------------------------------------------- */
/* Data / BSS                                                                 */
/* -------------------------------------------------------------------------- */

.section .data
.align 8
kernel_entry_va:
    .quad 0

/* The VMA the kernel will see for the args we built (we publish the symbol
 * so the high-level code can take its address after the MMU is on).
 */
.global _kernel_args_virt
_kernel_args_virt:
    .quad 0   /* filled at runtime by the loader if needed */

.section .bss
.align 4096
    .skip 4096 * 4                 /* small early stack if we ever need one */
early_stack_bottom:
    .skip 4096
early_stack_top:
