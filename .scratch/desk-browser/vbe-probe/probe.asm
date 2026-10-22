; Multiboot 2 probe. Limine sets the requested VESA mode, then this
; records the framebuffer tag and paints a pattern.

bits 32
section .multiboot
align 8
header_start:
    dd 0xe85250d6
    dd 0
    dd header_end - header_start
    dd -(0xe85250d6 + (header_end - header_start))
align 8
    dw 5
    dw 0
    dd 20
    dd 1024
    dd 768
    dd 32
align 8
    dw 0
    dw 0
    dd 8
header_end:

section .text
global _start
_start:
    cli
    cld
    mov esp, stack_top
    push eax
    push ebx
    call serial_init
    mov esi, banner
    call serial_str
    pop ebx
    pop eax

    cmp eax, 0x36d76289
    je .magic_ok
    mov dword [record + 8], 2
    jmp .finish
.magic_ok:
    mov esi, ebx
    mov ecx, [esi]
    add ecx, esi
    add esi, 8
.tag:
    cmp esi, ecx
    jae .no_fb
    mov eax, [esi]
    mov edx, [esi + 4]
    cmp eax, 8
    je .have_fb
    test edx, edx
    jz .no_fb
    add esi, edx
    add esi, 7
    and esi, ~7
    jmp .tag

.no_fb:
    mov dword [record + 8], 3
    jmp .finish

.have_fb:
    push esi
    push edx
    mov edi, record + 40
    mov ecx, 48
    cmp edx, ecx
    cmovb ecx, edx
    rep movsb
    pop edx
    pop esi
    mov eax, [esi + 8]
    mov [record + 12], eax
    mov eax, [esi + 12]
    mov [record + 16], eax
    mov eax, [esi + 16]
    mov [record + 20], eax
    mov eax, [esi + 20]
    mov [record + 24], eax
    mov eax, [esi + 24]
    mov [record + 28], eax
    mov al, [esi + 28]
    mov [record + 32], al
    mov al, [esi + 29]
    mov [record + 33], al
    cmp byte [esi + 29], 1
    jne .not_direct
    mov ax, [esi + 32]
    mov [record + 34], ax
    mov ax, [esi + 34]
    mov [record + 36], ax
    mov ax, [esi + 36]
    mov [record + 38], ax
    mov dword [record + 8], 1
    call paint
    jmp .report
.not_direct:
    mov dword [record + 8], 4
.report:
    call serial_report
.finish:
    mov esi, record
    mov edi, 0x6000
    mov ecx, 128
    rep movsd
    mov ecx, 0x04000000
.delay:
    dec ecx
    jnz .delay
    mov esi, stub_bin
    mov edi, 0x8000
    mov ecx, stub_len
    rep movsb
    jmp 0x8000

paint:
    mov eax, [record + 16]
    test eax, eax
    jnz .skip
    mov ebx, [record + 12]
    test ebx, ebx
    jz .skip
    mov ecx, [record + 28]
    test ecx, ecx
    jz .skip
    mov esi, 0
.row:
    mov edi, 0
.col:
    mov eax, edi
    shl eax, 2
    xor edx, edx
    div dword [record + 24]
    mov eax, edx
    and eax, 3
    mov edx, 0x000000ff
    cmp eax, 1
    jne .c1
    mov edx, 0x0000ff00
.c1:
    cmp eax, 2
    jne .c2
    mov edx, 0x00ff0000
.c2:
    cmp eax, 3
    jne .c3
    mov edx, 0x00ffffff
.c3:
    call pack
    mov eax, [record + 32]
    cmp al, 32
    jb .p24
    mov [ebx], edx
    add ebx, 4
    jmp .nextcol
.p24:
    cmp al, 24
    jb .p16
    mov [ebx], dx
    shr edx, 16
    mov [ebx + 2], dl
    add ebx, 3
    jmp .nextcol
.p16:
    mov [ebx], dx
    add ebx, 2
.nextcol:
    inc edi
    cmp edi, [record + 24]
    jb .col
    mov eax, [record + 20]
    mov edx, edi
    movzx ecx, byte [record + 32]
    shr cl, 3
    imul edx, ecx
    sub eax, edx
    add ebx, eax
    inc esi
    cmp esi, [record + 28]
    jb .row
