use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

use crate::models::{Nation, Vehicle};
use crate::statistics::ConfidenceInterval;
use std::cmp::Ordering;

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
        _ if value < level_success => "",
        _ if value < level_warning => "has-text-warning-dark",
        _ => "has-text-danger",
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

pub static TIER_MARKUP: phf::Map<i32, &'static str> = phf::phf_map! {
    1_i32 => "Ⅰ",
    2_i32 => "Ⅱ",
    3_i32 => "Ⅲ",
    4_i32 => "Ⅳ",
    5_i32 => "Ⅴ",
    6_i32 => "Ⅵ",
    7_i32 => "Ⅶ",
    8_i32 => "Ⅷ",
    9_i32 => "Ⅸ",
    10_i32 => "Ⅹ",
};

pub fn render_vehicle_name(vehicle: &Vehicle) -> Markup {
    html! {
        span.(if vehicle.is_premium { "has-text-warning-dark" } else { "" }) {
            (vehicle.name)
        }
    }
}
