use common::framebuffer;

use crate::physical_memory_manager;
use crate::unsafe_impl::arch::exceptions::load_exceptions;
use crate::unsafe_impl::arch::init::{available_memory_map_entries, frame_buffer};
use crate::unsafe_impl::init_cell::NoConcurrency;
use crate::unsafe_impl::paging::init::initialize_lower_half_table;

pub fn arch_init(_no_concurrency: &NoConcurrency) {
    load_exceptions();

    available_memory_map_entries().for_each(|(address, size)| {
        physical_memory_manager::mark_range_as_free(address, address + size);
    });

    initialize_lower_half_table();

    framebuffer::init(frame_buffer().expect("Could not get frame buffer"));
}
