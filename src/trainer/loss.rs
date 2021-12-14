use crate::Float;

/// Binary cross-entropy loss.
#[derive(Default, Copy, Clone)]
pub struct BCELoss {
    loss: Float,
    n: i32,
}

impl BCELoss {
    #[inline]
    pub fn push_sample(&mut self, prediction: Float, label: Float) {
        if label.abs() > Float::EPSILON {
            self.loss -= label * prediction.ln();
        }
        let inverse_label = 1.0 - label;
        if inverse_label.abs() > Float::EPSILON {
            self.loss -= inverse_label * (1.0 - prediction).ln();
        }
        self.n += 1;
    }

    #[must_use]
    pub fn finalise(&self) -> Float {
        (self.loss / self.n.max(1) as Float).sqrt()
    }
}

#[derive(Copy, Clone)]
pub struct LossPair {
    pub train: Float,
    pub test: Float,
}

impl LossPair {
    #[must_use]
    pub fn builder() -> LossPairBuilder {
        LossPairBuilder::default()
    }

    #[must_use]
    pub fn infinity() -> Self {
        Self {
            train: Float::INFINITY,
            test: Float::INFINITY,
        }
    }

    #[must_use]
    pub fn is_finite(&self) -> bool {
        self.train.is_finite() && self.test.is_finite()
    }
}

#[derive(Default)]
pub struct LossPairBuilder {
    pub train: BCELoss,
    pub test: BCELoss,
}

impl LossPairBuilder {
    #[must_use]
    pub fn finalise(self) -> LossPair {
        LossPair {
            train: self.train.finalise(),
            test: self.test.finalise(),
        }
    }
}
