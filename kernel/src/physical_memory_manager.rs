use crate::{
    assert::const_assert,
    paging::PAGE_SIZE,
    unsafe_impl::{
        memory_token::PhysicalMemoryToken,
        token_bitmap::{BitmapToken, TokenBitmap},
    },
};

// The size of a block (bit) in the bitmap allocator.
pub const BLOCK_SIZE: usize = 65536;
pub const LOG2_BLOCK_SIZE: u8 = BLOCK_SIZE.trailing_zeros() as u8;

const_assert!(
    1 << LOG2_BLOCK_SIZE == BLOCK_SIZE,
    "BLOCK_SIZE must be a power of two"
);

const_assert!(
    PAGE_SIZE < BLOCK_SIZE && BLOCK_SIZE.is_multiple_of(PAGE_SIZE),
    "Block size must be larger and divisible by (platform-specific) page size."
);

pub const PAGES_PER_BLOCK: usize = BLOCK_SIZE / PAGE_SIZE;

pub const MAX_PHYSICAL_MEMORY: usize = 0x1000000000;

pub const BLOCK_COUNT: usize = MAX_PHYSICAL_MEMORY / BLOCK_SIZE;

static GLOBAL_PMM: TokenBitmap<PhysicalMemoryToken, BLOCK_SIZE, BLOCK_COUNT> = TokenBitmap::new();

pub fn mark_as_free(range: PhysicalMemoryToken) {
    GLOBAL_PMM.store_range(
        BitmapToken::try_from_inner(range).expect("address and size must be BLOCK_SIZE aligned"),
    );
}

pub fn allocate_block() -> Option<PhysicalMemoryToken> {
    GLOBAL_PMM
        .read_range(0, BLOCK_COUNT)
        .next()
        .map(|token| token.into_inner())
}

// This is outside the test module because it is for testing in the real kernel environment and not part of the unit testing suite.
pub fn sanity_check() {
    // Make sure there is some memory to work with. I'll probably add more stuff later.
    mark_as_free(allocate_block().expect("There should be at least some memory by this point"));
}
