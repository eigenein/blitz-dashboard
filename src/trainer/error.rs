#[derive(Default)]
pub struct Error {
    error: f64,
    count: usize,
}

impl Error {
    pub fn push(&mut self, prediction: f64, is_win: bool) {
        self.error -= if is_win {
            prediction.ln()
        } else {
            (1.0 - prediction).ln()
        };
        self.count += 1;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        (self.error / self.count.max(1) as f64).sqrt()
    }
}
