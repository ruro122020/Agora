---
title: Closures, the Sealed Box a Thread Can Open
date: 2026-08-20
description: A closure is like a sealed box holding instructions plus the values it captured. 
draft: false
---

When I was implementing the ThreadPool in my web server, I found it difficult to understand closures in Rust so now I want to write about it. 

My ThreadPool creates 4 threads that sit waiting. The execute method puts a job on a queue, and the first free thread takes the job and runs it.

The closure is the job itself. It's treated as a value I can hand around. 

When my main writes:

```rust
pool.execute(move || handle_connection(stream));
```

I think of the closure as a sealed box. Inside the box are two things: instructions ("call handle_connection") and the one tool those instructions need (the stream, which move sealed inside). The box is a real value, like a String or a Vec. It can be stored, passed to a function, or sent to another thread. Under the hood the compiler builds a hidden struct holding the captured stream and calls that struct like a function. The whole point of the pool is to carry these boxes from main (which makes them) to a worker thread (which opens them).

```rust
pub fn execute<F>(&self, f: F)
```

The generic `<F>` exists because every box has a different type. The compiler creates a distinct hidden struct for each closure I write so none of them has a name I can type, and no two are the same type. So the execute method can't declare a parameter "of type closure", because no such single type exists. Instead execute says, this function has a blank in it, called F, and whatever type the caller's box happens to be, the compiler fills the blank with it. That's all what a generic is. It's a fill-in-the-blank type, resolved per caller at compile time.

The traits are the conditions on the blank. An unrestricted blank would accept anything, including a number, and execute couldn't do anything with it, since the compiler only lets me use whatever the conditions guarantee. So:

```rust
where F: FnOnce() + Send + 'static
```

is a checklist (the bounds) the box must satisfy, three items:

- FnOnce() (trait): the box can be opened and its instructions run, at least once, with no arguments. This is what makes it a runnable job rather than arbitrary data.
- Send (trait): the box is safe to hand to a different thread. Nearly everything is, the compiler tracks the rare exceptions.
- 'static (lifetime): everything in the box is owned by the box, no strings leading back to variables in the caller's scope. The compiler rejects that at the signature.
P.S. Traits and lifetime together is called "bounds", as in "the bounds on F". 

I write `move` to satisfy the `'static` item on the checklist. Without move, my closure does not take ownership of stream. It only holds a borrow, which is a pointer back to a variable in the loop iteration's scope. A closure holding a borrow fails the 'static requirement because the variable it points to disappears when the loop iteration ends. With `move`, the stream is sealed inside the box, the box owns everything it contains, and it passes the checklist. 

`where` is just the place the checklist goes. It attaches conditions to a generic blank: "this function works for any type F, where F meets these requirements."

It adds nothing I couldn't write inline; these two are identical:

```rust
pub fn execute<F: FnOnce() + Send + 'static>(&self, f: F)
```

```rust
pub fn execute<F>(&self, f: F)
where
  F: FnOnce() + Send + 'static,
```
The first crams the checklist inside the angle brackets; the second moves it out to its own line. `where` exists purely for readability when the checklist is long or there are several blanks. The compiler treats them the same: any caller whose type fails the checklist gets rejected at compile time.
