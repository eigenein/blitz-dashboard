use std::cmp::Ordering;
use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

pub fn render_period_li(
    period: StdDuration,
    new_period: StdDuration,
    text: &'static str,
) -> Markup {
    html! {
        li.(if period == new_period { "is-active" } else { "" }) {
            a href=(format!("?period={}", format_duration(new_period))) { (text) }
        }
    }
}

pub fn partial_cmp_class(ordering: Option<Ordering>) -> &'static str {
    match ordering {
        Some(Ordering::Less) => "has-background-danger-light",
        Some(Ordering::Greater) => "has-background-success-light",
        _ => "",
    }
}

pub fn partial_cmp_icon(ordering: Option<Ordering>) -> Markup {
    match ordering {
        Some(Ordering::Less) => html! {
            span.icon.has-text-danger title="Игра на этом танке уменьшает общий процент побед на аккаунте" {
                i.fas.fa-thumbs-down {}
            }
        },
        Some(Ordering::Greater) => html! {
            span.icon.has-text-success title="Игра на этом танке увеличивает общий процент побед на аккаунте" {
                i.fas.fa-thumbs-up {}
            }
        },
        _ => html! {},
    }
}

pub fn render_percentage(value: f64) -> String {
    format!("{:.1}%", value * 100.0)
}
