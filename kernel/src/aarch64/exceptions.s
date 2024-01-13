save_registers:
stp x0, x1, [sp, #0x00]
stp x2, x3, [sp, #0x10]
stp x4, x5, [sp, #0x20]
stp x6, x7, [sp, #0x30]
stp x8, x9, [sp, #0x40]
stp x10, x11, [sp, #0x50]
stp x12, x13, [sp, #0x60]
stp x14, x15, [sp, #0x70]
stp x16, x17, [sp, #0x80]
stp x18, x19, [sp, #0x90]
stp x20, x21, [sp, #0xa0]
stp x22, x23, [sp, #0xb0]
stp x24, x25, [sp, #0xc0]
stp x26, x27, [sp, #0xd0]
stp x28, x29, [sp, #0xe0]
str x30, [sp, #0xf0]
mrs x0, elr_el1
str x0, [sp, #0xf8]
ret

restore_registers:
ldp x0, x1, [sp, #0x00]
ldp x2, x3, [sp, #0x10]
ldp x4, x5, [sp, #0x20]
ldp x6, x7, [sp, #0x30]
ldp x8, x9, [sp, #0x40]
ldp x10, x11, [sp, #0x50]
ldp x12, x13, [sp, #0x60]
ldp x14, x15, [sp, #0x70]
ldp x16, x17, [sp, #0x80]
ldp x18, x19, [sp, #0x90]
ldp x20, x21, [sp, #0xa0]
ldp x22, x23, [sp, #0xb0]
ldp x24, x25, [sp, #0xc0]
ldp x26, x27, [sp, #0xd0]
ldp x28, x29, [sp, #0xe0]
ldr x30, [sp, #0xf0]
ret

.p2align 11
.globl exception_vector_table
exception_vector_table:
// For these first four, we don't bother saving anything since we expect not to return.
adr x0, sp0_synch
b invalid_vector
.p2align 7
adr x0, sp0_irq
b invalid_vector
.p2align 7
adr x0, sp0_fiq
b invalid_vector
.p2align 7
adr x0, sp0_serror
b invalid_vector
// The next four are actually going to be used, so we save the registers.
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl synchronous_vector
bl restore_registers
add sp, sp, #0x100
eret
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl irq_vector
bl restore_registers
add sp, sp, #0x100
eret
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl fiq_vector
bl restore_registers
add sp, sp, #0x100
eret
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl serror_vector
bl restore_registers
add sp, sp, #0x100
eret
// The next lot are the user mode vectors.
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl synchronous_vector_user
bl restore_registers
add sp, sp, #0x100
eret
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl irq_vector_user
bl restore_registers
add sp, sp, #0x100
eret
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl fiq_vector_user
bl restore_registers
add sp, sp, #0x100
eret
.p2align 7
sub sp, sp, #0x100
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl serror_vector_user
bl restore_registers
add sp, sp, #0x100
eret
// The last lot are for aarch32, which we don't support.
.p2align 7
adr x0, user32_synch
b invalid_vector
.p2align 7
adr x0, user32_irq
b invalid_vector
.p2align 7
adr x0, user32_fiq
b invalid_vector
.p2align 7
adr x0, user32_serror
b invalid_vector
