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
}
