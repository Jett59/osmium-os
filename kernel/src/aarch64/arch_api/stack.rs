const STACK_SIZE: usize = 8192;

#[repr(align(4096))]
pub struct Stack {
    data: [u8; STACK_SIZE],
}

impl Stack {
    pub const fn as_mut_ptr(&mut self) -> *mut u8 {
        self.data.as_mut_ptr()
    }

    pub const fn default() -> Self {
        Self {
            data: [0; STACK_SIZE],
        }
    }
}
