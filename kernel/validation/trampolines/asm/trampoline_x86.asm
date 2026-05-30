; SMP AP bring-up trampoline for 32-bit x86 (i586).
; Source of truth for kernel/src/arch/x86_shared/trampoline.rs (x86 array).
; Assembled with: nasm -f bin -o trampoline_x86.bin trampoline_x86.asm
; Captured from redox-os/kernel src/asm/x86/trampoline.asm at vendoring time.

ORG 0x8000
SECTION .text
USE16

trampoline:
    jmp short startup_ap
    times 8 - ($ - trampoline) nop
    .ready: dq 0
    .args_ptr: dq 0
    .page_table: dq 0
    .code: dq 0

startup_ap:
    cli

    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax

    mov sp, 0

    mov edi, [trampoline.page_table]
    mov cr3, edi

    mov eax, cr0
    and al, 11110011b
    or al, 00100010b
    mov cr0, eax

    mov eax, cr4
    or eax, 1 << 9 | 1 << 7 | 1 << 5 | 1 << 4
    mov cr4, eax

    fninit

    lgdt [gdtr]

    mov ebx, cr0
    or ebx, 1 << 31 | 1 << 16 | 1
    mov cr0, ebx

    jmp gdt.kernel_code:startup_ap32

USE32
startup_ap32:
    mov eax, gdt.kernel_data
    mov ds, eax
    mov es, eax
    mov fs, eax
    mov gs, eax
    mov ss, eax

    mov edi, [trampoline.args_ptr]

    mov eax, [trampoline.code]
    mov dword [trampoline.ready], 1
    jmp eax

; GDT (32-bit version)
gdtr:
    dw gdt.end + 1
    dd gdt

gdt:
.null equ $ - gdt
    dq 0

.kernel_code equ $ - gdt
    dw 0xffff
    dw 0
    db 0
    db 0x9a
    db 0xcf
    db 0

.kernel_data equ $ - gdt
    dw 0xffff
    dw 0
    db 0
    db 0x92
    db 0xcf
    db 0

.end equ $ - gdt
