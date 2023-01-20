// TODO: What granularity should we use? We must look at the support for the different sizes to make this judgement.
pub const PAGE_SIZE: usize = 16; // Not really.

pub fn get_physical_address(_virtual_address: usize) -> usize {
    unimplemented!();
}

pub fn map_page(_virtual_address: usize, _physical_address: usize) {
    unimplemented!();
}

pub fn unmap_page(_virtual_address: usize) {
    unimplemented!();
}
