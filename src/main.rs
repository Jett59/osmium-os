#![no_std]
#![no_main]

mod memory;

#[cfg(target_arch = "x86_64")]
mod x86_64;
#[cfg(target_arch = "x86_64")]
pub use x86_64::arch_api;

use arch_api::console;

// Needed to silence rust-analyzer which uses test mode, where this is unused because the panic handler is conditionally excluded in test mode
#[allow(unused_imports)]
use core::panic::PanicInfo;

#[panic_handler]
#[cfg(not(test))]
fn kpanic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
extern "C" fn kmain() -> ! {
    console::clear();
    console::write_string("Hello, World!\n");
    arch_api::init::arch_init();
    loop {}
}
