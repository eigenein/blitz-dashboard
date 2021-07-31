use std::cmp::Ordering;
use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

use crate::models::{Nation, TankType, Vehicle};

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

pub fn vehicle_th(vehicle: &Vehicle) -> Markup {
    let flag = match vehicle.nation {
        Nation::China => "🇨🇳",
        Nation::Europe => "🇪🇺",
        Nation::France => "🇫🇷",
        Nation::Germany => "🇩🇪",
        Nation::Japan => "🇯🇵",
        Nation::Other => "🏳",
        Nation::Uk => "🇬🇧",
        Nation::Usa => "🇺🇸",
        Nation::Ussr => "🇷🇺",
    };
    let name_class = if vehicle.is_premium {
        "has-text-warning-dark"
    } else if vehicle.type_ == TankType::Unknown {
        "has-text-grey"
    } else {
        ""
    };
    html! {
        th.is-white-space-nowrap {
            span."mx-1" { (flag) }
            strong."mx-1".(name_class) { (vehicle.name) }
        }
    }
}
