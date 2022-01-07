use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::task::yield_now;
use tokio::time::{sleep_until, Instant};

use crate::StdDuration;

#[derive(Clone)]
pub struct Throttler {
    period: StdDuration,
    instant: Arc<RwLock<Instant>>,
}

impl Throttler {
    pub fn new(period: StdDuration) -> Self {
        Self {
            period,
            instant: Arc::new(RwLock::new(Instant::now())),
        }
    }

    pub async fn throttle(&self) {
        loop {
            let read_instant = *self.instant.read().await;
            let deadline = read_instant + self.period;

            sleep_until(deadline).await;
            while Instant::now() < deadline {
                yield_now().await;
            }

            let mut instant = self.instant.write().await;
            if *instant == read_instant {
                *instant = deadline;
                break;
            }
        }
    }
}
