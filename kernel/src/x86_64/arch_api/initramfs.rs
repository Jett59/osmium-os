pub(in crate::arch) static mut INITRAMFS: Option<&[u8]> = None;

pub fn get_initramfs() -> Option<&'static [u8]> {
    // Safety: INITRAMFS is only initialized once, and then before threading is initialized.
    unsafe { INITRAMFS }
}
