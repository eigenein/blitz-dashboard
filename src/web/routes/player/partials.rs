use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

use crate::models::{Nation, Vehicle};
use crate::statistics::ConfidenceInterval;

pub fn render_period_li(
    period: StdDuration,
    new_period: StdDuration,
    text: &'static str,
) -> Markup {
    html! {
        li.(if period == new_period { "is-active" } else { "" }) {
            a href=(format!("?period={}#period", format_duration(new_period))) { (text) }
        }
    }
}

pub fn render_confidence_interval_level(n_trials: i32, n_successes: i32) -> Markup {
    let interval = ConfidenceInterval::default_wilson_score_interval(n_trials, n_successes);

    html! {
        div.level {
            div.level-item.has-text-centered {
                div {
                    p.heading { "Нижнее" }
                    p.title."is-5" { (render_f64(100.0 * interval.lower(), 1)) "%" }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Среднее" }
                    p.title { (render_f64(100.0 * n_successes as f64 / n_trials as f64, 1)) "%" }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Верхнее" }
                    p.title."is-5" { (render_f64(100.0 * interval.upper(), 1)) "%" }
                }
            }
        }
    }
}

pub fn margin_class(value: f64, level_success: f64, level_warning: f64) -> &'static str {
    match value {
        _ if value < level_success => "has-text-success",
        _ if value < level_warning => "has-text-warning-dark",
        _ => "has-text-danger",
    }
}

// TODO: `phf`.
pub fn render_nation(nation: &Nation) -> Markup {
    html! {
        @match nation {
            Nation::China => "🇨🇳",
            Nation::Europe => "🇪🇺",
            Nation::France => "🇫🇷",
            Nation::Germany => "🇩🇪",
            Nation::Japan => "🇯🇵",
            Nation::Other => "🏳",
            Nation::Uk => "🇬🇧",
            Nation::Usa => "🇺🇸",
            Nation::Ussr => "🇷🇺",
        }
    }
}

pub fn render_f64(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
    }
}

// TODO: `phf`.
pub fn render_tier(tier: i32) -> Markup {
    html! {
        @match tier {
            1 => "Ⅰ",
            2 => "Ⅱ",
            3 => "Ⅲ",
            4 => "Ⅳ",
            5 => "Ⅴ",
            6 => "Ⅵ",
            7 => "Ⅶ",
            8 => "Ⅷ",
            9 => "Ⅸ",
            10 => "Ⅹ",
            _ => "",
        }
    }
}

pub fn render_vehicle_name(vehicle: &Vehicle) -> Markup {
    html! {
        span.(if vehicle.is_premium { "has-text-warning-dark" } else { "" }) {
            (vehicle.name)
        }
    }
}
