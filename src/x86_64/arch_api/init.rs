use crate::x86_64::multiboot;

use super::paging;

pub fn arch_init() {
    multiboot::parse_multiboot_structures();
    paging::initialize_paging();
}
