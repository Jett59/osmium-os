use alloc::vec;
use syscall_interface::{Syscall, SyscallResult};

use crate::user_memory::UserAddressSpaceHandle;

pub fn handle_syscall(syscall: Syscall, address_space: UserAddressSpaceHandle) -> SyscallResult {
    match syscall {
        Syscall::Log(arguments) => {
            let string_handle = address_space.memory(arguments.string_address, arguments.length);
            let mut bytes = vec![0u8; arguments.length];
            string_handle.read(bytes.as_mut_slice());
            let string = core::str::from_utf8(&bytes).unwrap();
            crate::println!("{}", string);
            SyscallResult::None
        }
    }
}
