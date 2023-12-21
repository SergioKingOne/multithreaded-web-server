use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Message>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    NewJob(Job),
    Terminate,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let receiver = Arc::new(Mutex::new(receiver));

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);

        self.sender
            .as_ref()
            .unwrap()
            .send(Message::NewJob(job))
            .map_err(|e| Box::new(e) as _)
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.sender
                .as_ref()
                .unwrap()
                .send(Message::Terminate)
                .unwrap();
        }

        for worker in &mut self.workers {
            println!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv().unwrap();

            match message {
                Message::NewJob(job) => {
                    println!("Worker {id} got a job; executing.");
                    job();
                }
                Message::Terminate => {
                    println!("Worker {} was told to terminate.", id);
                    break;
                }
            }
        });

        Worker {
            id,
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    #[test]
    fn test_thread_pool_new() {
        // Test that a ThreadPool can be created with more than zero threads
        let pool = ThreadPool::new(5);
        assert_eq!(pool.workers.len(), 5);
    }

    #[test]
    #[should_panic]
    fn test_thread_pool_new_zero() {
        // Test that creating a ThreadPool with zero threads panics
        ThreadPool::new(0);
    }

    #[test]
    fn test_thread_pool_execute() {
        // Test that a ThreadPool can execute a job
        let pool = ThreadPool::new(5);
        let (tx, rx) = mpsc::channel();

        pool.execute(move || {
            tx.send(1).unwrap();
        })
        .unwrap();

        assert_eq!(rx.recv().unwrap(), 1);
    }

    #[test]
    fn test_thread_pool_multiple_execute() {
        // Test that a ThreadPool can execute multiple jobs
        let pool = ThreadPool::new(5);
        let (tx, rx) = mpsc::channel();

        for i in 0..10 {
            let tx = tx.clone();
            pool.execute(move || {
                tx.send(i).unwrap();
            })
            .unwrap();
        }

        drop(tx);

        let mut results: Vec<_> = rx.iter().collect();
        results.sort();

        assert_eq!(results, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn test_thread_pool_drop() {
        // Test that a ThreadPool shuts down its workers when dropped
        let pool = ThreadPool::new(5);
        let (tx, rx) = mpsc::channel();

        for _ in 0..10 {
            let tx = tx.clone();
            pool.execute(move || {
                thread::sleep(Duration::from_millis(100));
                tx.send(1).unwrap();
            })
            .unwrap();
        }

        drop(pool);

        // If the workers were not shut down, this would block indefinitely
        assert_eq!(rx.iter().take(10).fold(0, |sum, x| sum + x), 10);
    }
}
