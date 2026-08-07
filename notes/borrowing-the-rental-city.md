---
title: Many Readers, One Mover
date: 2026-08-06
description: This is about the borrow checker that allows many readers or one change permit, never both, and it's enforced by the compiler reading the whole script before the program ever runs.
draft: false
---

## Section 1: the crash that never ran 🦀 ⚙️

```rust
let mut s = String::from("hi");
let r = &s;
s.push_str("!");
println!("{r}");
```

**Line 1.** `s` signs a lease. On the stack, `s` is only a rental file with three numbers: the unit's address, how many bytes are stored there, how many fit. The bytes themselves live in the unit: heap memory, one piece of the city block the program rents through its property manager, the allocator. Today the manager's ledger gains a line: *unit `0x5000`, 2 bytes, rented*. The letters `h` `i` move in.

**Line 2.** `r` takes a copy of the address. That is the entire event. At runtime a reference is one number, `0x5000`, and a copy of an address records nothing else: not who the tenant is, not when the lease ends, no way to reach whoever knows it if anything changes. Remember how little that is. The whole story turns on it.

**Line 3.** `push_str("!")` needs a third byte and the unit fits two. The neighboring bytes belong to another tenant, so growing in place is not an option; this is moving day. The manager rents `s` a bigger unit at `0x6000`, the letters are carried over, `!` joins them, and the old lease ends. Watch what ending a lease means, because it is quieter than it sounds: nobody clears out unit `0x5000`. The letters `h` `i` still sit there like abandoned furniture. The only change in the world is one ledger line flipping to *vacant*.

And nobody who knows the address is told. There is no way to know who knows it.

**Line 4.** `println!` follows the address to `0x5000`. Vacant units have no locks. What it finds inside depends entirely on what the manager did with the unit in the meantime, and there are exactly three possibilities:

1. **Still vacant.** The old furniture is untouched, and the program prints `hi`. It looks correct. It will pass every test I run today, and the bug ships.
2. **Re-rented.** The manager leased `0x5000` to another part of the program, and `println!` prints whatever the new tenant stored: a fragment of a request buffer, half a password, noise.
3. **Returned to the city.** The manager gave the whole block back to the kernel, and the address no longer belongs to my program at all. The city does the one merciful thing in this story: it evicts the trespasser on the spot and kills the process.

Read the list again and notice the trap. The crash, ticket 3, is the *good* ending: loud, immediate, pointing straight at the crime scene. Ticket 1 is the one that reaches production and prints `hi` for six months, until the day the manager re-rents the unit. This failure has a name, **use-after-free**: following an address after the lease ended. E0502 is the compiler having read all the way here, seen all three endings, and torn the program up at line 3 so none of them can happen.

## Section 2: the rule the property manager cannot enforce 🦀

First, understand why anyone needs a rule at all. Addresses spread the way secrets do: anyone who sees one can copy it, and there is no record of who knows it. The manager's ledger tracks units and leases; it has no page listing who holds copies of an address, and when a lease ends, nobody who knows the old address is told, because no list of them exists. An address, once given out, cannot be taken back. At runtime, nothing in the city polices any of this. That is the hole the borrow checker exists to fill, and since it cannot be filled at runtime, it gets filled before the program ever runs.

The rule the compiler enforces: at any moment a value can have **either** any number of read-only copies of its address in circulation (`&`), **or** exactly one change permit out (`&mut`). Many readers or one writer, never both.

The shape of the rule comes straight from moving day. Looking is harmless, and looks can overlap; a hundred people knowing the address is a hundred harmless visits. But a *change* might be moving day, and moving day strands every copy of the old address at once. So the two cannot coexist: while copies circulate, nobody may change the unit; while the change permit is out, no copies may circulate.

Watched in code, the legal version first, where they take turns:

```rust
let mut s = String::from("hi");
let a = &s;
let b = &s;          // two copies of the address out: fine, they only look
println!("{a} {b}"); // last use of both copies: they leave the story here
let m = &mut s;      // no copies left, so the change permit can be issued
m.push_str("!");
```

And the version the compiler tears up, a copy still live across a change:

```rust
let mut s = String::from("hi");
let r = &s;        // a copy of the address is taken
s.push_str("!");   // possible moving day, while the copy is out
println!("{r}");   // the copy is still used down here, so it is live up there
```

```
error[E0502]: cannot borrow `s` as mutable because it is also borrowed as immutable
 --> src/main.rs:9:3
  |
8 |   let r = &s;
  |           -- immutable borrow occurs here
9 |   s.push_str("!");
  |   ^^^^^^^^^^^^^^^ mutable borrow occurs here
10|   println!("{r}");
  |              - immutable borrow later used here
```

Notice that the `push_str` line never says `&mut`. It does not have to: `push_str` is declared as taking `&mut self`, so calling it requisitions the change permit automatically. Methods borrow the value they are called on without you writing the `&`.

The rule also runs in the direction nobody expects: while the change permit is out, even *reading* through the owner is refused.

```rust
let mut s = String::from("hi");
let m = &mut s;
println!("{s}");   // just looking! refused anyway
m.push_str("!");
```

```
error[E0502]: cannot borrow `s` as immutable because it is also borrowed as mutable
 --> src/main.rs:4:14
  |
3 |   let m = &mut s;
  |           ------ mutable borrow occurs here
4 |   println!("{s}");
  |              ^ immutable borrow occurs here
5 |   m.push_str("!");
  |   - mutable borrow later used here
```

Same error number with the two words swapped: E0502 is the overlap, in either direction. And the reason "just looking" loses is that in Rust there is no such thing as looking without knowing where to look. `println!("{s}")` quietly takes its own copy of the address, a `&s` that exists for the duration of the print. So reading while `m` is out means a copy and the change permit circulating together, exactly the forbidden pair. It only looks like a different case because this copy is invisible.

## Section 3: the script reader 🦀

Inside the running city there is nothing to know: an address is bare information, it never expires, and no one is watching who knows it. The compiler is outside the city. It reads the entire function the way a script reader reads a play, before opening night, with every line visible at once.

Its procedure is plain. For each copy of the address, mark every line of the script that uses it. The last marked line is where that copy leaves the story. From the moment the copy is taken to its last use, the unit is frozen: no changes, no change permit. Past the last use, the compiler behaves as if the copy had never existed.

Which means the closing brace is irrelevant, and that is provable. Reorder the example so the look happens before moving day, and the same three statements compile:

```rust
let mut s = String::from("hi");
let r = &s;
println!("{r}");   // last use: this copy of the address leaves the story here
s.push_str("!");   // no copies in circulation, moving day proceeds
```

Between taking `r` and its last use, the compiler refuses every way of changing `s`: a `&mut self` method, an assignment, a new `&mut`. It is not about which variable name appears in the offending line, it is about which unit that line would touch, and only that unit; every other variable in the function is free. `r` itself is never the protected thing. `r` is only someone who knows the address. The rule protects the unit from changing while anyone who knows the address can still walk over and look.
