// src/core/cancellation_token.rs

use std::sync::{Condvar, Mutex};

pub struct CancellationToken {
    is_cancelled: bool,
    cancel_callback: Option<Box<dyn Fn()>>,
    cancel_callback_mutex: Mutex<()>,
    cancel_callback_condvar: Condvar,
    is_cancelled_mutex: Mutex<()>,
    is_cancelled_condvar: Condvar,
}

