use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::conn;
use crate::limits::ServerLimits;
use crate::router::Router;
use crate::thread_pool::ThreadPool;

pub fn run(
    listener: TcpListener,
    router: Router,
    limits: ServerLimits,
    workers: usize,
) -> io::Result<()> {
    let pool = ThreadPool::new(workers)?;
    let router = Arc::new(router);
    let limits = Arc::new(limits);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let router = Arc::clone(&router);
                let limits = Arc::clone(&limits);
                pool.execute(move || conn::handle_connection(stream, &router, &limits));
            }
            Err(error) => {
                eprintln!("connection failed: {error}");
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Ok(())
}
