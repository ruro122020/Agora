use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::panic::{self, AssertUnwindSafe};
use std::time::Duration;

use crate::error::ServerError;
use crate::http::{BodyLength, Method, ParseError, Request, Response, StatusCode};
use crate::limits::ServerLimits;
use crate::router::Router;

pub fn handle_connection(stream: TcpStream, router: &Router, limits: &ServerLimits) {
    if let Err(e) = serve(stream, router, limits) {
        eprintln!("connection ended with an error: {e}");
    }
}

fn serve(stream: TcpStream, router: &Router, limits: &ServerLimits) -> io::Result<()> {
    stream.set_read_timeout(Some(limits.read_timeout))?;
    stream.set_write_timeout(Some(limits.write_timeout))?;
    let mut reader = BufReader::new(stream);

    for served in 1..=limits.max_keep_alive_requests {
        match reader.fill_buf() {
            Ok([]) => return Ok(()),
            Ok(_) => {}
            Err(e) if is_timeout(&e) => return Ok(()),
            Err(e) => return Err(e),
        }

        let request = match read_request(&mut reader, limits) {
            Ok(request) => request,
            Err(error) => return reject(reader.get_mut(), &error),
        };

        let close = !request.keep_alive() || served == limits.max_keep_alive_requests;
        let response = panic::catch_unwind(AssertUnwindSafe(|| router.handle(&request)))
            .unwrap_or_else(|_| {
                Response::text(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "500 Internal Server Error\n",
                )
            });
        println!(
            "{} {} -> {}",
            request.method(),
            request.target(),
            response.status().as_u16()
        );

        let head_only = request.method() == Method::Head;
        response.write_to(reader.get_mut(), head_only, close)?;
        if close {
            return Ok(());
        }
    }
    Ok(())
}

fn read_request(
    reader: &mut BufReader<TcpStream>,
    limits: &ServerLimits,
) -> Result<Request, ServerError> {
    let mut request = Request::read_head(reader, limits)?;
    if matches!(request.body_length(), BodyLength::Fixed(n) if n > limits.max_body) {
        return Err(ParseError::BodyTooLarge.into());
    }
    if request.expects_continue() {
        reader
            .get_mut()
            .write_all(b"HTTP/1.1 100 Continue\r\n\r\n")?;
    }
    request.read_body(reader, limits)?;
    Ok(request)
}

fn reject(stream: &mut TcpStream, error: &ServerError) -> io::Result<()> {
    let status = match error {
        ServerError::Parse(e) => e.status(),
        ServerError::Io(e) if is_timeout(e) => StatusCode::REQUEST_TIMEOUT,
        ServerError::Io(_) => return Ok(()),
    };
    eprintln!("rejected request: {error} -> {}", status.as_u16());
    Response::text(status, format!("{status}\n")).write_to(stream, false, true)?;
    linger(stream)
}

fn linger(stream: &mut TcpStream) -> io::Result<()> {
    stream.shutdown(Shutdown::Write)?;
    stream.set_read_timeout(Some(Duration::from_millis(500)))?;
    let mut sink = [0; 4096];
    for _ in 0..64 {
        match stream.read(&mut sink) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
    Ok(())
}

fn is_timeout(error: &io::Error) -> bool {
    matches!(
        error.kind(),
        io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut
    )
}
