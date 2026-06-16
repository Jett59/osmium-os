use alloc::vec;
use syscall_interface::{LogError, Syscall, SyscallResult};

use crate::user_memory::UserAddressSpaceHandle;

pub fn handle_syscall(syscall: Syscall, address_space: UserAddressSpaceHandle) -> SyscallResult {
    match syscall {
        Syscall::Log(arguments) => {
            let string_handle = address_space.memory(arguments.string_address, arguments.length);
            let mut bytes = vec![0u8; arguments.length];
            string_handle.read(bytes.as_mut_slice());
            let result = match core::str::from_utf8(&bytes) {
                Ok(string) => {
                    crate::println!("{}", string);
                    Ok(())
                }
                Err(utf8_error) => Err(LogError::InvalidUtf8 {
                    position: utf8_error.valid_up_to(),
                }),
            };
            SyscallResult::Log(result)
        }
    }
}
