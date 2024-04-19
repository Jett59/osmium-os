#![no_std]
#![no_main]

#[allow(unused_imports)]
use osmium_runtime::panic as _;
use syscall_interface::user::log;

#[no_mangle]
extern "C" fn main() {
    log("Hello!").unwrap();
    log("Amazing! The syscall actually worked!").unwrap();
}
