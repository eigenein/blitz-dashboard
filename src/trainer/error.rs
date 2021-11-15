#[derive(Default)]
pub struct Error {
    error: f64,
    weight: f64,
}

impl Error {
    #[inline]
    pub fn push(&mut self, prediction: f64, label: f64, weight: f64) {
        if label.abs() > f64::EPSILON {
            self.error -= weight * label * prediction.ln();
        }
        let inverse_label = 1.0 - label;
        if inverse_label.abs() > f64::EPSILON {
            self.error -= weight * inverse_label * (1.0 - prediction).ln();
        }
        self.weight += weight;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        (self.error / self.weight.max(1.0)).sqrt()
    }
}
