#![no_std]
#![no_main]

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
use x86_64::arch_api;

use core::panic::PanicInfo;

#[panic_handler]
fn kpanic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn kmain() -> ! {
    arch_api::console::clear();
    arch_api::console::write_string("Hello, World!");
    loop {}
}
