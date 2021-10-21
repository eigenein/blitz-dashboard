#[derive(Default)]
pub struct Error {
    error: f64,
    count: usize,
}

impl Error {
    pub fn push(&mut self, residual_error: f64) {
        self.error += residual_error * residual_error;
        self.count += 1;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        (self.error / self.count.max(1) as f64).sqrt()
    }
}
