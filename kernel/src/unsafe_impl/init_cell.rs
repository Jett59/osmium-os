use core::{cell::UnsafeCell, marker::PhantomData};

/// Zero-sized type proving that there is no concurrency or preemption in the kernel.
///
/// This can only be constructed by unsafe code, and must only be constructed when the conditions are met.
pub struct NoConcurrency(PhantomData<()>);

impl NoConcurrency {
    /// Constructs a new `NoConcurrency` instance.
    ///
    /// # Safety
    ///
    /// This function is unsafe because it can only be called when there is no concurrency in the kernel.
    /// For example, main.rs can call this function before interrupts are enabled.
    pub unsafe fn new() -> Self {
        NoConcurrency(PhantomData)
    }
}

/// A cell that can be initialized once and then read from multiple threads.
///
/// Initialisation must occur when there is no concurrency, usually during kernel init when interrupts are disabled. After initialisation, the value can be read from multiple threads safely.
/// This is useful for variables which are populated from run-time tables, but which are never updated after initialisation.
pub struct InitCell<T> {
    value: UnsafeCell<Option<T>>,
}

unsafe impl<T> Sync for InitCell<T> where T: Sync {}

impl<T> InitCell<T> {
    pub const fn new() -> Self {
        InitCell {
            value: UnsafeCell::new(None),
        }
    }

    /// Sets the value of the cell. This can only be called when there is no concurrency, which is proven by the `NoConcurrency` argument.
    ///
    /// If this `InitCell` has already been set, this function will panic.
    pub fn set(&self, value: T, _no_concurrency: &NoConcurrency) {
        assert!(self.get().is_none(), "InitCell set twice");
        // SAFETY: The caller must ensure that there is no concurrency when calling this function, which means that data races are impossible.
        // It is impossible for client code to hold a reference to the value, since references are not generated while `value` is `None`.
        unsafe { *self.value.get() = Some(value) };
    }

    pub fn get(&self) -> Option<&T> {
        // SAFETY: mutation happens only when there is no concurrency, and hence can be regarded as atomic. This function only returns a reference after this point, when mutation is no longer possible.
        unsafe { (*self.value.get()).as_ref() }
    }
}
