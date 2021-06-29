use maud::{html, Markup};

use crate::models::Vehicle;

#[inline(always)]
pub fn format_f64(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
    }
}

pub fn format_tier(tier: i8) -> Markup {
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

pub fn format_vehicle_name(vehicle: &Vehicle) -> Markup {
    html! {
        span.(if vehicle.is_premium { "has-text-warning-dark" } else { "" }) {
            (vehicle.name)
        }
    }
}
