use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write, BufReader};
use std::fs;
use std::thread;

fn main() -> io::Result<()>{
  let listener = TcpListener::bind("127.0.0.1:7878")?;
  for stream in listener.incoming() {
    match stream {
      Ok(stream) => {
        thread::spawn(move || handle_connection(stream));
      }
      Err(error) => eprintln!("connection failed: {error}"),
    }
  } 
  Ok(())
}

fn handle_connection(stream: TcpStream){
  let mut reader = BufReader::new(stream);
  let request = match read_request(&mut reader) {
    Ok(bytes) => bytes,
    Err(e) => {
      eprintln!("failed to read request: {e}");
      return;
    }
  };
  println!("request complete: {} bytes", request.len());
  let path = request_path(&request);
  let response = match build_response(&path) {
    Ok(response) => response,
    Err(e) => {
      eprintln!("failed to build response: {e}");
      return;
    }
  };
  match reader.get_mut().write_all(response.as_bytes()){
    Ok(()) => println!("wrote {} bytes", response.len()),
    Err(e) => eprintln!("write failed: {e}"),
  }
}

fn read_request(reader:&mut BufReader<TcpStream>) -> io::Result<Vec<u8>> {
  let mut request: Vec<u8> = Vec::new();
  let mut buffer = [0u8; 512];
  loop {
    match reader.read(&mut buffer)? {
      0 => {
        return Err(io::Error::new(
          io::ErrorKind::UnexpectedEof,
          "client disconnected before finishing the request",
        ));
      }
      n => {
        println!("read {n} bytes");
        request.extend_from_slice(&buffer[..n]);
        if request.ends_with(b"\r\n\r\n") {
          return Ok(request);
        }
      }
    }
  } 
}

fn build_response(path: &str) -> io::Result<String>{
  let result = match path {
    "/on" => fetch_api("POST", "/on"),
    "/off" => fetch_api("POST", "/off"),
    _ => fetch_api("GET", "/health"),
  };
  let status = match result {
    Ok(body) => body,
    Err(e) => format!("API is down: {e}"),
  };
  let contents = fs::read_to_string("./dist/hello.html")?;
  let contents = contents.replace("{{API}}", status.trim());
  let length = contents.len();
  Ok(format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{contents}"))
}

fn fetch_api(method:&str, path: &str) -> io::Result<String>{
  let mut stream = TcpStream::connect("127.0.0.1:3000")?;
  let request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nContent-Length:0\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes())?;
  let mut response = String::new();
  stream.read_to_string(&mut response)?;
  let body = match response.find("\r\n\r\n"){
    Some(i)=> response[i + 4..].to_string(),
    None => String::new(),
  };
  Ok(body)
}

fn request_path(request:&[u8]) -> String {
  let text = String::from_utf8_lossy(request);
  let first_line = text.lines().next().unwrap_or("");
  let path = first_line.split_whitespace().nth(1).unwrap_or("/");
  path.to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn empty_input_falls_back_to_root(){
    assert_eq!(request_path(b""), "/");
  }

  #[test]
  fn missing_path_falls_back_to_root(){
    assert_eq!(request_path(b"GET\r\n\r\n"), "/");
  }

  #[test]
  fn invalid_utf8_does_not_panic(){
    let request = b"GET /\xFF\xFE HTTP/1.1\r\n\r\n";
    assert_eq!(request_path(request), "/\u{FFFD}\u{FFFD}");
  }

  #[test]
  fn request_path_extracts_the_path(){
    let request = b"GET /on HTTP/1.1\r\nHost: x\r\n\r\n";
    assert_eq!(request_path(request), "/on");
  }
}