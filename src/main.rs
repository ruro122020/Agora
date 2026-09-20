use std::fs;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::time::Duration;

use agora::{Method, Response, Router, ServerLimits, StatusCode, server};

const API_ADDR: ([u8; 4], u16) = ([127, 0, 0, 1], 3000);
const API_TIMEOUT: Duration = Duration::from_secs(5);

fn main() -> io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:7878")?;
    println!("listening on http://{}", listener.local_addr()?);

    let router = Router::new()
        .route(Method::Get, "/", |_| iris_page("GET", "/health"))
        .route(Method::Get, "/on", |_| iris_page("POST", "/on"))
        .route(Method::Post, "/on", |_| iris_page("POST", "/on"))
        .route(Method::Get, "/off", |_| iris_page("POST", "/off"))
        .route(Method::Post, "/off", |_| iris_page("POST", "/off"));

    server::run(listener, router, ServerLimits::default(), 4)
}

fn iris_page(method: &str, path: &str) -> Response {
    let status = match fetch_api(method, path) {
        Ok(body) => body,
        Err(e) => format!("API is down: {e}"),
    };
    match fs::read_to_string("./dist/hello.html") {
        Ok(contents) => Response::html(StatusCode::OK, contents.replace("{{API}}", status.trim())),
        Err(e) => {
            eprintln!("failed to read hello.html: {e}");
            Response::text(
                StatusCode::INTERNAL_SERVER_ERROR,
                "500 Internal Server Error\n",
            )
        }
    }
}

fn fetch_api(method: &str, path: &str) -> io::Result<String> {
    // Timeouts on all three waits, so a hung API cannot hold a worker forever.
    let mut stream = TcpStream::connect_timeout(&SocketAddr::from(API_ADDR), API_TIMEOUT)?;
    stream.set_read_timeout(Some(API_TIMEOUT))?;
    stream.set_write_timeout(Some(API_TIMEOUT))?;
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    let body = match response.find("\r\n\r\n") {
        Some(i) => response[i + 4..].to_string(),
        None => String::new(),
    };
    Ok(body)
}
