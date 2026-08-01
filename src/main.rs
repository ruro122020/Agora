use std::net::{TcpListener, TcpStream};
use std::io;


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
  println!("connection established")
}