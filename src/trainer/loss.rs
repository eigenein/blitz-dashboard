/// Binary cross-entropy loss.
#[derive(Default)]
pub struct BCELoss {
    loss: f64,
    n: i32,
}

impl BCELoss {
    #[inline]
    pub fn push_sample(&mut self, prediction: f64, label: f64) {
        if label.abs() > f64::EPSILON {
            self.loss -= label * prediction.ln();
        }
        let inverse_label = 1.0 - label;
        if inverse_label.abs() > f64::EPSILON {
            self.loss -= inverse_label * (1.0 - prediction).ln();
        }
        self.n += 1;
    }

    #[must_use]
    pub fn average(&self) -> f64 {
        (self.loss / self.n.max(1) as f64).sqrt()
    }
}
