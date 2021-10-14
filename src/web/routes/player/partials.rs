use std::cmp::Ordering;
use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

use crate::models::Tank;
use crate::statistics::ConfidenceInterval;
use crate::tankopedia::get_vehicle;
use crate::trainer::math::predict_win_rate;
use crate::trainer::vector::Vector;
use crate::web::partials::{margin_class, render_f64, tier_td, vehicle_th};

pub fn render_period_li(
    period: Option<StdDuration>,
    new_period: Option<StdDuration>,
    text: &'static str,
) -> Markup {
    html! {
        li.(if period == new_period { "is-active" } else { "" }) {
            @match new_period {
                Some(new_period) => { a href=(format!("?period={}", format_duration(new_period))) { (text) } },
                None => { a href="?" { (text) } },
            }
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

pub fn render_tank_tr(
    tank: &Tank,
    total_win_rate: &ConfidenceInterval,
    account_factors: &Option<Vector>,
    vehicle_factors: Option<&Vector>,
) -> Markup {
    html! {
        @let vehicle = get_vehicle(tank.statistics.base.tank_id);
        @let true_win_rate = tank.statistics.all.true_win_rate();
        @let win_rate_ordering = true_win_rate.partial_cmp(total_win_rate);

        tr.(partial_cmp_class(win_rate_ordering)) {
            (vehicle_th(&vehicle))
            (tier_td(vehicle.tier))
            td {
                (format!("{:?}", vehicle.type_))
            }
            td data-sort="battles" data-value=(tank.statistics.all.battles) {
                (tank.statistics.all.battles)
            }
            td data-sort="wins" data-value=(tank.statistics.all.wins) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (tank.statistics.all.wins) }
                }
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
                        strong { span { (render_percentage(true_win_rate.mean)) } }
                        span.(margin_class(true_win_rate.margin, 0.1, 0.25)) {
                            " ±" (render_f64(100.0 * true_win_rate.margin, 1))
                        }
                    }
                    (partial_cmp_icon(win_rate_ordering))
                }
            }

            @let predicted_win_rate = if let (Some(account_factors), Some(vehicle_factors)) = (account_factors, vehicle_factors) {
                Some(predict_win_rate(vehicle_factors, account_factors).clamp(0.0, 1.0))
            } else {
                None
            };
            td data-sort="predicted-win-rate" data-value=(predicted_win_rate.unwrap_or_default()) {
                @if let Some(predicted_win_rate) = predicted_win_rate {
                    sup title="В разработке" { strong.has-text-danger-dark { "ɑ" } }
                    strong title=(predicted_win_rate) { (format!("{:.0}%", predicted_win_rate * 100.0)) }
                } else {
                    span.icon-text.is-flex-wrap-nowrap.has-text-grey-light {
                        span.icon { i.fas.fa-hourglass-half {} }
                        span { "Обучение" }
                    }
                }
            }

            @let frags_per_battle = tank.statistics.all.frags_per_battle();
            td data-sort="frags-per-battle" data-value=(frags_per_battle) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-skull-crossbones.has-text-grey-light {} }
                    span { (render_f64(frags_per_battle, 1)) }
                }
            }

            @let wins_per_hour = tank.wins_per_hour();
            td data-sort="wins-per-hour" data-value=(wins_per_hour) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (render_f64(wins_per_hour, 1)) }
                }
            }

            @let expected_wins_per_hour = true_win_rate * tank.battles_per_hour();
            td.is-white-space-nowrap
                data-sort="expected-wins-per-hour"
                data-value=(expected_wins_per_hour.mean)
            {
                strong { (render_f64(expected_wins_per_hour.mean, 1)) }
                span.(margin_class(true_win_rate.margin, 0.1, 0.25)) {
                    (format!(" ±{:.1}", expected_wins_per_hour.margin))
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
                            (format!(" ±{:.1}", expected_gold.margin))
                        }
                    }
                }
            }

            td data-sort="damage-dealt" data-value=(tank.statistics.all.damage_dealt) {
                (tank.statistics.all.damage_dealt)
            }

            @let damage_per_battle = tank.statistics.all.damage_dealt as f64 / tank.statistics.all.battles as f64;
            td data-sort="damage-per-battle" data-value=(damage_per_battle) {
                (format!("{:.0}", damage_per_battle))
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
