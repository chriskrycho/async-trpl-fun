---
aliases:
  - std::pin::Pin
  - Pin
---
## Key bits

- Uses the type system/compiler to prevent moving *values* from being moved; “a Rust compiler considers all types movable” regardless of `Pin`. Prevents getting any `&mut T` access even when wrapping `Pin<&mut T>`.
- Has paired `Unpin` auto trait for types which are movable regardless of being `Pin`’d, e.g. basic types and references.
    - References being movable might be a surprise for a moment, but it’s fine: the key is that a thing *being referenced* not move, not the reference itself.

## Misc.

Related: `Box::pin` and `Box::into_pin`, which are likely common use patterns with `Pin` itself. Note that `Pin::new(Box::new("hey"))` is, under the hood. implementation-identical with `Box::new("hey").into_pin()` and *also* `Box::new("hey").into()` (with `Pin` as the target) since `impl<T> From<Box<T>> for Pin<Box<T>>` uses `Box::into_pin`, and `Pin::new()` and `Box::into_pin()` both just do `unsafe { Pin::new_unchecked(pointer) }`.