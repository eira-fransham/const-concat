# Const string concatenation

Rust has some great little magic built-in macros that you can use. A particularly-helpful one for building up paths and other text at compile-time is `concat!`. This takes two strings and returns the concatenation of them:

```rust
const HELLO_WORLD: &str = concat!("Hello", ", ", "world!");

assert_eq!(HELLO_WORLD, "Hello, world!");
```

This is nice, but it falls apart pretty quickly. You can use `concat!` on the strings returned from magic macros like `env!` and `include_str!` but you can't use it on constants:

```rust
const GREETING: &str = "Hello";
const PLACE: &str = "world";
const HELLO_WORLD: &str = concat!(GREETING, ", ", PLACE, "!");
```

This produces the error:

```
error: expected a literal
 --> src/main.rs:3:35
  |
3 | const HELLO_WORLD: &str = concat!(GREETING, ", ", PLACE, "!");
  |                                   ^^^^^^^^

error: expected a literal
 --> src/main.rs:3:51
  |
3 | const HELLO_WORLD: &str = concat!(GREETING, ", ", PLACE, "!");
  |                                                   ^^^^^
```

Well with `const_concat!` you can! It works just like the `concat!` macro:

```rust
#[macro_use]
extern crate const_concat;

const GREETING: &str = "Hello";
const PLACE: &str = "world";
const HELLO_WORLD: &str = const_concat!(GREETING, ", ", PLACE, "!");

assert_eq!(HELLO_WORLD, "Hello, world!");
```

All this, and it's implemented entirely without hooking into the compiler. So how does it work? Through dark, evil magicks. Firstly, why can't this just work the same as runtime string concatenation? Well, runtime string concatenation allocates a new `String`, but allocation isn't possible at compile-time - we have to do everything on the stack. Also, we can't do iteration at compile-time so there's no way to copy the characters from the source strings to the destination string. Let's look at the implementation. The "workhorse" of this macro is the `concat` function:

```rust
pub const unsafe fn concat<First, Second, Out>(a: &[u8], b: &[u8]) -> Out
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
```

So what we do is convert both the (arbitrarily-sized) input arrays to pointers to constant-size arrays (well, actually to pointer-to-`First` and pointer-to-`Second`, but the intent is that `First` and `Second` are fixed-size arrays). Then, we dereference them. This is wildly unsafe - there's nothing saying that `a.len()` is the same as the length of the `First` type parameter. We put them next to one another in a `#[repr(C)]` tuple struct - this essentially concatenates them together in memory. Finally, we transmute it to the `Out` type parameter. If `First` is `[u8; N0]` and `Second` is `[u8; N1]` then `Out` should be `[u8; N0 + N1]`. Why not just use a trait with associated constants? Well, here's an example of what that would look like:

```rust
trait ConcatHack {
    const A_LEN: usize;
    const B_LEN: usize;
}

pub const unsafe fn concat<C>(
    a: &[u8],
    b: &[u8],
) -> [u8; C::A_LEN + C::B_LEN]
where
    C: ConcatHack,
{
    #[repr(C)]
    #[derive(Copy, Clone)]
    struct Both<A, B>(A, B);

    let arr: Both<First, Second> =
        Both(*transmute::<_, &[u8; C::A_LEN]>(a), *transmute::<_, &[u8; C::B_LEN]>(b));

    transmute(arr)
}
```

This doesn't work though, because [type parameters are not respected when calculating fixed-size array lengths][fixed-size-length-problems]. So instead we use individual type parameters for each constant-size array.

[fixed-size-length-problems]: https://github.com/rust-lang/rust/issues/43408#issuecomment-318258935

Wait, though, if you look at [the documentation for `std::mem::tranmute`][transmute] at the time of writing it's not a `const fn`. What's going on here then? Well, I wrote my own `transmute`:

[transmute]: https://doc.rust-lang.org/1.26.0/std/mem/fn.transmute.html

```rust
#[allow(unions_with_drop_fields)]
pub const unsafe fn transmute<From, To>(from: From) -> To {
    union Transmute<From, To> {
        from: From,
        to: To,
    }

    Transmute { from }.to
}
```

This is allowed in a `const fn` where `std::mem::transmute` is not. Finally, let's look at the macro itself:

```rust
#[macro_export]
macro_rules! const_concat {
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
}
```

So first we create a `&'static [u8]` and then we transmute it to `&'static str`. This works for now because `&[u8]` and `&str` have the same layout, but it's not guaranteed to work forever. The cast to `&'static [u8]` works even though the right-hand side of that assignment is local to this scope because of something called ["rvalue static promotion"][rv-static-promotion].

This currently doesn't work in trait associated constants. I do have a way to support trait associated constants but again, you can't access type parameters in array lengths so that unfortunately doesn't work. Finally, it requires quite a few nightly features:

```rust
#![feature(const_fn, const_str_as_bytes, const_str_len, const_let, untagged_unions)]
```

[rv-static-promotion]: https://github.com/rust-lang/rfcs/blob/master/text/1414-rvalue_static_promotion.md