.skip:
    ret

; EDX = 00RRGGBB logical color. Returns packed pixel in EDX.
pack:
    push eax
    push ebx
    push ecx
    xor ebx, ebx
    mov eax, edx
    and eax, 0xff
    mov cl, [record + 38]
    shl eax, cl
    or ebx, eax
    mov eax, edx
    shr eax, 8
    and eax, 0xff
    mov cl, [record + 36]
    shl eax, cl
    or ebx, eax
    mov eax, edx
    shr eax, 16
    and eax, 0xff
    mov cl, [record + 34]
    shl eax, cl
    or ebx, eax
    mov edx, ebx
    pop ecx
    pop ebx
    pop eax
    ret

serial_init:
    mov dx, 0x3f8 + 1
    xor al, al
    out dx, al
    mov dx, 0x3f8 + 3
    mov al, 0x80
    out dx, al
    mov dx, 0x3f8
    mov al, 1
    out dx, al
    mov dx, 0x3f8 + 1
    xor al, al
    out dx, al
    mov dx, 0x3f8 + 3
    mov al, 0x03
    out dx, al
    mov dx, 0x3f8 + 2
    mov al, 0xc7
    out dx, al
    mov dx, 0x3f8 + 4
    mov al, 0x0b
    out dx, al
    ret

serial_report:
    mov esi, msg_status
    call serial_str
    mov eax, [record + 8]
    call serial_hex32
    mov esi, msg_addr
    call serial_str
    mov eax, [record + 16]
    call serial_hex32
    mov eax, [record + 12]
    call serial_hex32
    mov esi, msg_pitch
    call serial_str
    mov eax, [record + 20]
    call serial_hex32
    mov esi, msg_size
    call serial_str
    mov eax, [record + 24]
    call serial_hex32
    mov al, 'x'
    call serial_byte
    mov eax, [record + 28]
    call serial_hex32
    mov esi, msg_bpp
    call serial_str
    movzx eax, byte [record + 32]
    call serial_hex32
    mov esi, msg_masks
    call serial_str
    mov al, [record + 35]
    call serial_hex8
    mov al, '@'
    call serial_byte
    mov al, [record + 34]
    call serial_hex8
    mov al, ' '
    call serial_byte
    mov al, [record + 37]
    call serial_hex8
    mov al, '@'
    call serial_byte
    mov al, [record + 36]
    call serial_hex8
    mov al, ' '
    call serial_byte
    mov al, [record + 39]
    call serial_hex8
    mov al, '@'
    call serial_byte
    mov al, [record + 38]
    call serial_hex8
    mov al, 10
    call serial_byte
    ret

serial_str:
    lodsb
    test al, al
    jz .done
    call serial_byte
    jmp serial_str
.done:
    ret

serial_hex32:
    push eax
    shr eax, 16
    call serial_hex16
    pop eax
serial_hex16:
    push eax
    shr eax, 8
    call serial_hex8
    pop eax
serial_hex8:
    push eax
    shr al, 4
    call serial_nybble
    pop eax
serial_nybble:
    and al, 0x0f
    add al, '0'
    cmp al, '9'
    jbe serial_byte
    add al, 'a' - '0' - 10
serial_byte:
    push edx
    push eax
    mov dx, 0x3f8 + 5
.wait:
    in al, dx
    test al, 0x20
    jz .wait
    pop eax
    mov dx, 0x3f8
    out dx, al
    pop edx
    ret

banner: db "vbe-probe start", 10, 0
msg_status: db "status ", 0
msg_addr: db " addr ", 0
msg_pitch: db " pitch ", 0
msg_size: db " size ", 0
msg_bpp: db " bpp ", 0
msg_masks: db " masks ", 0

section .data
record:
    db "LERUXVBE"
    times 504 db 0

stub_bin:
    incbin "rmstub.bin"
stub_len equ $ - stub_bin

section .bss
align 16
stack_bottom:
    resb 0x4000
stack_top:
