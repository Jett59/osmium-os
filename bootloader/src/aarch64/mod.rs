mod registers;

pub fn check_environment() {
    assert!(
        registers::current_el() == registers::ExceptionLevel::EL1
            || registers::current_el() == registers::ExceptionLevel::EL2,
        "Must be running at EL1 or EL2"
    );
}
