// Sets up the data structure for the exception handlers to interpret.
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
mrs x0, sp_el0
stp x30, x0, [sp, #0xf0]
mrs x0, elr_el1
mrs x1, spsr_el1
stp x0, x1, [sp, #0x100]
ret

restore_registers_and_eret:
ldp x0, x1, [sp, #0x100]
msr spsr_el1, x1
msr elr_el1, x0
ldp x30, x0, [sp, #0xf0]
msr sp_el0, x0

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
add sp, sp, #0x110
eret

.p2align 11
.globl exception_vector_table
exception_vector_table:
// This table is structured as 16 functions, each of which is 128 bytes long (32 instructions).

// The first four are for system interrupts with a user stack, which we don't use.
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
// The next four are system exceptions with kernel stack, which is what we use.
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl synchronous_vector
b restore_registers_and_eret

.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl irq_vector
b restore_registers_and_eret
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl fiq_vector
b restore_registers_and_eret
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl serror_vector
b restore_registers_and_eret
// The next lot are the user mode vectors in aarch64 mode.
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl synchronous_vector_user
b restore_registers_and_eret
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl irq_vector_user
b restore_registers_and_eret
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl fiq_vector_user
b restore_registers_and_eret
.p2align 7
sub sp, sp, #0x110
bl save_registers
mov x0, sp // Passing the registers as the first argument.
bl serror_vector_user
b restore_registers_and_eret
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
