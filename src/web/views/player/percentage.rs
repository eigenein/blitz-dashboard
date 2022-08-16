//! Percentage element for displaying in a «level» item.

use maud::{html, Markup, Render};

use crate::web::partials::*;
use crate::web::views::player::view_constants::*;

pub struct Percentage {
    ratio: f64,
    precision: usize,
}

impl From<f64> for Percentage {
    fn from(ratio: f64) -> Self {
        Self {
            ratio,
            precision: 1,
        }
    }
}

impl Percentage {
    pub const fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }
}

impl Render for Percentage {
    fn render(&self) -> Markup {
        html! {
            (Float::from(100.0 * self.ratio).precision(self.precision))
            (CARD_PERCENTAGE_SIGN)
        }
    }
}
