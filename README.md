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

A working blocking HTTP/1.1 server on `std::net`, with zero dependencies:

- Request parsing into typed values (`Method`, `Request`, `ParseError`), with
  hostile input returning an error status instead of crashing a thread
- Body framing per RFC 9112 section 6.3: `Content-Length`, chunked bodies, and
  rejection of requests that carry both (request smuggling)
- Limits and timeouts on everything a client controls: request line, header
  count and size, body size, read and write waits, requests per connection
- Keep-alive, `HEAD`, `Expect: 100-continue`, `404` and `405` with `Allow`
- A router mapping (method, path) to a handler closure
- A hand-built fixed-size thread pool that survives panicking handlers

It listens on `127.0.0.1:7878` and serves `dist/hello.html`, filled in with the
reply of an API expected on `127.0.0.1:3000`.

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
cargo clippy -- -D warnings
cargo fmt

# Unit and integration tests
cargo test

# With the server running, in a second shell
curl -v http://127.0.0.1:7878/
```

## Planned direction

- Write an `epoll` event loop by hand, then port to `tokio`, so the async
  runtime reads as that loop generalized instead of as a black box
- Deploy the server on a Raspberry Pi running Linux

`hyper`, `tower` and `pingora` were considered and rejected: they hand over an
already-parsed request, which hides the accept loop, the buffering, and the
parsing that this project exists to understand. The reasoning is in
`CLAUDE.md` under "What We Are Building".

## Project layout

```
Agora/
├── Cargo.toml
├── dist/hello.html      # the page served at /
├── src/
│   ├── main.rs          # binary: bind the port, register routes, run
│   ├── lib.rs           # library root
│   ├── server.rs        # accept loop handing connections to the pool
│   ├── conn.rs          # one connection: timeouts, keep-alive loop, error to status
│   ├── limits.rs        # ServerLimits
│   ├── error.rs         # ServerError
│   ├── router.rs        # (method, path) to handler
│   ├── thread_pool.rs   # fixed-size worker pool
│   └── http/            # Method, StatusCode, Headers, BodyLength, Request, Response, ParseError
├── tests/server.rs      # end-to-end tests over real sockets
├── notes/               # one note per concept learned
└── tasks/               # todo.md (plan and progress), design.md, lessons.md
```
