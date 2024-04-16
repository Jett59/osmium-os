use core::arch::asm;

use crate::{encode_syscall, LogArguments, RegisterValues, Syscall, SyscallNumber};

#[inline(always)]
fn issue_syscall<const NUMBER: u16>(registers: RegisterValues) -> RegisterValues {
    let mut result = RegisterValues::default();
    unsafe {
        #[cfg(target_arch = "x86_64")]
        asm!(
            "syscall",
            inout("rax") NUMBER => _,
            inout("rdi") registers.rdi => result.rdi,
            inout("rsi") registers.rsi => result.rsi,
            inout("rdx") registers.rdx => result.rdx,
            inout("r10") registers.r10 => result.r10,
            inout("r8") registers.r8 => result.r8,
            inout("r9") registers.r9 => result.r9,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack)
        );
        #[cfg(target_arch = "aarch64")]
        asm!(
            "svc {number}",
            number = const NUMBER,
            inout("x0") registers.x0 => result.x0,
            inout("x1") registers.x1 => result.x1,
            inout("x2") registers.x2 => result.x2,
            inout("x3") registers.x3 => result.x3,
            inout("x4") registers.x4 => result.x4,
            inout("x5") registers.x5 => result.x5,
            options(nostack)
        );
    }
    result
}

#[inline(always)]
pub fn log(message: &str) {
    let message_bytes = message.as_bytes();
    let (_, register_values) = encode_syscall(Syscall::Log(LogArguments {
        string_address: message_bytes.as_ptr() as usize,
        length: message_bytes.len(),
    }));
    issue_syscall::<{ SyscallNumber::Log.as_integer() }>(register_values);
}
