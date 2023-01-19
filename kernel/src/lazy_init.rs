use core::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
};

pub struct LazilyInitialized<'initializer, T: Sized + Sync> {
    value: UnsafeCell<Option<T>>,
    initializer: &'initializer dyn Fn() -> T,
}

impl<'initializer, T: Sized + Sync> LazilyInitialized<'initializer, T> {
    pub const fn new(initializer: &'initializer dyn Fn() -> T) -> Self {
        Self {
            value: UnsafeCell::new(None),
            initializer,
        }
    }

    pub fn is_initialized(value: &Self) -> bool {
        // TODO: Make this safe. It should be ok for now because we don't have threads yet. Same goes for the rest of these functions.
        unsafe { value.value.get().as_ref().unwrap().is_some() }
    }
}

impl<'initializer, T: Sized + Sync> Deref for LazilyInitialized<'initializer, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // TODO: Make safe (see above)
        unsafe {
            if self.value.get().as_ref().unwrap().is_none() {
                *self.value.get() = Some((self.initializer)());
            }
            self.value.get().as_ref().unwrap().as_ref().unwrap()
        }
    }
}

impl<'initializer, T: Sized + Sync> DerefMut for LazilyInitialized<'initializer, T> {
    fn deref_mut(&mut self) -> &mut T {
        // TODO: Make safe.
        unsafe {
            if self.value.get_mut().is_none() {
                *self.value.get() = Some((self.initializer)());
            }
            self.value.get_mut().as_mut().unwrap()
        }
    }
}

macro_rules! lazy_static {
    ($visibility:vis static ref $name:ident : $type:ty = $initializer:expr ;) => {
        $visibility static mut $name: crate::lazy_init::LazilyInitialized<$type> = crate::lazy_init::LazilyInitialized::new(&||$initializer);
    };
}

pub(crate) use lazy_static;

#[cfg(test)]
mod test {
    use crate::lazy_init::LazilyInitialized;

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
            assert!(!LazilyInitialized::is_initialized(&LAZY_VARIABLE));
            LAZY_VARIABLE.nums[0] = 56;
            assert!(LazilyInitialized::is_initialized(&LAZY_VARIABLE));
            assert_eq!(LAZY_VARIABLE.nums[0], 56);
            assert_eq!(LAZY_VARIABLE.nums[1], 42);
        }
    }
}
