use std::cmp::Ordering;
use std::time::Duration as StdDuration;

use humantime::format_duration;
use maud::{html, Markup};

use crate::math::statistics::ConfidenceInterval;
use crate::models::{Tank, TankType};
use crate::tankopedia::get_vehicle;
use crate::web::partials::{render_float, vehicle_th};
use crate::web::views::bulma::*;
use crate::DateTime;

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

pub fn render_tank_tr(
    tank: &Tank,
    account_win_rate: &ConfidenceInterval,
    predicted_win_rate: Option<f64>,
    live_win_rate: Option<ConfidenceInterval>,
    last_account_battle_time: DateTime,
) -> crate::Result<Markup> {
    let markup = html! {
        @let vehicle = get_vehicle(tank.statistics.base.tank_id);
        @let true_win_rate = tank.statistics.all.true_win_rate();
        @let win_rate_ordering = true_win_rate.partial_cmp(account_win_rate);

        tr.(partial_cmp_class(win_rate_ordering)) {
            (vehicle_th(&vehicle))

            td.has-text-centered {
                @match vehicle.type_ {
                    TankType::Light => "ЛТ",
                    TankType::Medium => "СТ",
                    TankType::Heavy => "ТТ",
                    TankType::AT => "ПТ",
                    TankType::Unknown => "?",
                }
            }

            td.has-text-right.is-white-space-nowrap data-sort="battle-life-time" data-value=(tank.statistics.battle_life_time.num_seconds()) {
                (format_duration(tank.statistics.battle_life_time.to_std()?))
            }

            td.has-text-right data-sort="battles" data-value=(tank.statistics.all.battles) {
                (tank.statistics.all.battles)
            }

            td data-sort="wins" data-value=(tank.statistics.all.wins) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (tank.statistics.all.wins) }
                }
            }

            @let win_rate = tank.statistics.all.current_win_rate();
            td.has-text-right data-sort="win-rate" data-value=(win_rate) {
                strong { (render_percentage(win_rate)) }
            }

            td.is-white-space-nowrap
                data-sort="true-win-rate-mean"
                data-value=(true_win_rate.mean)
            {
                span.icon-text.is-flex-wrap-nowrap {
                    span {
                        strong { span { (render_percentage(true_win_rate.mean)) } }
                        span.has-text-grey {
                            " ±" (render_float(100.0 * true_win_rate.margin, 1))
                        }
                    }
                    (partial_cmp_icon(win_rate_ordering))
                }
            }

            @if let Some(predicted_win_rate) = predicted_win_rate {
                td data-sort="predicted-win-rate" data-value=(predicted_win_rate) {
                    span.icon-text.is-flex-wrap-nowrap {
                        @if tank.statistics.base.last_battle_time <= last_account_battle_time {
                            span.icon.has-text-link { i.fas.fa-dice-d20 {} }
                        } @else {
                            span.icon.has-text-grey-light title="Робот еще не просканировал последние бои на этом танке" {
                                i.fas.fa-hourglass-half {}
                            }
                        }
                        strong title=(predicted_win_rate) {
                            (format!("{:.0}%", predicted_win_rate * 100.0))
                        }
                    }
                }
            } @else {
                td.has-text-centered data-sort="predicted-win-rate" data-value="-1" {
                    span.icon.has-text-grey-light { i.fas.fa-hourglass-start {} }
                }
            }

            @if let Some(live_win_rate) = live_win_rate {
                td.is-white-space-nowrap data-sort="live-win-rate" data-value=(live_win_rate.mean) {
                    span.icon-text.is-flex-wrap-nowrap {
                        (Icon::ChartArea.into_span().color(Color::GreyLight))
                        span {
                            strong title=(live_win_rate.mean) {
                                (format!("{:.1}%", live_win_rate.mean * 100.0))
                            }
                            span.has-text-grey { (format!(" ±{:.1}", live_win_rate.margin * 100.0)) }
                        }
                    }
                }
            } @else {
                td.has-text-centered data-sort="live-win-rate" data-value="-1" {
                    span.icon.has-text-grey-light { i.fas.fa-hourglass-start {} }
                }
            }

            @let frags_per_battle = tank.statistics.all.frags_per_battle();
            td data-sort="frags-per-battle" data-value=(frags_per_battle) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-skull-crossbones.has-text-grey-light {} }
                    span { (render_float(frags_per_battle, 1)) }
                }
            }

            @let wins_per_hour = tank.wins_per_hour();
            td data-sort="wins-per-hour" data-value=(wins_per_hour) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (render_float(wins_per_hour, 1)) }
                }
            }

            @let expected_wins_per_hour = true_win_rate * tank.battles_per_hour();
            td.is-white-space-nowrap
                data-sort="expected-wins-per-hour"
                data-value=(expected_wins_per_hour.mean)
            {
                strong { (render_float(expected_wins_per_hour.mean, 1)) }
                span.has-text-grey {
                    (format!(" ±{:.1}", expected_wins_per_hour.margin))
                }
            }

            @if let Some(predicted_win_rate) = predicted_win_rate {
                @let predicted_wins_per_hour = predicted_win_rate * tank.battles_per_hour();
                td data-sort="predicted-wins-per-hour" data-value=(predicted_wins_per_hour) {
                    span.icon-text.is-flex-wrap-nowrap {
                        @if tank.statistics.base.last_battle_time <= last_account_battle_time {
                            span.icon.has-text-link { i.fas.fa-dice-d20 {} }
                        } @else {
                            span.icon.has-text-grey-light title="Робот еще не просканировал последние бои на этом танке" {
                                i.fas.fa-hourglass-half {}
                            }
                        }
                        strong title=(predicted_wins_per_hour) {
                            (format!("{:.0}", predicted_wins_per_hour))
                        }
                    }
                }
            } @else {
                td.has-text-centered data-sort="predicted-wins-per-hour" data-value="-1" {
                    span.icon.has-text-grey-light { i.fas.fa-hourglass-start {} }
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
                        strong { (render_float(expected_gold.mean, 1)) }
                        span.has-text-grey {
                            (format!(" ±{:.1}", expected_gold.margin))
                        }
                    }
                }
            }

            td.has-text-right data-sort="damage-dealt" data-value=(tank.statistics.all.damage_dealt) {
                (tank.statistics.all.damage_dealt)
            }

            @let damage_per_battle = tank.statistics.all.damage_dealt as f64 / tank.statistics.all.battles as f64;
            td.has-text-right data-sort="damage-per-battle" data-value=(damage_per_battle) {
                (format!("{:.0}", damage_per_battle))
            }

            td.has-text-right data-sort="survived-battles" data-value=(tank.statistics.all.survived_battles) {
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
    };
    Ok(markup)
}
