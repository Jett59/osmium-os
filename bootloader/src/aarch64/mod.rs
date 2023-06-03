mod registers;

pub fn check_environment() {
    if registers::current_el() != registers::ExceptionLevel::EL1 {
        unsafe { registers::switch_to_el1() };
    }
}
