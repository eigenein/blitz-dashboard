use std::fmt::Display;

use maud::{html, Markup, PreEscaped, Render};

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
    pub const fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }
}

impl<T: Display + num_traits::Float> Render for Float<T> {
    fn render(&self) -> Markup {
        html! {
            @if self.value.is_finite() {
                span title=(maud::display(self.value)) {
                    (format!("{0:.1$}", self.value, self.precision))
                }
            } @else if self.value.is_infinite() && self.value.is_sign_positive() {
                (PreEscaped("<span>∞</span>"))
            } @else if self.value.is_infinite() && self.value.is_sign_negative() {
                (PreEscaped("<span>-∞</span>"))
            } @else {
                (PreEscaped("<span>-</span>"))
            }
        }
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
        assert_eq!(Float::from(f64::INFINITY).render().into_string(), "<span>∞</span>");
    }

    #[test]
    fn nan_ok() {
        assert_eq!(Float::from(f64::NAN).render().into_string(), "<span>-</span>");
    }
}
