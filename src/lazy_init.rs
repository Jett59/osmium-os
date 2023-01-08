macro_rules! lazy_static {
    ($visibility:vis static ref $name:ident : $type:ty = $initializer:expr ;) => {
        #[allow(non_camel_case_types)]
        $visibility struct $name {
    value: core::cell::UnsafeCell<Option<$type>>,
}

impl $name {
    const fn new() -> Self {
        Self {
            value: core::cell::UnsafeCell::new(None),
        }
    }

    $visibility fn is_initialized(value: &Self) -> bool {
        // TODO: Make this safe. It should be ok for now because we don't have threads yet. Same goes for the rest of these functions.
        unsafe { value.value.get().as_ref().unwrap().is_some() }
    }
}

impl core::ops::Deref for $name {
    type Target = $type;

    // Needed if the initializer uses an unsafe block (this also hides other warnings but such is life).
    #[allow(unused_unsafe)]
    fn deref(&self) -> &$type {
        // TODO: Make safe (see above)
        unsafe {
            if self.value.get().as_ref().is_none() {
                *self.value.get() = Some($initializer);
            }
            self.value.get().as_ref().unwrap().as_ref().unwrap()
        }
    }
}

impl core::ops::DerefMut for $name {
    fn deref_mut(&mut self) -> &mut $type {
        // TODO: Make safe.
            if self.value.get_mut().is_none() {
                *self.value.get_mut() = Some($initializer);
            }
            self.value.get_mut().as_mut().unwrap()
    }
}
        $visibility static mut $name: $name = $name::new();
    };
}

pub(crate) use lazy_static;

#[cfg(test)]
mod test {
    use super::*;

    struct CantConstructAtCompileTime {
        nums: [u8; 24],
    }

    impl CantConstructAtCompileTime {
        pub fn new(n: u8) -> Self {
            Self { nums: [n; 24] }
        }
    }

    lazy_static! {
        static ref LAZY_VARIABLE: CantConstructAtCompileTime = CantConstructAtCompileTime::new(42);
    }

    #[test]
    fn lazy_initialization_test() {
        unsafe {
            assert!(!LAZY_VARIABLE::is_initialized(&LAZY_VARIABLE));
            LAZY_VARIABLE.nums[0] = 56;
            assert!(LAZY_VARIABLE::is_initialized(&LAZY_VARIABLE));
            assert_eq!(LAZY_VARIABLE.nums[0], 56);
            assert_eq!(LAZY_VARIABLE.nums[1], 42);
        }
    }
}
