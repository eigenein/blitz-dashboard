#[derive(Default)]
pub struct Error {
    error: f64,
    count: usize,
}

impl Error {
    #[inline]
    pub fn push(&mut self, prediction: f64, is_win: bool) {
        let argument = if is_win { prediction } else { 1.0 - prediction };
        self.error -= argument.ln();
        self.count += 1;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        (self.error / self.count.max(1) as f64).sqrt()
    }
}
