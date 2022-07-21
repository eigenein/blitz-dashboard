use std::fmt::{Display, Write};

use maud::{Escaper, Render};

pub struct Float<T> {
    value: T,
    precision: usize,
}

impl<T> From<T> for Float<T> {
    fn from(value: T) -> Self {
        Self {
            value,
            precision: 0,
        }
    }
}

impl<T> Float<T> {
    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }
}

impl<T: Display> Render for Float<T> {
    fn render_to(&self, buffer: &mut String) {
        write!(buffer, r#"<span title=""#).unwrap();
        write!(Escaper::new(buffer), "{}", self.value).unwrap();
        write!(buffer, r#"">"#).unwrap();
        write!(Escaper::new(buffer), "{0:.1$}", self.value, self.precision).unwrap();
        write!(buffer, "</span>").unwrap();
    }
}
