#![no_std]
// This whole no_main thing gets rather complicated when we want to support unit tests. See the (seemingly) random cfg attributes and other weirdness in this file and others (like skipping checks which won't work in the test environment etc.).
#![cfg_attr(not(test), no_main)]
// Lets just hope these aren't as unstable as the language says they are (it would be a pain to have to change everywhere one of these is used)
#![feature(
    core_intrinsics,
    generic_const_exprs,
    const_trait_impl,
    // Why all of these maybe_uninit things are separate is beyond me.
    maybe_uninit_uninit_array,
    maybe_uninit_array_assume_init,
    const_maybe_uninit_uninit_array,
    const_mut_refs,
    const_maybe_uninit_write,
    const_maybe_uninit_array_assume_init
)]

mod assert;
mod lazy_init;
mod memory;
mod paging;
mod pmm;

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
    console::write_string("Panic!!!\n");
    loop {}
}

#[no_mangle]
extern "C" fn kmain() -> ! {
    console::clear();
    console::write_string("Hello, World!\n");
    arch_api::init::arch_init();
    pmm::sanity_check();
    paging::map_block(0, pmm::allocate_block_address().unwrap());
    loop {}
}

#[cfg(test)]
fn main() {}
