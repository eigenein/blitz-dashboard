use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};
use ordered_float::OrderedFloat;

use crate::models::{AllStatistics, Nation, Tank, Vehicle};
use crate::statistics::wilson_score_interval;
use crate::tankopedia::get_vehicle;

// TODO: remove it.
pub struct DisplayRow {
    pub vehicle: Vehicle,
    pub all_statistics: AllStatistics,
    pub win_rate: OrderedFloat<f64>,
    pub expected_win_rate: OrderedFloat<f64>,
    pub expected_win_rate_margin: OrderedFloat<f64>,
    pub damage_per_battle: OrderedFloat<f64>,
    pub survival_rate: OrderedFloat<f64>,
    pub gold_per_battle: OrderedFloat<f64>,
    pub expected_gold_per_battle: OrderedFloat<f64>,
}

pub fn render_period_li(
    sort: &str,
    period: StdDuration,
    new_period: StdDuration,
    text: &'static str,
) -> Markup {
    html! {
        li.(if period == new_period { "is-active" } else { "" }) {
            a href=(format!("?sort={}&period={}#period", sort, format_duration(new_period))) { (text) }
        }
    }
}

pub fn render_vehicles_th(
    sort: &str,
    period: StdDuration,
    new_sort: &str,
    markup: Markup,
) -> Markup {
    html! {
        th {
            a href=(format!("?sort={}&period={}#vehicles", new_sort, format_duration(period))) {
                span.icon-text.is-flex-wrap-nowrap {
                    @if sort == new_sort { span.icon { i.fas.fa-angle-down {} } }
                    span { (markup) }
                }
            }
        }
    }
}

pub fn render_confidence_interval_level(n_trials: i32, n_successes: i32) -> Markup {
    let mean = 100.0 * n_successes as f64 / n_trials as f64;
    let (p, margin) = wilson_score_interval(n_trials, n_successes);
    let lower = 100.0 * (p - margin);
    let upper = 100.0 * (p + margin);

    html! {
        div.level {
            div.level-item.has-text-centered {
                div {
                    p.heading { "Нижнее" }
                    p.title."is-5" { (render_f64(lower, 1)) "%" }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Среднее" }
                    p.title { (render_f64(mean, 1)) "%" }
                }
            }
            div.level-item.has-text-centered {
                div {
                    p.heading { "Верхнее" }
                    p.title."is-5" { (render_f64(upper, 1)) "%" }
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

// FIXME: inline it.
pub fn make_display_row(tank: Tank) -> DisplayRow {
    let vehicle = get_vehicle(tank.tank_id);
    let stats = &tank.all_statistics;
    let win_rate = stats.wins as f64 / stats.battles as f64;
    let expected_win_rate = wilson_score_interval(stats.battles, stats.wins);
    DisplayRow {
        win_rate: OrderedFloat(win_rate),
        expected_win_rate: OrderedFloat(expected_win_rate.0),
        expected_win_rate_margin: OrderedFloat(expected_win_rate.1),
        damage_per_battle: OrderedFloat(stats.damage_dealt as f64 / stats.battles as f64),
        survival_rate: OrderedFloat(stats.survived_battles as f64 / stats.battles as f64),
        all_statistics: tank.all_statistics,
        gold_per_battle: OrderedFloat(10.0 + vehicle.tier as f64 * win_rate),
        expected_gold_per_battle: OrderedFloat(10.0 + vehicle.tier as f64 * expected_win_rate.0),
        vehicle,
    }
}
