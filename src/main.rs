use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, BufReader};


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
  let mut request: Vec<u8> = Vec::new();
  let mut buffer = [0u8; 512];
  loop {
    match reader.read(&mut buffer){
        Ok(0) => {
            eprintln!("client disconnected before finishing the request");
            return;
        }
        Ok(n) => {
            println!("read {n} bytes");
            request.extend_from_slice(&buffer[..n]);
            if request.ends_with(b"\r\n\r\n"){
                break;
            }
        }
        Err(e) => {
            eprintln!("read failed: {e}");
            return;
        }
    }
  }
  let text = String::from_utf8_lossy(&request);
  println!("{text}");
  println!("request complete: {} bytes", request.len());

}