#[cfg_attr(target_arch = "x86_64", path = "x86_64/mod.rs")]
#[cfg_attr(target_arch = "aarch64", path = "./aarch64/mod.rs")]
pub mod arch;
pub mod global_allocator;
pub mod init_cell;
pub mod memory_token;
pub mod paging;
pub mod slab;
pub mod token_bitmap;
