//! Interval component for displaying inside a «level» item.

use bpci::Interval;
use maud::{html, Markup, Render};

use crate::web::partials::Float;
use crate::web::views::player::percentage_item::PercentageItem;

pub struct IntervalItem<T>(pub T);

impl<T: Interval<f64>> Render for IntervalItem<T> {
    fn render(&self) -> Markup {
        html! {
            (PercentageItem::from(self.0.mean()))
            span."is-size-4" {
                span.has-text-grey-light { " ±" }
                (Float::from(100.0 * self.0.margin()).precision(1))
            }
        }
    }
}
