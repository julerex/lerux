; Real-mode tail of the mode-info probe. Copied to physical 0x8000.
; Scans BIOS hard disks for the marker sector and writes the record there.

org 0x8000

LBA equ 20480
RECORD equ 0x6000
SECTOR equ 0x1000

bits 32
stub32:
    cli
    lgdt [gdtr]
    jmp 0x18:stub16

bits 16
stub16:
    mov ax, 0x20
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov eax, cr0
    and al, 0xfe
    mov cr0, eax
    jmp 0:real_entry

real_entry:
    xor ax, ax
    mov ds, ax
    mov es, ax
    mov ss, ax
    mov sp, 0x7000
    sti
    mov dl, 0x80
.next_drive:
    push dx
    mov ax, SECTOR
    call read_sector
    jc .not_this
    mov si, SECTOR
    mov di, marker
    mov cx, 8
    repe cmpsb
    jne .not_this
    mov si, RECORD
    mov di, SECTOR
    mov cx, 256
    rep movsw
    pop dx
    mov ax, SECTOR
    call write_sector
    jmp .reboot
.not_this:
    pop dx
    inc dl
    cmp dl, 0x88
    jb .next_drive
.reboot:
    mov al, 0xfe
    out 0x64, al
    cli
.hang:
    hlt
    jmp .hang

; IN: DL drive, AX buffer segment-offset (offset only, segment 0)
read_sector:
    mov bx, ax
    mov ah, 0x42
    jmp do_disk
write_sector:
    mov bx, ax
    mov ah, 0x43
    mov al, 0
do_disk:
    mov word [dap], 0x0010
    mov word [dap + 2], 1
    mov word [dap + 4], bx
    mov word [dap + 6], 0
    mov dword [dap + 8], LBA
    mov dword [dap + 12], 0
    mov si, dap
    int 0x13
    ret

marker: db "LERUXVBE"
dap: times 16 db 0

align 8
gdt:
    dq 0
    dw 0xffff, 0x0000, 0x9a00, 0x00cf
    dw 0xffff, 0x0000, 0x9200, 0x00cf
    dw 0xffff, 0x0000, 0x9a00, 0x0000
    dw 0xffff, 0x0000, 0x9200, 0x0000
gdt_end:

gdtr:
    dw gdt_end - gdt - 1
    dd gdt
