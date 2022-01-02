use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::{sleep_until, Instant};

use crate::StdDuration;

#[derive(Clone)]
pub struct Throttler {
    period: StdDuration,
    next_instant: Arc<RwLock<Instant>>,
}

impl Throttler {
    pub fn new(period: StdDuration) -> Self {
        Self {
            period,
            next_instant: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn throttle(&self) {
        let mut next_instant = *self.next_instant.read().await;

        loop {
            // Wait for the next call instant to come.
            sleep_until(next_instant).await;
            // Attempting to update the next call instant.
            let mut guard = self.next_instant.write().await;
            if *guard == next_instant {
                // We succeeded to get the lock, update the next call instant.
                *guard = Instant::now() + self.period;
                // It also means the task is allowed to continue.
                break;
            }
            // Some other task has re-written the next call instant while we were sleeping.
            // We need to wait for the next available instant.
            next_instant = *guard;
        }
    }
}
