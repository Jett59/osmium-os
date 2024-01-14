static mut ACPI_ADDRESS: usize = 0;

pub(in crate::arch) fn init(rsdt_address: usize) {
    // # Safety
    // This should only ever be called once, right at the start of the program. Therefore race conditions are impossible.
    unsafe {
        ACPI_ADDRESS = rsdt_address;
    }
}

pub fn get_rsdt_address() -> usize {
    // # Safety
    // Se above for init.
    unsafe { ACPI_ADDRESS }
}
