//! End-to-end tests: a real server on a port the kernel picks (port 0), and a
//! raw `TcpStream` as the client so the bytes on the wire are what is tested.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use agora::{Method, Response, Router, ServerLimits, StatusCode, server};

fn start(limits: ServerLimits) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let router = Router::new()
        .route(Method::Get, "/", |_| {
            Response::text(StatusCode::OK, "hello")
        })
        .route(Method::Post, "/echo", |request| {
            Response::new(StatusCode::OK).body(request.body())
        })
        .route(Method::Get, "/panic", |_| {
            panic!("handler failure under test")
        });
    // The thread is never joined: `run` loops until the test process exits.
    thread::spawn(move || server::run(listener, router, limits, 2));
    addr
}

fn connect(addr: SocketAddr) -> TcpStream {
    let stream = TcpStream::connect(addr).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream
}

/// Sends `request`, then reads until the server closes the connection.
fn exchange(addr: SocketAddr, request: &[u8]) -> String {
    let mut stream = connect(addr);
    stream.write_all(request).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    String::from_utf8_lossy(&response).into_owned()
}

/// Reads exactly one response with a `Content-Length` off a kept-open stream.
fn read_response(stream: &mut TcpStream) -> String {
    let mut bytes = Vec::new();
    let mut byte = [0];
    while !bytes.ends_with(b"\r\n\r\n") {
        stream.read_exact(&mut byte).unwrap();
        bytes.push(byte[0]);
    }
    let head = String::from_utf8_lossy(&bytes).into_owned();
    let length: usize = head
        .lines()
        .find_map(|line| line.strip_prefix("Content-Length: "))
        .map(|value| value.parse().unwrap())
        .unwrap_or(0);
    let mut body = vec![0; length];
    stream.read_exact(&mut body).unwrap();
    head + &String::from_utf8_lossy(&body)
}

#[test]
fn get_returns_200_with_the_body() {
    let addr = start(ServerLimits::default());
    let response = exchange(
        addr,
        b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("Content-Length: 5\r\n"));
    assert!(response.contains("Connection: close\r\n"));
    assert!(response.ends_with("\r\n\r\nhello"));
}

#[test]
fn unknown_path_is_404_and_wrong_method_is_405() {
    let addr = start(ServerLimits::default());
    let response = exchange(
        addr,
        b"GET /nope HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
    let response = exchange(
        addr,
        b"DELETE / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 405 "), "{response}");
    assert!(response.contains("Allow: GET, HEAD\r\n"));
}

#[test]
fn head_sends_the_length_without_the_body() {
    let addr = start(ServerLimits::default());
    let response = exchange(
        addr,
        b"HEAD / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(response.contains("Content-Length: 5\r\n"), "{response}");
    assert!(response.ends_with("\r\n\r\n"));
}

#[test]
fn two_requests_share_one_connection() {
    let addr = start(ServerLimits::default());
    let mut stream = connect(addr);
    for _ in 0..2 {
        stream
            .write_all(b"GET / HTTP/1.1\r\nHost: x\r\n\r\n")
            .unwrap();
        let response = read_response(&mut stream);
        assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
        assert!(!response.contains("Connection: close"));
    }
}

#[test]
fn pipelined_requests_are_answered_in_order() {
    let addr = start(ServerLimits::default());
    let response = exchange(
        addr,
        b"POST /echo HTTP/1.1\r\nHost: x\r\nContent-Length: 3\r\n\r\noneGET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    let first = response.find("one").unwrap();
    let second = response.find("hello").unwrap();
    assert!(first < second, "{response}");
    assert_eq!(response.matches("HTTP/1.1 200 OK").count(), 2);
}

#[test]
fn keep_alive_cap_closes_the_connection() {
    let addr = start(ServerLimits {
        max_keep_alive_requests: 2,
        ..ServerLimits::default()
    });
    let response = exchange(
        addr,
        b"GET / HTTP/1.1\r\nHost: x\r\n\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\n",
    );
    assert_eq!(response.matches("HTTP/1.1 200 OK").count(), 2, "{response}");
    assert_eq!(response.matches("Connection: close").count(), 1);
}

#[test]
fn http10_closes_by_default() {
    let addr = start(ServerLimits::default());
    let response = exchange(addr, b"GET / HTTP/1.0\r\n\r\n");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("Connection: close\r\n"));
}

#[test]
fn post_with_fixed_and_chunked_bodies() {
    let addr = start(ServerLimits::default());
    let response = exchange(
        addr,
        b"POST /echo HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: 5\r\n\r\nfixed",
    );
    assert!(response.ends_with("\r\n\r\nfixed"), "{response}");

    // Sent in pieces so a chunk-size line and chunk data cross read calls.
    let mut stream = connect(addr);
    for piece in [
        &b"POST /echo HTTP/1.1\r\nHost: x\r\nConnection: close\r\nTransfer-Encoding: chunked\r\n\r\n4"[..],
        b"\r\nch",
        b"un\r\n3\r\nked\r",
        b"\n0\r\n\r\n",
    ] {
        stream.write_all(piece).unwrap();
        stream.flush().unwrap();
        thread::sleep(Duration::from_millis(20));
    }
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.ends_with("\r\n\r\nchunked"), "{response}");
}

#[test]
fn expect_continue_is_answered_before_the_body() {
    let addr = start(ServerLimits::default());
    let mut stream = connect(addr);
    stream
        .write_all(b"POST /echo HTTP/1.1\r\nHost: x\r\nConnection: close\r\nContent-Length: 2\r\nExpect: 100-continue\r\n\r\n")
        .unwrap();
    let mut interim = [0; 25];
    stream.read_exact(&mut interim).unwrap();
    assert_eq!(&interim, b"HTTP/1.1 100 Continue\r\n\r\n");
    stream.write_all(b"ok").unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).unwrap();
    assert!(response.ends_with("\r\n\r\nok"), "{response}");
}

#[test]
fn smuggling_attempt_gets_400_and_a_closed_socket() {
    let addr = start(ServerLimits::default());
    // `exchange` returning at all proves the server closed the connection.
    let response = exchange(
        addr,
        b"POST /echo HTTP/1.1\r\nHost: x\r\nContent-Length: 3\r\nTransfer-Encoding: chunked\r\n\r\n0\r\n\r\nGET / HTTP/1.1\r\nHost: x\r\n\r\n",
    );
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert_eq!(response.matches("HTTP/1.1 ").count(), 1, "{response}");
}

#[test]
fn oversized_body_is_413() {
    let addr = start(ServerLimits {
        max_body: 8,
        ..ServerLimits::default()
    });
    let response = exchange(
        addr,
        b"POST /echo HTTP/1.1\r\nHost: x\r\nContent-Length: 9\r\n\r\n123456789",
    );
    assert!(response.starts_with("HTTP/1.1 413 "), "{response}");
}

#[test]
fn garbage_is_400_not_a_crash() {
    let addr = start(ServerLimits::default());
    let response = exchange(addr, b"\x16\x03\x01\x02\x00\xff\xfe\r\n\r\n");
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    let response = exchange(
        addr,
        b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
}

#[test]
fn silent_client_is_disconnected_without_a_response() {
    let addr = start(ServerLimits {
        read_timeout: Duration::from_millis(200),
        ..ServerLimits::default()
    });
    let started = Instant::now();
    let response = exchange(addr, b"");
    assert_eq!(response, "");
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn client_that_stalls_mid_request_gets_408() {
    let addr = start(ServerLimits {
        read_timeout: Duration::from_millis(200),
        ..ServerLimits::default()
    });
    let response = exchange(addr, b"GET / HTTP/1.1\r\nHost: x\r\n");
    assert!(
        response.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "{response}"
    );
}

#[test]
fn panicking_handler_is_500_and_the_worker_survives() {
    let addr = start(ServerLimits::default());
    // More panics than workers: if a panic killed its worker, the pool of two
    // would be empty and the final request would hang.
    for _ in 0..3 {
        let response = exchange(
            addr,
            b"GET /panic HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
        );
        assert!(response.starts_with("HTTP/1.1 500 "), "{response}");
    }
    let response = exchange(
        addr,
        b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
}

#[test]
fn slow_client_does_not_block_the_others() {
    let addr = start(ServerLimits::default());
    // One worker of two is parked reading this unfinished request.
    let mut slow = connect(addr);
    slow.write_all(b"GET / HTTP/1.1\r\n").unwrap();
    let started = Instant::now();
    let response = exchange(
        addr,
        b"GET / HTTP/1.1\r\nHost: x\r\nConnection: close\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[test]
fn oversized_upload_is_refused_without_inviting_the_body() {
    let addr = start(ServerLimits {
        max_body: 8,
        ..ServerLimits::default()
    });
    let response = exchange(
        addr,
        b"POST /echo HTTP/1.1\r\nHost: x\r\nContent-Length: 9\r\nExpect: 100-continue\r\n\r\n",
    );
    assert!(response.starts_with("HTTP/1.1 413 "), "{response}");
    assert!(!response.contains("100 Continue"));
}
