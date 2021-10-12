use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;

#[derive(Default)]
pub struct Error {
    total_error: f64,
    count: usize,
}

impl Error {
    pub fn push(&mut self, error: f64) {
        self.total_error += error;
        self.count += 1;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        self.total_error / self.count.max(1) as f64
    }

    #[must_use]
    pub async fn smooth(
        self,
        redis: &mut MultiplexedConnection,
        key: &str,
        smoothing: f64,
    ) -> crate::Result<(f64, f64)> {
        let old_error: Option<f64> = redis.get(key).await?;
        let average = self.average();
        let new_error = average * smoothing + old_error.unwrap_or(0.0) * (1.0 - smoothing);
        redis.set(key, new_error).await?;
        Ok((new_error, average))
    }
}
