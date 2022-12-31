use crate::x86_64::multiboot;

pub fn arch_init() {
    multiboot::parse_multiboot_structures();
}
