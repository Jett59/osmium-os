#![no_std]
// This whole no_main thing gets rather complicated when we want to support unit tests. See the (seemingly) random cfg attributes and other weirdness in this file and others (like skipping checks which won't work in the test environment etc.).
#![cfg_attr(not(test), no_main)]
// Lets just hope these aren't as unstable as the language says they are (it would be a pain to have to change everywhere one of these is used)
#![feature(
    generic_const_exprs,
    const_trait_impl,
    // Why all of these maybe_uninit things are separate is beyond me.
    maybe_uninit_uninit_array,
    maybe_uninit_array_assume_init,
    const_maybe_uninit_uninit_array,
    const_mut_refs,
    const_maybe_uninit_write,
    const_maybe_uninit_array_assume_init,
    let_chains,
    new_uninit,
    asm_const,
)]
// Shut up the compiler about const generic expressions.
#![allow(incomplete_features)]
// While I don't enjoy surpressing warnings, I think that this particular warning is unnecessary at this stage of development. It would be more useful when the basic components are in place and working.
#![allow(dead_code)]

mod assert;
mod buddy;
mod console;
mod font_renderer;
mod heap;
mod lazy_init;
mod memory;
mod paging;
mod physical_memory_manager;

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
#[cfg_attr(target_arch = "aarch64", path = "aarch64/mod.rs")]
mod arch;

pub use arch::arch_api;

use core::panic::PanicInfo;

extern crate alloc;

#[cfg_attr(not(test), panic_handler)]
fn kpanic(info: &PanicInfo) -> ! {
    console::print!("Kernel panic: {}\n", info);
    loop {}
}

#[no_mangle]
extern "C" fn kmain() -> ! {
    arch_api::init::arch_init();
    physical_memory_manager::sanity_check();
    heap::sanity_check();
    console::println!("Initialized the display (obviously)");
    console::println!("ACPI tables: {:#x?}", arch_api::acpi::get_rsdt_address());
    loop {}
}

#[cfg(test)]
fn main() {}
