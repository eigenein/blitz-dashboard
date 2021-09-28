use std::ops::Deref;

pub struct Vector(Vec<f64>);

impl Deref for Vector {
    type Target = Vec<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
