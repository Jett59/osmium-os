static mut ROOT_TABLE_ADDRESS: usize = 0;

pub(in crate::arch) fn init(rsdt_address: usize) {
    // # Safety
    // It is safe to assign to ROOT_TABLE_ADDRESS because this function is only called once, and then before threading is initialized.
    unsafe {
        ROOT_TABLE_ADDRESS = rsdt_address;
    }
}

pub fn get_root_table_address() -> usize {
    // # Safety
    // Se above for init.
    unsafe { ROOT_TABLE_ADDRESS }
}
