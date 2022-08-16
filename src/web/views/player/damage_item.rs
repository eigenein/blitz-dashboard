use maud::{html, Markup, Render};

use crate::web::partials::{Float, HumanFloat, SemaphoreClass};

pub struct DamageItem {
    average: f64,
    ratio: f64,
}

impl DamageItem {
    pub const fn new(average: f64, ratio: f64) -> Self {
        Self { average, ratio }
    }
}

impl Render for DamageItem {
    fn render(&self) -> Markup {
        html! {
            (HumanFloat(self.average))
            span."is-size-4".has-text-grey { " (" }
            span."is-size-4".(SemaphoreClass::new(self.ratio).threshold(1.0)) {
                (Float::from(self.ratio).precision(1))
            }
            span."is-size-4".has-text-grey { "×)" }
        }
    }
}
