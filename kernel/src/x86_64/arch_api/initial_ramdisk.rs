use crate::unsafe_impl::init_cell::{InitCell, NoConcurrency};

static INITIAL_RAMDISK: InitCell<&'static [u8]> = InitCell::new();

pub fn set_initial_ramdisk(ramdisk: &'static [u8], no_concurrency: &NoConcurrency) {
    INITIAL_RAMDISK.set(ramdisk, no_concurrency);
}

pub fn get_initial_ramdisk() -> Option<&'static [u8]> {
    INITIAL_RAMDISK.get().copied()
}
