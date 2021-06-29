use maud::{html, Markup};

use crate::models::{Nation, Vehicle};

#[inline(always)]
pub fn render_f64(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
    }
}

pub fn render_tier(tier: i8) -> Markup {
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
