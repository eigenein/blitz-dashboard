use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;
use tokio::time::sleep;

use crate::StdDuration;

#[derive(Clone)]
pub struct Throttler {
    period: StdDuration,
    limit: usize,

    /// Stores `(start_instant, n_requests)`.
    counter: Arc<Mutex<(Instant, usize)>>,
}

impl Throttler {
    pub fn new(period: StdDuration, limit: usize) -> Self {
        Self {
            period,
            limit,
            counter: Arc::new(Mutex::new((Instant::now(), 0))),
        }
    }

    pub async fn throttle(&self) {
        let mut guard = self.counter.lock().await;

        if guard.1 >= self.limit {
            let deadline = guard.0 + self.period;
            if let Some(duration) = deadline.checked_duration_since(Instant::now()) {
                sleep(duration).await;
            }
            *guard = (deadline, 1);
        } else {
            guard.1 += 1;
        }
    }
}
