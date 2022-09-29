use std::cmp::Ordering;

use maud::{html, Markup};

use crate::prelude::*;
use crate::web::partials::Float;

pub const fn partial_cmp_class(ordering: Option<Ordering>) -> &'static str {
    match ordering {
        Some(Ordering::Less) => "has-background-danger-light",
        Some(Ordering::Greater) => "has-background-success-light",
        _ => "",
    }
}

pub fn render_percentage(value: f64) -> Markup {
    html! {
        (Float::from(value * 100.0).precision(1))
        span.has-text-grey-light { "%" }
    }
}
