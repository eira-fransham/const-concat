#![feature(const_fn, const_str_as_bytes, const_slice_len, const_str_len, const_let, untagged_unions)]

#[allow(unions_with_drop_fields)]
pub const unsafe fn transmute<From, To>(from: From) -> To {
    union Transmute<From, To> {
        from: From,
        to: To,
    }

    Transmute { from }.to
}

pub trait ConstBuffer {
    type First: Copy;
    type Second: Copy;
    type Out: Copy;
}

pub const unsafe fn concat_inner<First, Second, Out>(a: &[u8], b: &[u8]) -> Out
where
    First: Copy,
    Second: Copy,
    Out: Copy,
{
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Both<A, B>(A, B);

    let arr: Both<First, Second> =
        Both(*transmute::<_, &First>(a), *transmute::<_, &Second>(b));

    transmute(arr)
}

pub const unsafe fn concat<C: ConstBuffer>(a: &[u8], b: &[u8]) -> C::Out {


    concat_inner::<C::First, C::Second, C::Out>(a, b)
}

#[macro_export]
macro_rules! const_concat {
    (@bytes $a:expr, $b:expr) => {{
        unsafe { $crate::concat_inner::<[u8; $a.len()], [u8; $b.len()], [u8; $a.len() + $b.len()]>($a, $b) }
    }};
    (@inner $a:expr, $b:expr) => {{
        const_concat!(@bytes $a.as_bytes(), $b.as_bytes())
    }};
    (@inner $a:expr, $($rest:expr),*) => {{
        const_concat!(@bytes $a.as_bytes(), &const_concat!(@inner $($rest),*))
    }};
    ($a:expr, $b:expr) => {{
        let bytes: &'static [u8] = &const_concat!(@inner $a.as_bytes(), $b.as_bytes());
        
        unsafe { $crate::transmute::<_, &'static str>(bytes) }
    }};
    ($a:expr, $($rest:expr),*) => {{
        let bytes: &'static [u8] = &const_concat!(@inner $a, $($rest),*);
        
        unsafe { $crate::transmute::<_, &'static str>(bytes) }
    }};
    ($a:expr, $($rest:expr),*,) => {
        const_concat!($a, $($rest),*);
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn top_level_constants() {
        const SALUTATION: &str = "Hello";
        const TARGET: &str = "world";
        const GREETING: &str = const_concat!(SALUTATION, ", ", TARGET, "!");

        assert_eq!(GREETING, "Hello, world!");
    }

    #[test]
    fn assoc_constants() {
        trait DebugTypeString {
            const NAME: &'static str;
        }

        struct Foo;
        impl DebugTypeString for Foo {
            const NAME: &'static str = "Foo";
        }

        struct Bar;
        impl DebugTypeString for Bar {
            const NAME: &'static str = "Bar";
        }

        impl<A: DebugTypeString, B: DebugTypeString> DebugTypeString for (A, B) {
            const NAME: &'static str = const_concat!("(", A::NAME, ", ", B::NAME, ")");
        }

        assert_eq!(<(Foo, Bar)>::NAME, "Hello, world!");
    }
}
