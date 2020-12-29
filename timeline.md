# Roadmap

## Steps
1. Immediate type inferrence (`auto` style)
2. Traits + operators (e.g.: `a + b` becomes `std::Ops::Add(a, b)` where `a` and `b` implement `Add`)
3. Structs, tuples, enums
4. Rust FFI
5. Generics (choose poly/mono-morphic approach)
6. Bounds

## End Goals
- Type inferrence similar to Rust's
- Simple memory management with a variety of options