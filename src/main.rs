use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write, BufReader};
use std::fs;

fn main() -> io::Result<()>{
  let listener = TcpListener::bind("127.0.0.1:7878")?;
  for stream in listener.incoming() {
    match stream {
      Ok(stream) => {
        handle_connection(stream);
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
  let response = match build_response() {
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

fn build_response() -> io::Result<String>{
  let contents = fs::read_to_string("./dist/hello.html")?;
  let status = match fetch_api("/health"){
    Ok(body) => body,
    Err(e) => format!("API is down: {e}"),
  };
  let contents = contents.replace("{{API}}", status.trim());
  let length = contents.len();
  Ok(format!("HTTP/1.1 200 OK\r\nContent-Length: {length}\r\n\r\n{contents}"))
}

fn fetch_api(path: &str) -> io::Result<String>{
  let mut stream = TcpStream::connect("127.0.0.1:3000")?;
  let request = format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1:3000\r\nConnection: close\r\n\r\n");
  stream.write_all(request.as_bytes())?;
  let mut response = String::new();
  stream.read_to_string(&mut response)?;
  let body = match response.find("\r\n\r\n"){
    Some(i)=> response[i + 4..].to_string(),
    None => String::new(),
  };
  Ok(body)
}