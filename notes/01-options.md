---
title: Option, a Value That Might Not Be There
date: 2026-07-31
description: Rust has no null. `Option<T>` is either `Some(value)` holding a value or `None` holding nothing, and the compiler makes you handle the `None` case before you can touch the value.
draft: false
---

# `Option`: a value that might not be there

Rust doesn't have null. When a value might be absent, the type says so: `Option` is an enum whose variants are `Some(value)` or `None`. `None` holds nothing. `Some(value)` contains whatever value was put inside it. Unlike null in other languages, `None` can't go just anywhere: it only exists inside an `Option`, so only types that declare "might be absent" can ever be absent.

The compiler forces you to handle the `None` case (with `match`, for now) before you can touch the value inside.

## Variant versus type parameter

Two different kinds of names appear in these enums, and they live in different places:

```rust
enum Option<T> {
    Some(T),
    None,
}

enum Result<T, E> {
    Ok(T),
    Err(E),
}
```

| Name | Kind | What it is |
|------|------|------------|
| `T` in `Option<T>` | type parameter | a placeholder in `<angle brackets>`, replaced by a real type at compile time |
| `Some`, `None` | variants | the possible shapes of the value, the names you write in a `match` |
| `T`, `E` in `Result<T, E>` | type parameters | same placeholder idea; `E` is conventionally the error type |
| `Ok`, `Err` | variants | success shape and failure shape |

The compiler fills the placeholders with real types at compile time, based on what you wrote at the use site (the place in your code where you use the type): in `Option<TcpStream>`, `T` has become `TcpStream`; in `Result<TcpListener, io::Error>`, `T` has become `TcpListener` and `E` has become `io::Error`.

## When to use which

An `Option` is for when a value can simply be absent and absence needs no explanation, like an iterator that has finished. A `Result` is for when you need to know why something failed.
