const STACK_SIZE: usize = 8192;

#[repr(C, align(4096))]
pub struct Stack {
    data: [u8; STACK_SIZE],
}

impl Stack {
    pub const fn default() -> Self {
        Self {
            data: [0; STACK_SIZE],
        }
    }
}
