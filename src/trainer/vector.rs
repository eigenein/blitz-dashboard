use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Vector(pub Vec<f64>);

impl Vector {
    #[must_use]
    pub fn new() -> Self {
        Self(Vec::new())
    }

    #[must_use]
    pub fn norm(&self) -> f64 {
        self.0.iter().map(|xi| xi * xi).sum::<f64>().sqrt()
    }

    #[must_use]
    pub fn sub(self, rhs: &Self) -> Self {
        Self(
            self.0
                .iter()
                .zip(&rhs.0)
                .map(|(left, right)| left - right)
                .collect(),
        )
    }

    #[must_use]
    pub fn mul(&self, rhs: f64) -> Self {
        Self(self.0.iter().map(|xi| xi * rhs).collect())
    }

    pub fn add_assign(&mut self, rhs: Self) {
        for (left, right) in self.0.iter_mut().zip(rhs.0) {
            *left += right;
        }
    }

    #[must_use]
    pub fn dot(&self, other: &Self) -> f64 {
        self.0.iter().zip(&other.0).map(|(xi, yi)| xi * yi).sum()
    }

    #[must_use]
    pub fn cosine_similarity(&self, other: &Self) -> f64 {
        let (dot, x_len_squared, y_len_squared) = self.0.iter().zip(&other.0).fold(
            (0.0, 0.0, 0.0),
            |(dot, x_len_squared, y_len_squared), (xi, yi)| {
                (
                    dot + xi * yi,
                    x_len_squared + xi * xi,
                    y_len_squared + yi * yi,
                )
            },
        );
        dot / x_len_squared.sqrt() / y_len_squared.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_similarity_ok() {
        let vector_1 = Vector(vec![1.0, 2.0, 3.0]);
        let vector_2 = Vector(vec![3.0, 5.0, 7.0]);
        assert!((vector_1.cosine_similarity(&vector_2) - 0.9974149030430578).abs() < f64::EPSILON);
    }
}
