; Minimal QEMU loader for lerux kernel bring-up (x86_64).
; Loaded via QEMU -kernel (Multiboot2).
;
; Goals for v1 bring-up:
;   - Enter long mode with usable paging (identity low + higher half)
;   - Load the kernel ELF from a fixed physical address (via -device loader)
;     or fall back to multiboot module search.
;   - Construct a minimal but non-crashing KernelArgs
;   - Jump to kstart (virtual) with RDI = &KernelArgs
;
; This loader is development-only. We will replace it with a pure-Rust version later.

%define PAGE_SIZE 0x1000

; When using QEMU -device loader, we place the kernel at this fixed physical address.
; This is the most reliable method during bring-up.
%define KERNEL_PHYS_ADDR 0x200000

; -------------------- Multiboot v1 header (best compatibility with QEMU -kernel) --------------------
section .multiboot progbits alloc noexec nowrite align=4
    dd 0x1BADB002
    dd 0x00010003                ; flags: want modules + meminfo
    dd -(0x1BADB002 + 0x00010003)
    dd 0, 0, 0, 0, 0
    dd 0

; -------------------- 32-bit startup --------------------
section .text32 progbits alloc exec nowrite align=16
bits 32
global _start
_start:
    cli
    cld
    mov edi, ebx                 ; save multiboot2 info pointer

    ; Use a temporary stack
    mov esp, stack32_top

    ; === Build page tables (identity 0-1GiB + higher half) ===
    ; PML4[0] = PDPT_low (identity)
    mov eax, pdpt_low
    or  eax, 0x3
    mov [pml4], eax

    ; PML4[511] = PDPT_high
    mov eax, pdpt_high
    or  eax, 0x3
    mov [pml4 + 511*8], eax

    ; PDPT_low[0] = PD_low (huge pages)
    mov eax, pd_low
    or  eax, 0x3
    mov [pdpt_low], eax

    ; Fill PD_low with 512 * 2MiB huge pages (0-1GiB identity)
    mov ecx, 0
    mov eax, 0x83                   ; Present | Writable | Huge (PS)
.fill_pd:
    mov [pd_low + ecx*8], eax
    add eax, 0x200000
    inc ecx
    cmp ecx, 512
    jb .fill_pd

    ; Higher half mapping: map the same physical low 1GB at 0xffffffff80000000
    ; 0xffffffff80000000 lives in PML4[511], PDPT index 510
    mov eax, pd_low
    or  eax, 0x3
    mov [pdpt_high + 510*8], eax

    ; Load CR3
    mov eax, pml4
    mov cr3, eax

    ; Enable PAE + PGE
    mov eax, cr4
    or  eax, (1<<5) | (1<<7)
    mov cr4, eax

    ; Enable long mode in EFER
    mov ecx, 0xC0000080
    rdmsr
    or  eax, (1<<8)
    wrmsr

    ; Load our GDT (required before long jump)
    lgdt [gdt64.pointer]

    ; Enable paging + PE + WP
    mov eax, cr0
    or  eax, (1<<31) | (1<<16) | 1
    mov cr0, eax

    ; Far jump into 64-bit code (selector 0x08 = code64)
    jmp 0x08:long_mode_entry

; -------------------- 64-bit code --------------------
section .text64 progbits alloc exec nowrite align=16
bits 64
long_mode_entry:
    mov ax, 0x10
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax
    mov ss, ax

    mov rsp, stack64_top

    ; === Try to find and load the kernel ===
    ; Preferred path for development: kernel placed at fixed physical address
    ; via QEMU -device loader (see run.sh)
    mov rdi, KERNEL_PHYS_ADDR
    call try_load_kernel_at
    test rax, rax
    jnz .have_kernel

    ; Fallback: look for it as a multiboot module (older / other boot methods)
    call find_kernel_module
    test rax, rax
    jz .no_kernel

    mov rdi, rax
    call load_elf64_segments
    jmp .have_kernel

.have_kernel:
    ; Build minimal KernelArgs
    call build_minimal_kernel_args
    ; RDI now contains pointer to KernelArgs

    ; Jump to kernel virtual entry point (kstart)
    mov rax, [kernel_entry_va]
    jmp rax

.no_kernel:
    mov al, 'N'
    mov dx, 0x3f8
    out dx, al
    cli
    hlt

; -------------------- Helpers (64-bit) --------------------

; Returns physical address of first multiboot module in RAX, or 0
find_kernel_module:
    mov rbx, rdi                 ; multiboot info
    mov ecx, [rbx]               ; total_size
    add rcx, rbx

    add rbx, 8                   ; first tag

.tag_loop:
    cmp rbx, rcx
    jae .not_found

    mov eax, [rbx]
    mov edx, [rbx+4]

    cmp eax, 3                   ; MODULE tag
    je .found

    add rbx, rdx
    add rbx, 7
    and rbx, ~7
    jmp .tag_loop

