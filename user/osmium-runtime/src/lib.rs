#![no_std]

#[cfg_attr(not(test), panic_handler)]
pub fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern "Rust" {
    fn main();
}

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe { main() };
    panic!("Main should not return");
}
