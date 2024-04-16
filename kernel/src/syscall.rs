use syscall_interface::{Syscall, SyscallResult};

pub fn handle_syscall(syscall: Syscall) -> SyscallResult {
    crate::println!("Syscall: {:?}", syscall);
    match syscall {
        Syscall::Log(arguments) => {
            // TODO
            SyscallResult::None
        }
    }
}
