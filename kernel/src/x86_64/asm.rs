use core::arch::asm;

/// # Safety
/// This function could cause undefined behavior if the port does something strange.
/// For example, a port-mapped DMA controller could overwrite parts of the kernel.
pub unsafe fn write_port8(port: u16, value: u8) {
    asm!("out dx, al", in("dx") port, in("al") value, options(nomem, nostack));
}

pub fn io_wait() {
    unsafe {
        asm!("out dx, al", in("dx") 0x80, in("al") 0u8, options(nomem, nostack));
    }
}

/// # Safety
/// This could break code in the surrounding scope which relies on interrupts being disabled.
pub unsafe fn enable_interrupts() {
    asm!("sti", options(nomem, nostack));
}

pub unsafe fn iret(
    stack_segment: u64,
    stack_pointer: u64,
    flags: u64,
    code_segment: u64,
    instruction_pointer: u64,
) -> ! {
    asm!(
        "push {}",
        "push {}",
        "push {}",
        "push {}",
        "push {}",
        "iretq",
        in(reg) stack_segment, in(reg) stack_pointer, in(reg) flags, in(reg) code_segment, in(reg) instruction_pointer, options(nomem, nostack, noreturn));
}

pub const USER_CODE_SELECTOR: u16 = 0x23;
pub const USER_DATA_SELECTOR: u16 = 0x1b;

pub unsafe fn load_task_state_segment(selector: u16) {
    asm!("ltr ax", in("ax") selector, options(nomem, nostack));
}

pub unsafe fn read_msr(msr: u32) -> u64 {
    let mut low: u32;
    let mut high: u32;
    asm!("rdmsr", in("ecx") msr, out("eax") low, out("edx") high, options(nomem, nostack));
    ((high as u64) << 32) | low as u64
}

pub unsafe fn write_msr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;
    asm!("wrmsr", in("ecx") msr, in("eax") low, in("edx") high, options(nomem, nostack));
}

/// Stores the CS and SS selectors for kernel mode and user mode.
/// The SYSCALL and SYSRET instructions use these selectors when changing privilege levels.
/// Bits 63:48 give the user mode selectors, where SS is set to this field + 8 and CS to this field + 16.
/// Bits 47:32 give the kernel mode selectors, where CS is set to the value of this field and SS is set to this field + 8.
pub const STAR_MSR: u32 = 0xC0000081;
/// Holds the entrypoint address of the 64-bit syscall handler.
pub const LSTAR_MSR: u32 = 0xC0000082;
/// Contains the RFLAGS mask for the syscall instruction.
/// Every set bit in this mask clears the corresponding flag in RFLAGS.
pub const SFMASK_MSR: u32 = 0xC0000084;

/// Stores the base address for the GS segment register.
pub const GS_BASE_MSR: u32 = 0xC0000101;
/// Stores the alternate base address for the GS segment register.
/// The SWAPGS instruction exchanges the base address of the GS register with this value, allowing us to locate a stack in the syscall entrypoint.
pub const ALTERNATE_GS_BASE_MSR: u32 = 0xC0000102;
