macro_rules! const_assert {
    ($e:expr, $msg:literal) => {
        const _: () = assert!($e, $msg);
    };
}

pub(crate) use const_assert;
