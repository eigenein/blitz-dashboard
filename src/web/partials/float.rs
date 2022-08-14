use std::fmt::Display;

use maud::{html, Markup, Render};

pub struct Float<'a, T> {
    value: T,
    precision: usize,
    class: Option<&'a str>,
}

impl<T> From<T> for Float<'_, T> {
    fn from(value: T) -> Self {
        Self {
            value,
            precision: 0,
            class: None,
        }
    }
}

impl<'a, T> Float<'a, T> {
    pub const fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    #[allow(dead_code)]
    pub const fn class(mut self, class: &'a str) -> Self {
        self.class = Some(class);
        self
    }
}

impl<T: Display + num_traits::Float> Render for Float<'_, T> {
    fn render(&self) -> Markup {
        html! {
            @if self.value.is_finite() {
                span.(self.class.unwrap_or("")) title=(self.value) {
                    (format!("{0:.1$}", self.value, self.precision))
                }
            } @else {
                span { "-" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finite_ok() {
        assert_eq!(
            Float::from(0.5)
                .class("has-text-grey")
                .render()
                .into_string(),
            r#"<span class="has-text-grey" title="0.5">1</span>"#
        );
    }

    #[test]
    fn infinite_ok() {
        assert_eq!(Float::from(f64::INFINITY).render().into_string(), "<span>-</span>");
    }
}
