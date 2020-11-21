#![feature(
    const_fn,
    const_fn_union,
    untagged_unions,
    const_raw_ptr_deref
)]

use std::mem::ManuallyDrop;

pub const unsafe fn transmute<From, To>(from: From) -> To {
    union Transmute<From, To> {
        from: ManuallyDrop<From>,
        to: ManuallyDrop<To>,
    }

    ManuallyDrop::into_inner(Transmute{from:ManuallyDrop::new(from)}.to)
}

pub const unsafe fn concat<First, Second, Out>(a: &[u8], b: &[u8]) -> Out
where
    First: Copy,
    Second: Copy,
    Out: Copy,
{
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Both<A, B>(A, B);

    let arr: Both<First, Second> = Both(
        *transmute::<_, *const First>(a.as_ptr()),
        *transmute::<_, *const Second>(b.as_ptr()),
    );

    transmute(arr)
}

#[macro_export]
macro_rules! const_concat {
    () => {
        ""
    };
    ($a:expr) => {
        $a
    };
    ($a:expr, $b:expr) => {{
        let bytes: &'static [u8] = unsafe {
            &$crate::concat::<
                [u8; $a.len()],
                [u8; $b.len()],
                [u8; $a.len() + $b.len()],
            >($a.as_bytes(), $b.as_bytes())
        };

        unsafe { $crate::transmute::<_, &'static str>(bytes) }
    }};
    ($a:expr, $($rest:expr),*) => {{
        const TAIL: &str = const_concat!($($rest),*);
        const_concat!($a, TAIL)
    }};
    ($a:expr, $($rest:expr),*,) => {
        const_concat!($a, $($rest),*)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn top_level_constants() {
        const SALUTATION: &str = "Hello";
        const TARGET: &str = "world";
        const GREETING: &str = const_concat!(SALUTATION, ", ", TARGET, "!");
        const GREETING_TRAILING_COMMA: &str = const_concat!(SALUTATION, ", ", TARGET, "!",);

        assert_eq!(GREETING, "Hello, world!");
        assert_eq!(GREETING_TRAILING_COMMA, "Hello, world!");
    }
}
