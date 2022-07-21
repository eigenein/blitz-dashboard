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

impl<T: Display + num_traits::Float> Render for Float<T> {
    fn render_to(&self, buffer: &mut String) {
        write!(buffer, r"<span").unwrap();
        if self.value.is_finite() {
            write!(buffer, r#" title=""#).unwrap();
            write!(Escaper::new(buffer), "{}", self.value).unwrap();
            write!(buffer, r#"""#).unwrap();
        }
        write!(buffer, ">").unwrap();
        if self.value.is_finite() {
            write!(Escaper::new(buffer), "{0:.1$}", self.value, self.precision).unwrap();
        } else {
            buffer.push('-');
        }
        write!(buffer, "</span>").unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_ok() {
        assert_eq!(Float::from(0.5).render().into_string(), r#"<span title="0.5">1</span>"#);
    }

    #[test]
    fn infinite_ok() {
        assert_eq!(Float::from(f64::INFINITY).render().into_string(), "<span>-</span>");
    }
}
