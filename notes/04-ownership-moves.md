---
title: The Move, Ownership Transfers Into the Call
date: 2026-08-01
description: Passing a value without & transfers ownership into the function's parameter for that one call. The caller's name for it is dead from that line on (E0382 if touched), and the value's cleanup brace becomes the function's.
draft: true
---

# The move: ownership transfers into the call

In `main`, each accepted connection is used for one line and then handed to `handle_connection`. Rust has to settle who is responsible for the connection after that handoff, because at any moment exactly one variable is responsible for a value. Passing without `&` settles it completely: the whole responsibility goes.

First, dissolve the parameter mystery:

```rust
fn handle_connection(stream: TcpStream) {
    println!("connection established");
}

// a parameter is a `let` in disguise. The function behaves exactly like:
{
    let stream = /* whatever the caller passed in */;
    println!("connection established");
}
```

A parameter is an ordinary variable that happens to get its value from the caller.

The two mechanisms, with one connection walked through each:

```
MOVE (this code):  handle_connection(stream)

  main's `stream` ──the value transfers──►  the function's `stream` variable
  main's name: dead                          body uses it
                                             }  ← the value's life ENDS here
  nothing comes back. main never sees that connection again.

BORROW (later, written with &):  handle_connection(&stream)

  main's `stream` ──lends access────────►  the function may use it
  main still the owner                       body uses it
                                             }  ← the ACCESS ends here
  the value survives, still owned by main, usable again after the call.
```

The `&` is the visible difference: no `&` means the value itself is handed over, one way, permanently.

Touch the dead name and the compiler stops the build:

```rust
Ok(stream) => {
    handle_connection(stream);   // the value moves here
    println!("{stream:?}");      // touching the dead name: E0382
}
```
The compiler error:

```
error[E0382]: borrow of moved value: `stream`
  |
9 |       Ok(stream) => {
  |          ------ move occurs because `stream` has type `TcpStream`,
  |                 which does not implement the `Copy` trait
10|         handle_connection(stream);
  |                           ------ value moved here
11|         println!("{stream:?}");
  |                    ^^^^^^ value borrowed here after move
```

Three arrows, telling the whole story in order: where the variable was born (line 9), where the value moved out of it (line 10), and where the dead name was touched (line 11). This check happens at compile time, the program never gets to run.

Each call of `handle_connection` is one execution with its own parameter variable and its own closing brace. Two connections means two calls, two owners, two separate cleanup moments. What happens at that brace is note 05.
