---
title: "Reading a Line of Rust: `use`, `::`, and `&str`"
date: 2026-07-26
description: The first four lines of the server, token by token. What `use` actually does, when to write `::` instead of `.`, why `bind` is not called on a listener, and what a string literal really is.
draft: false
---

# Reading a Line of Rust: `use`, `::`, and `&str`

🔑 Core Concept
Format: the concept, the mental model, the check questions, and the answers worth remembering.

**Introduced while writing:** `src/main.rs`, the first four lines of the server

### The code that introduced it

```rust
use std::net::TcpListener;

fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878");
}
```

### The concept

> **🔑 Core Concept: `::` reaches for a name, `.` reaches into a value**
>
> `::` walks through **names**: crates, modules, types, and the functions attached to them.
>
> `.` acts on a **value you already have**.
>
> The question to ask: **do I have a thing, or am I naming a thing?**  Naming takes `::` Having takes `.`

### 1. `use` is a shortcut for typing

The type's full name is `std::net::TcpListener`: the `std` crate (Rust's standard library), its `net`
module, then the type. `use` says "when I write `TcpListener`, I mean that full path."

That is all it does. Delete the line and the program still has access to the type; you just write
`std::net::TcpListener::bind(...)` instead. Same machine code either way.

### 2. Why `TcpListener::bind` and not `listener.bind`

At that moment no `TcpListener` exists yet. Creating one is the point of the call, so there is
nothing for a `.` to reach into.

A function that takes `self` is a **method**, and it works on one specific value you already hold
([the three receiver forms are covered here](https://ruro122020.github.io/portfolio/blog/03-method-chaining-by-ownership/)).
A function that takes **no `self`** is an **associated function**: it belongs to the type rather than
to any value of it, so it is reached with `::`.

`bind` has no `self`, because a method would need a listener to already exist and `bind` is what makes
one. This is how constructors work in Rust generally: `Vec::new()`, `String::from("hello")`. There is
no `new` keyword, just a naming convention.

### 3. `"127.0.0.1:7878"` is a `&str`

`&str` **borrows** text. `String` **owns** it and can grow. A string literal is baked into the
compiled program, so a `&str` pointing at one is just a view: nothing to allocate, nothing to free.

Rule of thumb: take `&str` when a function only reads text, return `String` when it makes new text.

### Check questions (and the answers that matter)

**1. If you delete the `use` line, what breaks?**

Only the short name. You must write the full path instead. `use` controls spelling, not access.

**2. Why can you write `listener.accept()` but not `listener.bind(addr)`?**

`accept` takes `self`, so it needs a listener to act on and you have one. `bind` takes no `self`; it
belongs to the type, so it is named with `::`.

### Common pitfall

Reading a compiler warning as a claim about memory. This program warns `unused variable: listener`,
which sounds like the listener is wasting space.

It is not. The compiler's suggested fix is to rename it `_listener`, and a rename changes nothing
about what is allocated. The lint is about **intent**: you named something and never used the name,
so you are probably not finished.
