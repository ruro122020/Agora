use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::conn;
use crate::limits::ServerLimits;
use crate::router::Router;
use crate::thread_pool::ThreadPool;

/// Accepts connections on `listener` forever, handing each to a pool of
/// `workers` threads. Returns only if the pool cannot be started.
pub fn run(
    listener: TcpListener,
    router: Router,
    limits: ServerLimits,
    workers: usize,
) -> io::Result<()> {
    let pool = ThreadPool::new(workers)?;
    // `Arc` (Atomically Reference Counted): each job gets its own pointer to
    // the one `Router`, which is freed when the last pointer is dropped. A
    // plain `&router` is refused because a job must be `'static`.
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
                // `accept` failing because the process is out of file
                // descriptors fails again immediately; do not spin on it.
                thread::sleep(Duration::from_millis(50));
            }
        }
    }
    Ok(())
}
