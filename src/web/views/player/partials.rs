use std::cmp::Ordering;

use maud::{html, Markup};
use poem::i18n::Locale;

use crate::prelude::*;
use crate::web::partials::Float;

pub const fn partial_cmp_class(ordering: Option<Ordering>) -> &'static str {
    match ordering {
        Some(Ordering::Less) => "has-background-danger-light",
        Some(Ordering::Greater) => "has-background-success-light",
        _ => "",
    }
}

pub fn partial_cmp_icon(ordering: Option<Ordering>, locale: &Locale) -> Result<Markup> {
    let markup = match ordering {
        Some(Ordering::Less) => html! {
            span.icon.has-text-danger title=(locale.text("hint-significantly-lower-than-target")?) {
                i.fas.fa-thumbs-down {}
            }
        },
        Some(Ordering::Greater) => html! {
            span.icon.has-text-success title=(locale.text("hint-significantly-higher-than-target")?) {
                i.fas.fa-thumbs-up {}
            }
        },
        _ => html! {},
    };
    Ok(markup)
}

pub fn render_percentage(value: f64) -> Markup {
    html! {
        (Float::from(value * 100.0).precision(1))
        span.has-text-grey-light { "%" }
    }
}
