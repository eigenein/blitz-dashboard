use std::sync::Arc;

use tokio::sync::Mutex;
use tokio::task::yield_now;
use tokio::time::{sleep_until, Instant};

use crate::StdDuration;

#[derive(Clone)]
pub struct Throttler {
    period: StdDuration,
    counter: Arc<Mutex<Instant>>,
}

impl Throttler {
    pub fn new(period: StdDuration) -> Self {
        Self {
            period,
            counter: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn throttle(&self) {
        let mut guard = self.counter.lock().await;
        let deadline = *guard + self.period;
        sleep_until(deadline).await;
        while Instant::now() < deadline {
            yield_now().await;
        }
        *guard = deadline;
    }
}
