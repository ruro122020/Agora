# Agora

A learning project: building a **web server in Rust** from scratch, as a way to
deeply understand how Rust works.

## What this is

Agora is a hands-on exercise in writing a web server using Rust. The name comes
from the *agora*, the public square of ancient Greek cities where people
gathered to exchange goods, news, and ideas. That is the idea here too: a
central place that receives requests and serves back responses.

The goal is not only a functioning server, but a solid mental model of *why*
Rust code is written the way it is: ownership, borrowing, lifetimes, error
handling, traits, and async (asynchronous) programming.

## Goals

- **Learn Rust properly.** Ownership, borrowing, lifetimes, error handling
  (`Result`/`Option`), traits, the async model and more.
- **Build a working web server.** Listen on a TCP port and accept HTTP requests.
- **Keep dependencies minimal and justified.** Every crate added to
  `Cargo.toml` should serve the learning process.

## Current status

Early scaffold. Right now the project is a fresh Cargo binary: `main.rs` prints
`Hello, world!` and there are no dependencies yet. The web server itself is
still to be built.

## Getting started

Prerequisites: a recent Rust toolchain installed via
[rustup](https://rustup.rs/).

```bash
# Build the project
cargo build

# Run it
cargo run

# Check without producing a binary (fast feedback loop)
cargo check

# Lint and format
cargo clippy
cargo fmt
```

## Planned direction

Rough order of what this project will grow into:

- A server that listens on a TCP port and responds to a basic HTTP request
- Routing: mapping a URL path
- Deploying the server on a Raspberry Pi running Linux

The exact crates are not chosen yet. Since this is a web server, the likely building blocks live at the HTTP and networking layer:
**`hyper`** (a low-level HTTP implementation), **`tokio`** (the async runtime it
runs on), **`tower`/`tower-http`** (service and middleware pieces such as static
file serving), and **`pingora`** (a toolkit for HTTP servers and reverse
proxies). Going straight to the standard library's **`std::net`** is also an
option for maximum learning with zero dependencies. The choice will be made deliberately,
weighing learning value and trade-offs rather than defaulting to the most
popular option.

## Project layout

```
Agora/
├── Cargo.toml      # Package manifest: metadata + dependencies
├── Cargo.lock      # Exact resolved dependency versions
├── src/
│   └── main.rs     # Entry point (currently "Hello, world!")
└── README.md       # This file
```
