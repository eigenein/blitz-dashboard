use std::ops::Deref;

pub struct Vector(Vec<f64>);

impl Deref for Vector {
    type Target = Vec<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Vector {
    #[must_use]
    pub fn dot(&self, other: &Vector) -> f64 {
        self.0.iter().zip(&other.0).map(|(xi, yi)| xi * yi).sum()
    }
}
