use std::ops::{AddAssign, Deref, DerefMut, Mul, Sub};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Vector(Vec<f64>);

impl Deref for Vector {
    type Target = Vec<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Vector {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Mul<f64> for Vector {
    type Output = Self;

    #[must_use]
    fn mul(self, rhs: f64) -> Self::Output {
        Self(self.0.into_iter().map(|xi| xi * rhs).collect())
    }
}

impl Mul<Vector> for f64 {
    type Output = Vector;

    fn mul(self, rhs: Vector) -> Self::Output {
        rhs * self
    }
}

impl Sub<Self> for Vector {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(
            self.0
                .into_iter()
                .zip(rhs.0)
                .map(|(left, right)| left - right)
                .collect(),
        )
    }
}

impl AddAssign<Self> for Vector {
    fn add_assign(&mut self, rhs: Self) {
        for (left, right) in self.0.iter_mut().zip(rhs.0) {
            *left += right;
        }
    }
}

impl Vector {
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
