#[derive(Default)]
pub struct Error {
    error: f64,
    n: i32,
}

impl Error {
    #[inline]
    pub fn push(&mut self, prediction: f64, label: f64) {
        if label.abs() > f64::EPSILON {
            self.error -= label * prediction.ln();
        }
        let inverse_label = 1.0 - label;
        if inverse_label.abs() > f64::EPSILON {
            self.error -= inverse_label * (1.0 - prediction).ln();
        }
        self.n += 1;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        (self.error / self.n.max(1) as f64).sqrt()
    }
}
