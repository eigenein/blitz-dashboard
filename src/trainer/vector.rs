use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Serialize, Deserialize, Clone)]
pub struct Vector(pub SmallVec<[f64; 32]>);

impl From<Vec<f64>> for Vector {
    fn from(vec: Vec<f64>) -> Self {
        Self(vec.into())
    }
}

impl Vector {
    #[must_use]
    pub const fn new() -> Self {
        Self(SmallVec::new_const())
    }

    #[must_use]
    pub fn norm(&self) -> f64 {
        self.0.iter().map(|xi| xi * xi).sum::<f64>().sqrt()
    }

    #[must_use]
    pub fn dot(&self, other: &Self) -> f64 {
        self.0.iter().zip(&other.0).map(|(xi, yi)| xi * yi).sum()
    }

    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f64 {
        self.dot(other) / self.norm() / other.norm()
    }
}

#[cfg(test)]
mod tests {
    use smallvec::smallvec;

    use super::*;

    #[test]
    fn cosine_similarity_ok() {
        let vector_1 = Vector(smallvec![1.0, 2.0, 3.0]);
        let vector_2 = Vector(smallvec![3.0, 5.0, 7.0]);
        assert!((vector_1.cosine_similarity(&vector_2) - 0.9974149030430578).abs() < f64::EPSILON);
    }
}
