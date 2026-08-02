---
title: The Drop Trait, Cleanup at the Closing Brace
date: 2026-08-01
description: The std library's Drop trait has one method, drop. The compiler is its only caller, inserting the call at the } that ends the owner's scope. For TcpStream it hands the kernel the fd, and the kernel closes that connected socket.
draft: false
---

Every connection the server accepts is something the kernel holds open on the
program's behalf, and it stays held until the program says it is finished with
it. Some languages leave that goodbye to the programmer, to be remembered on
every path through the code. Rust ties it to something that already exists on
every path: the end of the owner's scope. This note is how that works and who
does what.

The chain, top to bottom, with who supplies each piece:

```
}  of handle_connection          ← you wrote this
 └─ drop(stream) inserted here   ← the COMPILER adds this call, invisibly,
     │                             at every owner's scope end
     └─ runs TcpStream's drop    ← code written by the STD LIBRARY authors
         │                         as part of the standard library
         └─ close(4) syscall     ← tells the KERNEL: tear down this
                                   connection, ticket 4 is returned
```

Where the method comes from: a **trait** named `Drop`, defined in the standard
library (`std::ops::Drop`). A trait, in one clause, is a named capability that a
type can opt into by providing the required method. `Drop`'s entire definition is:

```rust
trait Drop {
    fn drop(&mut self);   // "when my owner's scope ends, run this"
}
```

And the standard library, in its own source code, opts `TcpStream` into it with
an implementation that boils down to:

```rust
impl Drop for /* the fd holder inside TcpStream */ {
    fn drop(&mut self) {
        close(self.fd);   // the syscall; the kernel does the actual teardown
    }
}
```

So three parties split the work: the std library authors wrote *what* cleanup
means for a `TcpStream` (close the fd), the compiler decides *when* it runs
(the owner's closing brace, automatically, no way to forget), and the kernel
does the *actual* teardown when the syscall arrives.

You never call `drop` yourself; the compiler's insertion is the only caller.
Writing `stream.drop();` is rejected by the compiler, because the automatic
call at the `}` is still coming, and running cleanup twice would hand the
kernel the same ticket number twice, after it may already have re-issued that
number to a different connection. When cleanup is genuinely needed early,
before the brace, the standard library gives the function spelling
`drop(stream)`: it takes ownership, so the value's scope ends right there, and
the automatic mechanism fires early instead of twice.
