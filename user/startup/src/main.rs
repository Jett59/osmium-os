#![no_std]
#![no_main]

#[allow(unused_imports)]
use osmium_runtime::panic as _;

#[no_mangle]
extern "C" fn main() {
    #[cfg(target_arch = "aarch64")]
    unsafe {
        core::arch::asm!("mov x0, #3", "svc 0")
    };
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("int 0x80")
    };
    loop {}
}
