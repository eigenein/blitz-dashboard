pub struct LearningRate {
    initial: f64,
    decay: f64,
    factor: f64,
    minimal: f64,
}

impl LearningRate {
    pub fn new(initial: f64, decay: f64, minimal: f64) -> Self {
        LearningRate {
            initial,
            decay,
            minimal,
            factor: 1.0,
        }
    }
}

impl Iterator for LearningRate {
    type Item = f64;

    fn next(&mut self) -> Option<Self::Item> {
        let rate = self.initial / self.factor;
        if rate >= self.minimal {
            self.factor += self.decay;
            Some(rate)
        } else {
            Some(self.minimal)
        }
    }
}
