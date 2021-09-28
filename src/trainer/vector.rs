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
