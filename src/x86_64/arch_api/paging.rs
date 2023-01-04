pub const PAGE_SIZE: usize = 4096;

const RECURSIVE_PAGE_TABLE_INDEX: usize = 257; // We are in the last 2g so we can't use 511.
