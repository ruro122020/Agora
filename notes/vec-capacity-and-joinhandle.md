---
title: Vec Capacity and JoinHandle, What Growing and Waiting Cost
date: 2026-09-09
description: A Vec asks the allocator for memory only when a push would exceed its capacity, and each grow copies every element into a bigger block. A JoinHandle is the one value that refers to a spawned thread; dropping it does nothing to the thread, and only join parks the caller until that thread ends.
draft: false
---

Project: [Agora](https://github.com/ruro122020/Agora)


### Capacity is not a limit

```rust 
pub fn new(size: usize) -> ThreadPool {
    assert!( size > 0);
    let mut threads = Vec::with_capacity(size);
    for _ in 0..size { 

    }
    ThreadPool { threads }
}
```
Capacity is the current room, not a cap. `with_capacity(4)` asks the allocator (the part of the standard library that hands out heap blocks) once, at construction, for a block that holds 4. Pushes 1 to 4 fit without asking the allocator again. The fifth push overflows, so the `Vec` asks for a bigger block (implementations grow by roughly doubling, so about 8), copies the four elements across, frees the old block, and pushes. The next allocation comes when that block overflows, not every four elements.

`Vec::new()` starts with room for 0 and no heap block at all. The first push asks the allocator for a small block. After that it asks again only when the current block is full, growing by the same doubling. It does not ask on every push.

The rule in one line: a `Vec` asks the allocator only when a push would exceed its capacity, and each time it asks for more than it strictly needs so it will not have to ask again soon. `with_capacity` lets the first allocation be the right size. Both forms compile, both behave the same, and both would run out of memory the same way if pushed forever. At four elements the difference is not observable.

### Why start with room for four instead of none

Fewer allocations, and no copying. Each time a `Vec` outgrows its block it asks the allocator for a bigger block and copies every existing element from the old block to the new one. Two caveats. At four elements the gain is unmeasurable; the idiom matters at thousands of elements, where growing from zero means around a dozen reallocations and a full copy each time. The other reason is the reader. `with_capacity(size)` says "I know exactly how many are coming". `Vec::new()` says "I do not know". Here we know, so the code says so.

### What growing costs the system

Doubling bounds the count. A `Vec` that ends at 10,000 elements asked the allocator about 14 times, not 10,000, and copied about 20,000 elements in total across all the grows. What each grow costs:

- CPU time on copying the whole current contents. The last grow of a large `Vec` copies megabytes.
- A possible syscall. The allocator keeps its own pool of heap memory. When a big enough block is not in the pool it asks the kernel for more pages through `mmap` or `brk`. That is the same cost class as 2.3, a switch into the kernel and back. Small grows stay inside the allocator's pool; the big ones do not.
- Page faults. New pages from the kernel are not backed by physical memory until first touched. The first write to each page traps into the kernel, which finds a physical page and maps it. One small kernel entry per 4 KiB.
- Peak memory. During a grow the old block and the new block exist at once. A 100 MiB `Vec` briefly needs 150 MiB.
- Fragmentation. Each freed old block leaves a hole in the allocator's pool that a later, differently sized allocation may not fit. Over a long-running process the heap gets holey and the allocator asks the kernel for more even though total live data has not grown.
- Lock contention. With many threads allocating at once the allocator's shared state needs locking. Modern allocators keep per-thread pools to reduce this, but it is not free.


### What a JoinHandle is

```rust
pub struct ThreadPool{
    threads: Vec<thread::JoinHandle<()>>
}
```

A `JoinHandle` is the value `thread::spawn` gives back when it starts a thread. The thread runs off on its own; the handle is the one thing left that refers to it. It lets me call `join`, which parks the calling thread until the spawned thread has finished, then hands back whatever that thread returned. Our threads return nothing, so the type is `JoinHandle<()>`.

The calling thread is the thread that executes the `join` line. The spawned thread is the one `thread::spawn` created. In this program the calling thread is `main`'s thread, the one the operating system started the program on and the one the panic message called `thread 'main'`. Parked means the kernel takes `main`'s thread off the CPU and does not schedule it again until the spawned thread exits. It is the same state `main` sat in during `accept` in 1.4c, waiting on a different event. Nothing burns CPU while waiting. The spawned thread is not affected by `join` at all; it runs to its end, and that end is what wakes `main`.

Nothing in a running thread refers back to its `JoinHandle`, so dropping the handle has no effect on the thread. It does not notice and it does not stop. What ends it is something else entirely. `main` returning ends the process, and the kernel tears down every thread in a process when the process exits, mid-instruction if need be. The sequence is: handle dropped, thread unaffected, `main` returns, kernel kills all four. That gap between dropping a handle and actually waiting for the thread is what `join` closes.

Ownership chains: `ThreadPool` owns the `Vec`, the `Vec` owns each handle, so the handles live exactly as long as the pool does.
