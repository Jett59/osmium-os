#![no_std]

#![no_main]

#[allow(unused_imports)]
use osmium_runtime::panic as _;

#[no_mangle]
fn main() {
    unsafe { *(0x4 as *mut u32) = 0xdeadbeef };
    loop {}
}
