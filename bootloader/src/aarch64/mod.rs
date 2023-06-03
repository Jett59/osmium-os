mod registers;

pub fn check_environment() {
    assert_eq!(
        registers::current_el(),
        registers::ExceptionLevel::EL2,
        "Must start in EL2"
    );
}
