#[derive(Default)]
pub struct Average {
    sum: f64,
    count: usize,
}

impl Average {
    pub fn push(&mut self, value: f64) {
        self.count += 1;
        self.sum += value;
    }

    pub fn average(&self) -> f64 {
        self.sum / self.count as f64
    }
}
