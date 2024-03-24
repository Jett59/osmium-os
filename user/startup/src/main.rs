#![no_std]

#![no_main]

#[allow(unused_imports)]
use osmium_runtime::panic as _;

#[no_mangle]
extern "C" fn main() {
    loop {}
}
