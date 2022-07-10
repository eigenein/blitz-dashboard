//! Player view.
//!
//! «Abandon hope, all ye who enter here».

use std::collections::hash_map::Entry;
use std::time::Instant;

use chrono::{Duration, Utc};
use chrono_humanize::Tense;
use either::Either;
use futures::future::try_join;
use humantime::format_duration;
use itertools::Itertools;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use mongodb::bson;
use poem::error::{InternalServerError, NotFoundError};
use poem::web::{Data, Html, Path, Query};
use poem::{handler, IntoResponse, Response};

use self::models::*;
use crate::helpers::sentry::set_user;
use crate::helpers::time::{from_days, from_months};
use crate::math::statistics::{ConfidenceInterval, ConfidenceLevel};
use crate::math::traits::{AverageDamageDealt, CurrentWinRate, TrueWinRate};
use crate::prelude::*;
use crate::tankopedia::get_vehicle;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::models::subtract_tanks;
use crate::web::partials::*;
use crate::web::views::player::partials::*;
use crate::web::TrackingCode;
use crate::{database, format_elapsed, wargaming};

mod models;
mod partials;

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all, level = "info", fields(account_id = account_id, period = ?params.period.0))]
#[handler]
pub async fn get(
    Path(Segments { realm, account_id }): Path<Segments>,
    params: Query<Params>,
    mongodb: Data<&mongodb::Database>,
    info_cache: Data<&AccountInfoCache>,
    tracking_code: Data<&TrackingCode>,
    tanks_cache: Data<&AccountTanksCache>,
) -> poem::Result<Response> {
    let start_instant = Instant::now();
    let (actual_info, actual_tanks) =
        try_join(info_cache.get(realm, account_id), tanks_cache.get(realm, account_id)).await?;
    let actual_info = actual_info.ok_or(NotFoundError)?;
    set_user(&actual_info.nickname);
    database::Account::new(realm, account_id)
        .upsert(&mongodb, database::Account::OPERATION_SET_ON_INSERT)
        .await?;

    let before = Utc::now() - Duration::from_std(params.period.0).map_err(InternalServerError)?;
    let current_win_rate = ConfidenceInterval::wilson_score_interval(
        actual_info.statistics.all.n_battles,
        actual_info.statistics.all.n_wins,
        ConfidenceLevel::default(),
    );
    let stats_delta = match retrieve_deltas_quickly(
        &mongodb,
        realm,
        account_id,
        actual_info.statistics.all,
        actual_info.statistics.rating,
        actual_tanks,
        before,
    )
    .await?
    {
        Either::Left(delta) => delta,
        Either::Right(tanks) => {
            retrieve_deltas_slowly(
                &mongodb,
                realm,
                account_id,
                tanks,
                before,
                actual_info.statistics.rating,
            )
            .await?
        }
    };
    let battle_life_time: i64 = stats_delta
        .tanks
        .iter()
        .map(|snapshot| snapshot.battle_life_time.num_seconds())
        .sum();

    let navbar = html! {
        nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
            div.container {
                div.navbar-brand {
                    (home_button())

                    div.navbar-item title="Последний бой" {
                        time.(if actual_info.has_recently_played() { "has-text-success-dark" } else if !actual_info.is_active() { "has-text-danger-dark" } else { "" })
                            datetime=(actual_info.last_battle_time.to_rfc3339())
                            title=(actual_info.last_battle_time) {
                                (datetime(actual_info.last_battle_time, Tense::Past))
                            }
                    }

                    div.navbar-item title="Боев" {
                        span.icon-text {
                            span.icon { i.fas.fa-sort-numeric-up-alt {} }
                            span { (actual_info.statistics.n_total_battles()) }
                        }
                    }

                    div.navbar-item title="Возраст аккаунта" {
                        span.icon-text {
                            @if actual_info.is_account_birthday() {
                                span.icon title="День рождения!" { i.fas.fa-birthday-cake.has-text-danger {} }
                            } @else {
                                span.icon { i.far.fa-calendar-alt {} }
                            }
                            span title=(actual_info.created_at) {
                                (datetime(actual_info.created_at, Tense::Present))
                            }
                        }
                    }
                }
                div.navbar-menu.is-active {
                    div.navbar-end {
                        form.navbar-item action="/search" method="GET" {
                            (account_search("", realm, &actual_info.nickname, false, actual_info.is_prerelease_account()))
                        }
                    }
                }
            }
        }
    };
    let tabs = html! {
        nav.tabs.is-boxed.has-text-weight-medium {
            div.container {
                ul {
                    (render_period_li(params.period.0, from_days(1), "24 часа"))
                    (render_period_li(params.period.0, from_days(2), "2 дня"))
                    (render_period_li(params.period.0, from_days(3), "3 дня"))
                    (render_period_li(params.period.0, from_days(7), "Неделя"))
                    (render_period_li(params.period.0, from_days(14), "2 недели"))
                    (render_period_li(params.period.0, from_days(21), "3 недели"))
                    (render_period_li(params.period.0, from_months(1), "Месяц"))
                    (render_period_li(params.period.0, from_months(2), "2 месяца"))
                    (render_period_li(params.period.0, from_months(3), "3 месяца"))
                }
            }
        }
    };
    let vehicles_thead = html! {
        tr {
            th {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-truck-monster {} }
                    span { "Техника" }
                }
            }

            th { "Тип" }

            th.has-text-right {
                a data-sort="battle-life-time" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Время в боях" }
                    }
                }
            }

            th.has-text-right {
                a data-sort="battles" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Бои" }
                    }
                }
            }

            th {
                a data-sort="wins" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Победы" }
                    }
                }
            }

            th.has-text-right {
                a data-sort="win-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Процент побед" }
                    }
                }
            }

            th {
                a data-sort="true-win-rate-mean" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Процент побед, скорректированный на число боев, CI 98%" { "Процент побед (интервал)" } }
                    }
                }
            }

            th {
                a data-sort="frags-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Фраги за бой" }
                    }
                }
            }

            th {
                a data-sort="wins-per-hour" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Победы в час" }
                    }
                }
            }

            th {
                a data-sort="expected-wins-per-hour" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Число побед в час, скорректированное на число проведенных боев, CI 98%" { "Победы в час (интервал)" } }
                    }
                }
            }

            th {
                a data-sort="true-gold" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { abbr title="Доходность золотого бустера за бой, скорректированная на число проведенных боев, CI 98%" { "Ожидаемое золото" } }
                    }
                }
            }

            th.has-text-right {
                a data-sort="damage-dealt" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Общий урон" }
                    }
                }
            }

            th.has-text-right {
                a data-sort="damage-per-minute" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Урон в минуту" }
                    }
                }
            }

            th.has-text-right.is-white-space-nowrap {
                a data-sort="damage-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Урон за бой" }
                    }
                }
            }

            th.has-text-right {
                a data-sort="survived-battles" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Выжил" }
                    }
                }
            }

            th {
                a data-sort="survival-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Выживаемость" }
                    }
                }
            }
        }
    };
    let markup = html! {
        (DOCTYPE)
        html lang="en" {
            head {
                script type="module" defer {
                    (PreEscaped(r#"""
                        "use strict";
                        
                        import { initSortableTable } from "/static/table.js?v5";
                        
                        (function () {
                            const vehicles = document.getElementById("vehicles");
                            if (vehicles != null) {
                                initSortableTable(vehicles, "battles");
                            }
                        })();
                    """#))
                }

                (headers())
                link rel="canonical" href=(format!("/{}/{}", realm, account_id));
                title { (actual_info.nickname) " – Я – статист в World of Tanks Blitz!" }
            }
            body {
                (tracking_code.0)

                (navbar)

                section.section.has-background-info-light."pt-5" {
                    p.subtitle.has-text-weight-medium { "За все время" }

                    div.container {
                        div.columns.is-multiline {
                            div.column."is-3-tablet"."is-3-desktop"."is-2-widescreen" {
                                div.card {
                                    header.card-header {
                                        p.card-header-title {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span.icon.has-text-warning { i.fa-solid.fa-star-half-stroke {} }
                                                span { "Рейтинг" }
                                            }
                                        }
                                    }
                                    div.card-content {
                                        div.level.is-mobile {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Сейчас" }
                                                    @let rating = actual_info.statistics.rating.rating();
                                                    p.title title=(rating) { (format!("{:.0}", rating)) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div.column."is-6-tablet"."is-5-desktop"."is-4-widescreen" {
                                div.card {
                                    header.card-header {
                                        p.card-header-title {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span.icon.has-text-info { i.fa-solid.fa-percentage {} }
                                                span { "Процент побед" }
                                            }
                                        }
                                    }
                                    div.card-content {
                                        div.level.is-mobile {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Случайные бои" }
                                                    @let win_rate = 100.0 * actual_info.statistics.all.current_win_rate();
                                                    p.title title=(win_rate) {
                                                        (format!("{:.2}", win_rate))
                                                        span.has-text-grey-light { "%" }
                                                    }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Рейтинговые бои" }
                                                    @let win_rate = 100.0 * actual_info.statistics.rating.basic.current_win_rate();
                                                    p.title title=(win_rate) {
                                                        (format!("{:.2}", win_rate))
                                                        span.has-text-grey-light { "%" }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div.column."is-6-tablet"."is-5-desktop"."is-4-widescreen" {
                                div.card {
                                    header.card-header {
                                        p.card-header-title {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span.icon { i.fa-solid.fa-solid.fa-house-damage {} }
                                                span { "Средний урон" }
                                            }
                                        }
                                    }
                                    div.card-content {
                                        div.level.is-mobile {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Случайные бои" }
                                                    @let damage_dealt = actual_info.statistics.all.average_damage_dealt();
                                                    p.title title=(damage_dealt) {
                                                        (format!("{:.0}", damage_dealt))
                                                    }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { "Рейтинговые бои" }
                                                    @let damage_dealt = actual_info.statistics.rating.basic.average_damage_dealt();
                                                    p.title title=(damage_dealt) {
                                                        (format!("{:.0}", damage_dealt))
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                section.section."pt-5" {
                    (tabs)

                    div.container {
                        @if stats_delta.rating.n_battles != 0 {
                            div.columns.is-multiline.has-background-warning-light {
                                div.column."is-4-tablet"."is-4-desktop"."is-3-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-warning { i.fa-solid.fa-star-half-stroke {} }
                                                    span { "Рейтинг" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-warning { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Изменение" }
                                                        @let delta = stats_delta.rating.delta();
                                                        p.title.(sign_class(delta)) title=(delta) {
                                                            (format!("{:+.0}", delta))
                                                        }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        @let delta_per_battle = stats_delta.rating.delta_per_battle();
                                                        p.title.(sign_class(delta_per_battle)) title=(delta_per_battle) {
                                                            (format!("{:+.0}", delta_per_battle))
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-5-tablet"."is-4-desktop"."is-3-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-link { i.fa-solid.fa-sort-numeric-up-alt {} }
                                                    span { "Рейтинговые бои" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-warning { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.rating.n_battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Победы" }
                                                        p.title { (stats_delta.rating.n_wins) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-7-tablet"."is-6-desktop"."is-4-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-info { i.fa-solid.fa-percentage {} }
                                                    span { "Процент побед" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-warning { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Средний" }
                                                        p.title { (render_percentage(stats_delta.rating.current_win_rate())) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Истинный" }
                                                        p.title.is-white-space-nowrap {
                                                            @let true_win_rate = stats_delta.rating.true_win_rate();
                                                            (render_percentage(true_win_rate.mean))
                                                            span.has-text-grey-light { " ±" (render_float(100.0 * true_win_rate.margin, 1)) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } @else {
                            article.message.is-warning {
                                div.message-body {
                                    p { "Пользователь не играл в " strong { "рейтинговых" } " боях за этот период времени." }
                                }
                            }
                        }

                        @if stats_delta.random.n_battles != 0 {
                            div.columns.is-multiline {
                                div.column."is-6-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-link { i.fa-solid.fa-sort-numeric-up-alt {} }
                                                    span { "Случайные бои" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.random.n_battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Победы" }
                                                        p.title { (stats_delta.random.n_wins) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Выжил" }
                                                        p.title { (stats_delta.random.n_survived_battles) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-6-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon { i.fa-solid.fa-house-damage {} }
                                                    span { "Нанесенный урон" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.random.damage_dealt) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_float(stats_delta.random.damage_per_battle(), 0)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-12-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon { i.fa-solid.fa-skull-crossbones {} }
                                                    span { "Уничтоженная техника" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Всего" }
                                                        p.title { (stats_delta.random.n_frags) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "За бой" }
                                                        p.title { (render_float(stats_delta.random.frags_per_battle(), 1)) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "В час" }
                                                        p.title { (render_float(stats_delta.random.n_frags as f64 / battle_life_time as f64 * 3600.0, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div.columns.is-multiline {
                                div.column."is-8-tablet"."is-6-desktop"."is-4-widescreen" {
                                    @let period_win_rate = stats_delta.random.true_win_rate();
                                    div.card.(partial_cmp_class(period_win_rate.partial_cmp(&current_win_rate))) {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-info { i.fa-solid.fa-percentage {} }
                                                    span { "Процент побед" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Средний" }
                                                        p.title { (render_percentage(stats_delta.random.current_win_rate())) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Истинный" }
                                                        p.title.is-white-space-nowrap {
                                                            (render_percentage(period_win_rate.mean))
                                                            span.has-text-grey-light { " ±" (render_float(100.0 * period_win_rate.margin, 1)) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-8-tablet"."is-6-desktop"."is-4-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-danger { i.fa-solid.fa-heart {} }
                                                    span { "Выживаемость" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Средняя" }
                                                        p.title { (render_percentage(stats_delta.random.survival_rate())) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "Истинная" }
                                                        p.title.is-white-space-nowrap {
                                                            @let expected_period_survival_rate = ConfidenceInterval::wilson_score_interval(
                                                                stats_delta.random.n_battles, stats_delta.random.n_survived_battles, ConfidenceLevel::default());
                                                            (render_percentage(expected_period_survival_rate.mean))
                                                            span.has-text-grey-light { (format!(" ±{:.1}", 100.0 * expected_period_survival_rate.margin)) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-4-tablet"."is-3-desktop"."is-2-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-success { i.fa-solid.fa-check {} }
                                                    span { "Победы" }
                                                }
                                            }
                                            p.card-header-icon {
                                                span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { "В час" }
                                                        p.title { (render_float(stats_delta.random.n_wins as f64 / battle_life_time as f64 * 3600.0, 1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                @if stats_delta.random.n_shots != 0 {
                                    div.column."is-4-tablet"."is-3-desktop"."is-2-widescreen" {
                                        div.card {
                                            header.card-header {
                                                p.card-header-title {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span.icon.has-text-warning-dark { i.fa-solid.fa-bullseye {} }
                                                        span { "Попадания" }
                                                    }
                                                }
                                                p.card-header-icon {
                                                    span.icon.has-text-grey-light { i.fa-solid.fa-dice {} }
                                                }
                                            }
                                            div.card-content {
                                                div.level.is-mobile {
                                                    div.level-item.has-text-centered {
                                                        div {
                                                            p.heading { "В среднем" }
                                                            p.title { (render_percentage(stats_delta.random.hit_rate())) }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        } @else {
                            article.message {
                                div.message-body {
                                    p { "Пользователь не играл в " strong { "случайных" } " боях за этот период времени." }
                                }
                            }
                        }

                        @if !stats_delta.tanks.is_empty() {
                            div.box {
                                div.table-container {
                                    table.table.is-hoverable.is-striped.is-fullwidth id="vehicles" {
                                        thead { (vehicles_thead) }
                                        tbody {
                                            @for tank in &stats_delta.tanks {
                                                (render_tank_tr(tank, &current_win_rate)?)
                                            }
                                        }
                                        @if stats_delta.tanks.len() >= 25 {
                                            tfoot { (vehicles_thead) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                (footer())
            }
        }
    };

    let response = Html(markup.into_string())
        .with_header("Cache-Control", "max-age=60, stale-while-revalidate=3600")
        .into_response();
    info!(elapsed = format_elapsed(start_instant).as_str(), "finished");
    Ok(response)
}

fn render_tank_tr(
    snapshot: &database::TankSnapshot,
    account_win_rate: &ConfidenceInterval,
) -> Result<Markup> {
    let markup = html! {
        @let vehicle = get_vehicle(snapshot.tank_id);
        @let true_win_rate = snapshot.stats.true_win_rate();
        @let win_rate_ordering = true_win_rate.partial_cmp(account_win_rate);

        tr.(partial_cmp_class(win_rate_ordering)) {
            (vehicle_th(&vehicle))

            td.has-text-centered {
                @match vehicle.type_ {
                    wargaming::TankType::Light => "ЛТ",
                    wargaming::TankType::Medium => "СТ",
                    wargaming::TankType::Heavy => "ТТ",
                    wargaming::TankType::AT => "ПТ",
                    wargaming::TankType::Unknown => "",
                }
            }

            td.has-text-right.is-white-space-nowrap data-sort="battle-life-time" data-value=(snapshot.battle_life_time.num_seconds()) {
                (format_duration(snapshot.battle_life_time.to_std()?))
            }

            td.has-text-right data-sort="battles" data-value=(snapshot.stats.n_battles) {
                (snapshot.stats.n_battles)
            }

            td data-sort="wins" data-value=(snapshot.stats.n_wins) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (snapshot.stats.n_wins) }
                }
            }

            @let win_rate = snapshot.stats.current_win_rate();
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

            @let frags_per_battle = snapshot.stats.frags_per_battle();
            td data-sort="frags-per-battle" data-value=(frags_per_battle) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-skull-crossbones.has-text-grey-light {} }
                    span { (render_float(frags_per_battle, 1)) }
                }
            }

            @let wins_per_hour = snapshot.wins_per_hour();
            td data-sort="wins-per-hour" data-value=(wins_per_hour) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-success { i.fas.fa-check {} }
                    span { (render_float(wins_per_hour, 1)) }
                }
            }

            @let expected_wins_per_hour = true_win_rate * snapshot.battles_per_hour();
            td.is-white-space-nowrap
                data-sort="expected-wins-per-hour"
                data-value=(expected_wins_per_hour.mean)
            {
                strong { (render_float(expected_wins_per_hour.mean, 1)) }
                span.has-text-grey {
                    (format!(" ±{:.1}", expected_wins_per_hour.margin))
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

            td.has-text-right data-sort="damage-dealt" data-value=(snapshot.stats.damage_dealt) {
                (snapshot.stats.damage_dealt)
            }

            @let damage_per_minute = snapshot.damage_per_minute();
            td.has-text-right data-sort="damage-per-minute" data-value=(damage_per_minute) {
                (format!("{:.0}", damage_per_minute))
            }

            @let damage_per_battle = snapshot.stats.damage_dealt as f64 / snapshot.stats.n_battles as f64;
            td.has-text-right data-sort="damage-per-battle" data-value=(damage_per_battle) {
                (format!("{:.0}", damage_per_battle))
            }

            td.has-text-right data-sort="survived-battles" data-value=(snapshot.stats.n_survived_battles) {
                (snapshot.stats.n_survived_battles)
            }

            @let survival_rate = snapshot.stats.n_survived_battles as f64 / snapshot.stats.n_battles as f64;
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

struct StatsDelta {
    random: database::RandomStatsSnapshot,
    rating: database::RatingStatsSnapshot,
    tanks: Vec<database::TankSnapshot>,
}

#[instrument(skip_all, level = "debug", fields(account_id = account_id, before = ?before))]
async fn retrieve_deltas_quickly(
    from: &mongodb::Database,
    realm: wargaming::Realm,
    account_id: wargaming::AccountId,
    random_stats: wargaming::BasicStatistics,
    rating_stats: wargaming::RatingStatistics,
    mut actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
    before: DateTime,
) -> Result<Either<StatsDelta, AHashMap<wargaming::TankId, wargaming::Tank>>> {
    match database::AccountSnapshot::retrieve_latest(from, realm, account_id, before).await? {
        Some(account_snapshot) => {
            let tank_last_battle_times = account_snapshot.tank_last_battle_times.iter().filter(
                |(tank_id, last_battle_time)| {
                    let tank_entry = actual_tanks.entry(*tank_id);
                    match tank_entry {
                        Entry::Occupied(entry) => {
                            let keep =
                                bson::DateTime::from(entry.get().statistics.last_battle_time)
                                    > *last_battle_time;
                            if !keep {
                                entry.remove();
                            }
                            keep
                        }
                        Entry::Vacant(_) => false,
                    }
                },
            );
            let snapshots = database::TankSnapshot::retrieve_many(
                from,
                realm,
                account_id,
                tank_last_battle_times,
            )
            .await?;
            Ok(Either::Left(StatsDelta {
                random: random_stats - account_snapshot.random_stats,
                rating: rating_stats - account_snapshot.rating_stats,
                tanks: subtract_tanks(realm, actual_tanks, snapshots),
            }))
        }
        None => Ok(Either::Right(actual_tanks)),
    }
}

#[instrument(skip_all, level = "debug", fields(account_id = account_id))]
async fn retrieve_deltas_slowly(
    from: &mongodb::Database,
    realm: wargaming::Realm,
    account_id: wargaming::AccountId,
    actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank>,
    before: DateTime,
    rating_stats: wargaming::RatingStatistics,
) -> Result<StatsDelta> {
    debug!("taking the slow path");
    let actual_tanks: AHashMap<wargaming::TankId, wargaming::Tank> = actual_tanks
        .into_iter()
        .filter(|(_, tank)| tank.statistics.last_battle_time >= before)
        .collect();
    let snapshots = {
        let tank_ids = actual_tanks
            .values()
            .map(wargaming::Tank::tank_id)
            .collect_vec();
        database::TankSnapshot::retrieve_latest_tank_snapshots(
            from, realm, account_id, before, &tank_ids,
        )
        .await?
    };
    let tanks_delta = subtract_tanks(realm, actual_tanks, snapshots);
    Ok(StatsDelta {
        random: tanks_delta.iter().map(|tank| tank.stats).sum(),
        rating: rating_stats.into(),
        tanks: tanks_delta,
    })
}
