---
title: The for Loop Desugar, Two Layers of Wrapping
date: 2026-08-01
description: A single value with two layers of wrapping comes out of next(), and each match opens one layer. The for loop performs the outer Option match silently, which is why the loop body starts one layer deep.
draft: true
---

The code written in `main.rs`:

```rust
for stream in listener.incoming() {
    match stream {
        Ok(stream) => { /* handle the connection */ }
        Err(error) => eprintln!("connection failed: {error}"),
    }
}
```

# The `for` loop desugar: two layers of wrapping

The nested match is confusing because a single value with **two layers of wrapping** comes out of `next()`, and each `match` opens one layer.

When a client connects, `next()` hands you this value:

```
Some( Ok( TcpStream ) )
  │    │      └── the actual connection
  │    └── Result layer: did accept succeed or fail?
  └── Option layer: did the iterator produce an item at all?
```

The compiler desugars the code to the following:

```rust
let mut iter = listener.incoming();
loop {
    match iter.next() {                    // opens layer 1: Option
        Some(connection_result) => {       // connection_result: Result<TcpStream, ...>
            match connection_result {      // opens layer 2: Result
                Ok(stream) => { /* stream: TcpStream, finally the real thing */ }
                Err(error) => eprintln!("connection failed: {error}"),
            }
        }
        None => break,
    }
}
```

A package inside a package. One `match` can only open one layer:

- The **outer** `match iter.next()` opens the `Option` layer: item or finished? Matching `Some(connection_result)` peels off the `Some` and binds what was inside to `connection_result`. But what was inside is `Ok(TcpStream)`, still wrapped. So that `connection_result` is a `Result`, not a connection yet.
- The **inner** `match connection_result` opens the `Result` layer: good connection or failure? `Ok(stream)` peels the `Ok` and binds the naked `TcpStream`. *Now* it's a connection.




