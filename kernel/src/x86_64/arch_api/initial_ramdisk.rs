pub(in crate::arch) static mut INITIAL_RAMDISK: Option<&[u8]> = None;

pub fn get_initial_ramdisk() -> Option<&'static [u8]> {
    // Safety: initial_ramdisk is only initialized once, and then before threading is initialized.
    unsafe { INITIAL_RAMDISK }
}
