use std::io;
use std::panic::{self, AssertUnwindSafe};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> io::Result<ThreadPool> {
        assert!(size > 0);
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);
        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver))?);
        }
        Ok(ThreadPool {
            workers,
            sender: Some(sender),
        })
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let Some(sender) = &self.sender else {
            return;
        };

        if sender.send(Box::new(f)).is_err() {
            eprintln!("thread pool has no live workers, job dropped");
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take()
                && thread.join().is_err()
            {
                eprintln!("worker {} panicked", worker.id);
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<Receiver<Job>>>) -> io::Result<Worker> {
        let thread = thread::Builder::new()
            .name(format!("worker-{id}"))
            .spawn(move || Worker::run(id, &receiver))?;
        Ok(Worker {
            id,
            thread: Some(thread),
        })
    }

    fn run(id: usize, receiver: &Mutex<Receiver<Job>>) {
        loop {
            let message = receiver
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .recv();
            let Ok(job) = message else {
                break;
            };
            if panic::catch_unwind(AssertUnwindSafe(job)).is_err() {
                eprintln!("worker {id}: job panicked");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn runs_every_job_before_drop_returns() {
        let counter = Arc::new(AtomicUsize::new(0));
        let pool = ThreadPool::new(4).unwrap();
        for _ in 0..32 {
            let counter = Arc::clone(&counter);
            pool.execute(move || {
                counter.fetch_add(1, Ordering::SeqCst);
            });
        }
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 32);
    }

    #[test]
    fn survives_a_panicking_job() {
        let counter = Arc::new(AtomicUsize::new(0));
        let pool = ThreadPool::new(1).unwrap();
        pool.execute(|| panic!("job failure under test"));
        let after = Arc::clone(&counter);
        pool.execute(move || {
            after.fetch_add(1, Ordering::SeqCst);
        });
        drop(pool);
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[should_panic]
    fn zero_workers_is_rejected() {
        let _ = ThreadPool::new(0);
    }
}
