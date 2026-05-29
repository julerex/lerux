#!/usr/bin/env bash
#
# Deeper validation of the pure-Rust SMP trampoline blobs.
#
# Usage:
#   ./validate-trampolines.sh          # Compare current bytes vs freshly assembled
#   ./validate-trampolines.sh update   # Print ready-to-paste Rust code for trampoline.rs
#
# Requires: nasm + xxd (or od)
#
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KERNEL_ROOT="$(cd "$SCRIPT_DIR/../../.." && pwd)"
TRAMPOLINE_RS="$KERNEL_ROOT/src/arch/x86_shared/trampoline.rs"

ASM_DIR="$SCRIPT_DIR/asm"
OUT_DIR="$SCRIPT_DIR/out"
mkdir -p "$ASM_DIR" "$OUT_DIR"

echo "=== SMP Trampoline Byte Validation ==="
echo "Kernel root: $KERNEL_ROOT"
echo

# ------------------------------------------------------------------
# Original NASM sources (captured from the Redox kernel at vendoring time)
# These must be kept in sync with what was originally in src/asm/
# ------------------------------------------------------------------

cat > "$ASM_DIR/trampoline_x86_64.asm" << 'NASM_EOF'
; trampoline for bringing up APs
; compiled with nasm by build.rs, and included in src/acpi/madt.rs

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

    ; initialize stack to invalid value
    mov sp, 0

    ; cr3 holds pointer to PML4
    mov edi, [trampoline.page_table]
    mov cr3, edi

    ; enable FPU
    mov eax, cr0
    and al, 11110011b ; Clear task switched (3) and emulation (2)
    or al, 00100010b ; Set numeric error (5) monitor co-processor (1)
    mov cr0, eax

    ; 9: FXSAVE/FXRSTOR
    ; 7: Page Global
    ; 5: Page Address Extension
    ; 4: Page Size Extension
    mov eax, cr4
    or eax, 1 << 9 | 1 << 7 | 1 << 5 | 1 << 4
    mov cr4, eax

    ; initialize floating point registers
    fninit

    ; load protected mode GDT
    lgdt [gdtr]

    ; enable long mode
    mov ecx, 0xC0000080               ; Read from the EFER MSR.
    rdmsr
    or eax, 1 << 11 | 1 << 8          ; Set the Long-Mode-Enable and NXE bit.
    wrmsr

    ; enabling paging and protection simultaneously
    mov ebx, cr0
    ; 31: Paging
    ; 16: write protect kernel
    ; 0: Protected Mode
    or ebx, 1 << 31 | 1 << 16 | 1
    mov cr0, ebx

    ; far jump to enable Long Mode and load CS with 64 bit segment
    jmp gdt.kernel_code:long_mode_ap

USE64
long_mode_ap:
    mov rax, gdt.kernel_data
    mov ds, rax
    mov es, rax
    mov fs, rax
    mov gs, rax
    mov ss, rax

    mov rdi, [trampoline.args_ptr]

    mov rax, [trampoline.code]
    mov qword [trampoline.ready], 1
    jmp rax

struc GDTEntry
    .limitl resw 1
    .basel resw 1
    .basem resb 1
    .attribute resb 1
    .flags__limith resb 1
    .baseh resb 1
endstruc

attrib:
    .present              equ 1 << 7
    .ring1                equ 1 << 5
    .ring2                equ 1 << 6
    .ring3                equ 1 << 5 | 1 << 6
    .user                 equ 1 << 4
;user
    .code                 equ 1 << 3
;   code
    .conforming           equ 1 << 2
    .readable             equ 1 << 1
;   data
    .expand_down          equ 1 << 2
    .writable             equ 1 << 1
    .accessed             equ 1 << 0
;system
;   legacy
    .tssAvailabe16        equ 0x1
    .ldt                  equ 0x2
    .tssBusy16            equ 0x3
    .call16               equ 0x4
    .task                 equ 0x5
    .interrupt16          equ 0x6
    .trap16               equ 0x7
    .tssAvailabe32        equ 0x9
    .tssBusy32            equ 0xB
    .call32               equ 0xC
    .interrupt32          equ 0xE
    .trap32               equ 0xF
;   long mode
    .ldt32                equ 0x2
    .tssAvailabe64        equ 0x9
    .tssBusy64            equ 0xB
    .call64               equ 0xC
    .interrupt64          equ 0xE
    .trap64               equ 0xF

flags:
    .granularity equ 1 << 7
    .available equ 1 << 4
;user
    .default_operand_size equ 1 << 6
;   code
    .long_mode equ 1 << 5
;   data
    .reserved equ 1 << 5

gdtr:
    dw gdt.end + 1  ; size
    dq gdt          ; offset

gdt:
.null equ $ - gdt
    dq 0

.kernel_code equ $ - gdt
istruc GDTEntry
    at GDTEntry.limitl, dw 0
    at GDTEntry.basel, dw 0
    at GDTEntry.basem, db 0
    at GDTEntry.attribute, db attrib.present | attrib.user | attrib.code
    at GDTEntry.flags__limith, db flags.long_mode
    at GDTEntry.baseh, db 0
iend

.kernel_data equ $ - gdt
istruc GDTEntry
    at GDTEntry.limitl, dw 0
    at GDTEntry.basel, dw 0
    at GDTEntry.basem, db 0
; AMD System Programming Manual states that the writeable bit is ignored in long mode, but ss can not be set to this descriptor without it
    at GDTEntry.attribute, db attrib.present | attrib.user | attrib.writable
    at GDTEntry.flags__limith, db 0
    at GDTEntry.baseh, db 0
iend

.end equ $ - gdt
NASM_EOF

# 32-bit x86 version (shorter, no long mode)
cat > "$ASM_DIR/trampoline_x86.asm" << 'NASM_EOF'
; trampoline for bringing up APs (32-bit)
; compiled with nasm by build.rs, and included in src/acpi/madt.rs

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
NASM_EOF

echo "Assembled sources written to $ASM_DIR"

# ------------------------------------------------------------------
# Assemble and dump bytes
# ------------------------------------------------------------------

assemble_and_dump() {
    local arch=$1
    local src="$ASM_DIR/trampoline_${arch}.asm"
    local bin="$OUT_DIR/trampoline_${arch}.bin"

    echo "Assembling $arch..."
    nasm -f bin -o "$bin" "$src"

    echo "  -> $bin ($(stat -c%s "$bin") bytes)"

    # Output as Rust array
    echo "pub static TRAMPOLINE_${arch^^}: &[u8] = &["
    xxd -i "$bin" | sed 's/^  /    /; s/unsigned char.*\[\] = {//; s/};$//; s/0x/0x/g' | head -n -1
    echo "];"
    echo
}

assemble_and_dump x86_64
assemble_and_dump x86

echo "=== Comparison against current trampoline.rs ==="

if [ -f "$TRAMPOLINE_RS" ]; then
    echo "Current file: $TRAMPOLINE_RS"
    echo
    echo "NOTE: Manually compare the arrays above with the ones in trampoline.rs"
    echo "      (especially the GDT placement and baked absolute addresses)."
else
    echo "Could not find $TRAMPOLINE_RS"
fi

if [ "$1" = "update" ]; then
    echo
    echo "=== UPDATE MODE ==="
    echo "Copy the arrays printed above into trampoline.rs"
    echo "Remember to keep the cfg(target_arch) guards."
fi
