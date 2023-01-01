pub struct LazilyInitialized<'initializer, T: Sized + Sync> {
    value: Option<T>,
    initializer: &'initializer dyn Fn() -> T,
}

impl<'initializer, T: Sized + Sync> LazilyInitialized<'initializer, T> {
    pub const fn new(initializer: &'initializer dyn Fn() -> T) -> Self {
        Self {
            value: None,
            initializer,
        }
    }
}

impl<'initializer, T: Sized + Sync> LazilyInitialized<'initializer, T> {
    pub fn get(&mut self) -> &T {
        if self.value.is_none() {
            self.value = Some((self.initializer)());
        }
        self.value.as_ref().unwrap()
    }
}

macro_rules! lazy_static {
    (static ref $visibility:vis $name:ident : $type:ty = $initializer:expr ;) => {
        $visibility static mut $name: crate::lazy_init::LazilyInitialized<$type> = crate::lazy_init::LazilyInitialized::new(&||$initializer);
    };
}

pub(crate) use lazy_static;
