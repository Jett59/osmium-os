#![no_std]
#![feature(asm_const)]

use core::{
    fmt::{self, Debug, Formatter},
    mem::{size_of, transmute},
};

mod log;

pub use log::*;

pub mod user;

#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SyscallNumber {
    /// Append the given string to the kernel log.
    Log = 0,
    #[doc(hidden)]
    _Max,
}

impl SyscallNumber {
    pub const fn as_integer(self) -> u16 {
        self as u16
    }
}

#[cfg(target_arch = "x86_64")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RegisterValues {
    pub rdi: u64,
    pub rsi: u64,
    pub rdx: u64,
    pub r10: u64,
    pub r8: u64,
    pub r9: u64,
}
#[cfg(target_arch = "aarch64")]
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct RegisterValues {
    pub x0: u64,
    pub x1: u64,
    pub x2: u64,
    pub x3: u64,
    pub x4: u64,
    pub x5: u64,
}

#[repr(C)]
union EncodedArguments {
    // every field *must* consist only of values that are safe to transmute from `usize`.
    // IMPORTANT: This even excludes pointers: https://github.com/rust-lang/unsafe-code-guidelines/issues/286.
    log_arguments: LogArguments,
    register_values: RegisterValues,
}

// Compile-time assertion that no syscall has too many arguments.
const _: () = assert!(size_of::<EncodedArguments>() == size_of::<RegisterValues>());

#[derive(Debug, Clone, Copy)]
pub enum Syscall {
    Log(LogArguments),
}

#[inline]
pub fn encode_syscall(syscall: Syscall) -> (u16, RegisterValues) {
    let (syscall_number, encoded_arguments) = match syscall {
        Syscall::Log(arguments) => (
            SyscallNumber::Log,
            EncodedArguments {
                log_arguments: arguments,
            },
        ),
    };
    // SAFETY: `usize` can store any combination of bits, so this will never be undefined behaviour.
    (syscall_number as u16, unsafe {
        encoded_arguments.register_values
    })
}

#[derive(Copy, Clone)]
pub enum SyscallDecodeError {
    InvalidSyscallNumber(u16),
}

impl Debug for SyscallDecodeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidSyscallNumber(number) => {
                write!(f, "Invalid syscall number: {}", number)
            }
        }
    }
}

#[inline]
pub fn decode_syscall(
    number: u16,
    arguments: RegisterValues,
) -> Result<Syscall, SyscallDecodeError> {
    if number >= SyscallNumber::_Max as u16 {
        return Err(SyscallDecodeError::InvalidSyscallNumber(number));
    }
    // SAFETY: We checked above that `number` is within the range of valid `SyscallNumber`s.
    let syscall_number = unsafe { transmute::<u16, SyscallNumber>(number) };
    let encoded_arguments = EncodedArguments {
        register_values: arguments,
    };
    // SAFETY: All specific argument types are safe to transmute from `usize`.
    unsafe {
        match (syscall_number, encoded_arguments) {
            (SyscallNumber::Log, EncodedArguments { log_arguments }) => {
                Ok(Syscall::Log(log_arguments))
            }
            (SyscallNumber::_Max, _) => unreachable!(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
union EncodedResult {
    // The same rules apply for what is a valid result type.
    log_result: log::EncodedResult,
    register_values: RegisterValues,
}

const _: () = assert!(size_of::<EncodedResult>() == size_of::<RegisterValues>());

#[derive(Debug)]
pub enum SyscallResult {
    Log(Result<(), LogError>),
}

#[inline]
pub fn encode_syscall_result(result: SyscallResult) -> RegisterValues {
    let encoded_result = match result {
        SyscallResult::Log(log_result) => EncodedResult {
            log_result: log::encode_log_result(log_result),
        },
    };
    // SAFETY: `usize` can store any combination of bits, so this will never be undefined behaviour.
    unsafe { encoded_result.register_values }
}

pub enum SyscallResultDecodeError {
    InvalidResultField(&'static str),
}

impl Debug for SyscallResultDecodeError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Self::InvalidResultField(field) => {
                write!(f, "Invalid {} in result", field)
            }
        }
    }
}

#[inline]
pub fn decode_syscall_result(
    syscall_number: SyscallNumber,
    result_registers: RegisterValues,
) -> Result<SyscallResult, SyscallResultDecodeError> {
    let encoded_result = EncodedResult {
        register_values: result_registers,
    };
    // SAFETY: All specific result types are safe to transmute from `usize`.
    unsafe {
        match (syscall_number, encoded_result) {
            (SyscallNumber::Log, EncodedResult { log_result }) => {
                let decoded_log_result = log::decode_log_result(log_result)?;
                Ok(SyscallResult::Log(decoded_log_result))
            }
            (SyscallNumber::_Max, _) => unreachable!(),
        }
    }
}