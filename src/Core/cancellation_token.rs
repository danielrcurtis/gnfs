// src/core/cancellation_token.rs

use std::sync::{Arc, Mutex, Condvar};

pub struct CancellationToken {
    is_cancelled: Arc<Mutex<bool>>,
    condvar: Arc<Condvar>,
}

impl CancellationToken {
    pub fn new() -> Self {
        CancellationToken {
            is_cancelled: Arc::new(Mutex::new(false)),
            condvar: Arc::new(Condvar::new()),
        }
    }

    pub fn is_cancellation_requested(&self) -> bool {
        *self.is_cancelled.lock().unwrap()
    }

    pub fn cancel(&self) {
        *self.is_cancelled.lock().unwrap() = true;
        self.condvar.notify_all();
    }

    pub fn wait(&self) {
        let mut is_cancelled = self.is_cancelled.lock().unwrap();
        while !*is_cancelled {
            is_cancelled = self.condvar.wait(is_cancelled).unwrap();
        }
    }

    pub fn register_callback<F>(&self, callback: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let token = self.clone();
        std::thread::spawn(move || {
            token.wait();
            callback();
        });
    }
}

impl Clone for CancellationToken {
    fn clone(&self) -> Self {
        CancellationToken {
            is_cancelled: self.is_cancelled.clone(),
            condvar: self.condvar.clone(),
        }
    }
}