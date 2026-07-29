use common::framebuffer;

use crate::physical_memory_manager::{self, BLOCK_SIZE};
use crate::unsafe_impl::arch::exceptions::load_exceptions;
use crate::unsafe_impl::arch::init::{available_memory_map_entries, frame_buffer};
use crate::unsafe_impl::init_cell::NoConcurrency;
use crate::unsafe_impl::memory_token::MemoryToken;
use crate::unsafe_impl::paging::init::initialize_lower_half_table;

pub fn arch_init(_no_concurrency: &NoConcurrency) {
    load_exceptions();

    available_memory_map_entries()
        .filter(|token| token.size() >= BLOCK_SIZE) // <-- VERY IMPORTANT to make split_at not panic.
        .map(|token| {
            // Align the tokens to block size, as expected by the PMM.
            let token_address = token.address();
            // TODO: this calculation is slightly wrong, if the token is already aligned then it will skip a block.
            let (_, token) = token.split_at(BLOCK_SIZE - token_address % BLOCK_SIZE);
            let token_size = token.size();
            let (token, _) = token.split_at(token_size - (token_size % BLOCK_SIZE));
            token
        })
        .for_each(physical_memory_manager::mark_as_free);

    initialize_lower_half_table();

    framebuffer::init(frame_buffer().expect("Could not get frame buffer"));
}
