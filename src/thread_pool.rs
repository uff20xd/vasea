use std::{
    thread,
    sync::{
        atomic::AtomicBool,
        Mutex,
        Arc,
        Condvar,
    },
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    job_queue: Arc<Mutex<Vec<Job>>>,
    job_signal: Arc<(Mutex<bool>, Condvar)>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        todo!("Implement ThreadPool::new()")
    }
}

struct Worker {
    working: AtomicBool,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(
        job_queue: Arc<Mutex<Vec<Job>>>,
        job_signal: Arc<(Mutex<bool>, Condvar)>,
        ) -> Self {
        todo!("Implement Worker::new()")
    }
}

type Job = Box<dyn FnOnce() -> Result<(), Box<dyn std::error::Error>> + Send + 'static>;
