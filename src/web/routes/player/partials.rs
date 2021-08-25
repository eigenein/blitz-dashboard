use std::cmp::Ordering;
use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

use crate::models::{Nation, Tank, TankType, Vehicle};
use crate::statistics::ConfidenceInterval;
use crate::tankopedia::get_vehicle;

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

pub fn render_percentage(value: f64) -> Markup {
    html! {
        (render_f64(value * 100.0, 1)) "%"
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

pub fn render_tank_tr(tank: &Tank, total_win_rate: &ConfidenceInterval) -> Markup {
    html! {
        @let vehicle = get_vehicle(tank.statistics.base.tank_id);
        @let true_win_rate = tank.statistics.all.true_win_rate();
        @let win_rate_ordering = true_win_rate.partial_cmp(total_win_rate);

        tr.(partial_cmp_class(win_rate_ordering)) {
            (vehicle_th(&vehicle))
            td.has-text-centered data-sort="tier" data-value=(vehicle.tier) {
                @if let Some(tier_markup) = TIER_MARKUP.get(&vehicle.tier) {
                    strong { (tier_markup) }
                }
            }
            td {
                (format!("{:?}", vehicle.type_))
            }
            td data-sort="battles" data-value=(tank.statistics.all.battles) {
                (tank.statistics.all.battles)
            }
            td data-sort="wins" data-value=(tank.statistics.all.wins) {
                (tank.statistics.all.wins)
            }

            @let win_rate = tank.statistics.all.current_win_rate();
            td data-sort="win-rate" data-value=(win_rate) {
                strong { (render_percentage(win_rate)) }
            }

            td.is-white-space-nowrap
                data-sort="true-win-rate-mean"
                data-value=(true_win_rate.mean)
            {
                span.icon-text.is-flex-wrap-nowrap {
                    span {
                        strong { (render_percentage(true_win_rate.mean)) }
                        span.(margin_class(true_win_rate.margin, 0.1, 0.25)) {
                            " ±" (render_f64(100.0 * true_win_rate.margin, 1))
                        }
                        (partial_cmp_icon(win_rate_ordering))
                    }
                }
            }

            @let wins_per_hour = tank.wins_per_hour();
            td data-sort="wins-per-hour" data-value=(wins_per_hour) {
                (render_f64(wins_per_hour, 1))
            }

            @let expected_wins_per_hour = true_win_rate * tank.battles_per_hour();
            td.is-white-space-nowrap
                data-sort="expected-wins-per-hour"
                data-value=(expected_wins_per_hour.mean)
            {
                strong { (render_f64(expected_wins_per_hour.mean, 1)) }
                span.(margin_class(true_win_rate.margin, 0.1, 0.25)) {
                    " ±" (render_f64(expected_wins_per_hour.margin, 1))
                }
            }

            @let gold = 10 * tank.statistics.all.battles + vehicle.tier * tank.statistics.all.wins;
            td data-sort="gold" data-value=(gold) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                    span { (gold) }
                }
            }

            @let expected_gold = 10.0 + vehicle.tier as f64 * true_win_rate;
            td.is-white-space-nowrap data-sort="true-gold" data-value=(expected_gold.mean) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                    span {
                        strong { (render_f64(expected_gold.mean, 1)) }
                        span.(margin_class(expected_gold.margin, 2.0, 3.0)) {
                            " ±" (render_f64(expected_gold.margin, 1))
                        }
                    }
                }
            }

            td data-sort="damage-dealt" data-value=(tank.statistics.all.damage_dealt) {
                (tank.statistics.all.damage_dealt)
            }

            @let damage_per_battle = tank.statistics.all.damage_dealt as f64 / tank.statistics.all.battles as f64;
            td data-sort="damage-per-battle" data-value=(damage_per_battle) {
                (render_f64(damage_per_battle, 0))
            }

            td data-sort="survived-battles" data-value=(tank.statistics.all.survived_battles) {
                (tank.statistics.all.survived_battles)
            }

            @let survival_rate = tank.statistics.all.survived_battles as f64 / tank.statistics.all.battles as f64;
            td data-sort="survival-rate" data-value=(survival_rate) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-heart.has-text-danger {} }
                    span { (render_percentage(survival_rate)) }
                }
            }
        }
    }
}