.found:
    mov eax, [rbx+8]             ; mod_start
    ret
.not_found:
    xor eax, eax
    ret

; Try to load a kernel ELF from a known physical address.
; Input:  RDI = physical address to check
; Output: RAX = 0 on failure, non-zero on success (the address we used)
; Side effect: calls load_elf64_segments and fills kernel_entry_va on success
try_load_kernel_at:
    ; Check for ELF magic at this address
    cmp dword [rdi], 0x464C457F          ; \x7FELF
    jne .fail

    ; Looks like an ELF — try to load it
    push rdi
    call load_elf64_segments
    pop rax                              ; return the address we used
    ret

.fail:
    xor eax, eax
    ret

; rdi = physical ELF image
; Fills [kernel_entry_va]
load_elf64_segments:
    ; Check ELF magic
    cmp dword [rdi], 0x464C457F
    jne .bad

    ; e_entry (virtual)
    mov rax, [rdi + 0x18]
    mov [kernel_entry_va], rax

    ; Program headers
    mov r8,  [rdi + 0x20]        ; e_phoff
    movzx r9, word [rdi + 0x38]  ; e_phnum
    movzx r10, word [rdi + 0x36] ; e_phentsize

    add r8, rdi                  ; first phdr

.ph_loop:
    test r9, r9
    jz .done

    cmp dword [r8], 1            ; PT_LOAD
    jne .next

    ; src = image + p_offset
    mov rsi, rdi
    add rsi, [r8 + 8]

    ; dst = p_paddr (physical load address)
    mov rdx, [r8 + 0x10]         ; p_paddr

    ; filesz
    mov rcx, [r8 + 0x20]
    ; copy
    push rdi
    mov rdi, rdx
    rep movsb
    pop rdi

    ; zero bss (memsz - filesz)
    mov rcx, [r8 + 0x28]
    sub rcx, [r8 + 0x20]
    xor al, al
    ; rdi is now at p_paddr + filesz after the copy above? No — we restored it.
    ; Actually after pop rdi, rdi is the ELF base again. We need to zero starting at p_paddr + filesz
    mov rdi, rdx
    add rdi, [r8 + 0x20]
    rep stosb

.next:
    add r8, r10
    dec r9
    jmp .ph_loop

.done:
    ret

.bad:
    mov al, 'E'
    mov dx, 0x3f8
    out dx, al
    cli
    hlt

; Build a minimal KernelArgs at 0x20000
; Returns pointer in RDI
build_minimal_kernel_args:
    mov rdi, 0x20000
    ; zero the struct (we only need ~96 bytes)
    xor eax, eax
    mov rcx, 32
    rep stosd
    mov rdi, 0x20000

    ; kernel_base / kernel_size (rough)
    mov qword [rdi + 0],  0x100000
    mov qword [rdi + 8],  0x1000000     ; 16 MiB claim

    ; stack_base / stack_size (our loader stack area)
    mov qword [rdi + 16], 0x80000
    mov qword [rdi + 24], 0x40000

    ; env (empty)
    mov qword [rdi + 32], 0
    mov qword [rdi + 40], 0

    ; hwdesc (none for first boot)
    mov qword [rdi + 48], 0
    mov qword [rdi + 56], 0

    ; areas (memory map) - point to a small table we provide
    mov qword [rdi + 64], memory_map
    mov qword [rdi + 72], 128

    ; bootstrap (tiny region we reserve)
    mov qword [rdi + 80], 0x40000
    mov qword [rdi + 88], 0x10000

    ; Fill a minimal memory map (one big entry for 0-128MiB usable)
    mov rax, memory_map
    mov qword [rax + 0],  0x1000        ; base
    mov qword [rax + 8],  0x7F00000     ; ~127 MiB length
    mov qword [rax + 16], 1             ; type = usable (Redox convention is usually 1)

    ret

; -------------------- Data --------------------
section .data
align 8
kernel_entry_va: dq 0

; -------------------- GDT --------------------
section .rodata
align 16
gdt64:
    dq 0x0000000000000000          ; null
    dq 0x00AF9A000000FFFF          ; code64 (0x08)
    dq 0x00AF92000000FFFF          ; data64 (0x10)
gdt64.pointer:
    dw $ - gdt64 - 1
    dq gdt64

; -------------------- BSS (page tables + stacks) --------------------
section .bss
align 4096
pml4:        resb 4096
pdpt_low:    resb 4096
pd_low:      resb 4096
pdpt_high:   resb 4096

memory_map:  resb 4096

stack32_bottom: resb 4096
stack32_top:

stack64_bottom: resb 4096 * 4
stack64_top:
