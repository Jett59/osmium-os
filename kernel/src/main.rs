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
    array_chunks,
)]
// Shut up the compiler about const generic expressions.
#![allow(incomplete_features)]
// While I don't enjoy surpressing warnings, I think that this particular warning is unnecessary at this stage of development. It would be more useful when the basic components are in place and working.
#![allow(dead_code)]

mod acpi;
mod assert;
mod buddy;
mod console;
mod elf;
mod font_renderer;
mod heap;
mod initial_ramdisk;
mod lazy_init;
mod memory;
mod mmio;
mod paging;
mod physical_memory_manager;
mod user_memory;

#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
#[cfg_attr(target_arch = "aarch64", path = "aarch64/mod.rs")]
mod arch;

pub use arch::arch_api;
use common::elf::load_elf;

use core::panic::PanicInfo;

use crate::{
    arch_api::user_mode::enter_user_mode, elf::map_sections, initial_ramdisk::read_initial_ramdisk,
};

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
    let required_acpi_tables = acpi::find_required_acpi_tables().unwrap();
    let acpi_info = arch_api::acpi::handle_acpi_info(required_acpi_tables);
    arch_api::irq::initialize(&acpi_info);
    arch_api::timer::initialize(&acpi_info);

    let initial_ramdisk = read_initial_ramdisk(
        arch_api::initial_ramdisk::get_initial_ramdisk().expect("No initial_ramdisk found"),
    );
    let startup_program = initial_ramdisk
        .get("services/startup")
        .expect("No startup program found in initial ramdisk");
    let startup_elf_info = load_elf(startup_program).expect("Failed to parse startup program");
    map_sections(&startup_elf_info, startup_program);
    unsafe { enter_user_mode(startup_elf_info.entrypoint) };
}

#[cfg(test)]
fn main() {}
