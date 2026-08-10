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
  let mut buffer = [0u8; 512];
  match reader.read(&mut buffer){
    Ok(n) => println!("read {n} bytes"),
    Err(e) => eprintln!("read failed: {e}"),
  }
}