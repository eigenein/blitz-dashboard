//! Player view.
//!
//! «Abandon hope, all ye who enter here».

use std::str::FromStr;
use std::time::Instant;

use bpci::Interval;
use chrono_humanize::Tense;
use clap::crate_version;
use maud::{html, Markup, PreEscaped, DOCTYPE};
use poem::http::StatusCode;
use poem::i18n::Locale;
use poem::web::cookie::CookieJar;
use poem::web::headers::{ETag, IfNoneMatch};
use poem::web::{Data, Form, Html, Path, RealIp, Redirect, TypedHeader};
use poem::{handler, IntoResponse, Response};

use self::damage_item::DamageItem;
use self::display_preferences::UpdateDisplayPreferences;
use self::interval_item::IntervalItem;
use self::partials::*;
use self::path::PathSegments;
use self::percentage_item::PercentageItem;
use self::view_model::ViewModel;
use crate::helpers::time::{from_days, from_hours, from_months};
use crate::math::traits::*;
use crate::prelude::*;
use crate::tankopedia::get_vehicle;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::web::partials::*;
use crate::web::views::player::display_preferences::DisplayPreferences;
use crate::web::{cookies, TrackingCode};
use crate::{database, wargaming};

mod damage_item;
mod display_preferences;
mod interval_item;
mod partials;
mod path;
mod percentage_item;
mod stats_delta;
mod view_constants;
mod view_model;

/// Updates display preferences.
#[allow(clippy::too_many_arguments)]
#[instrument(
    skip_all,
    level = "info",
    fields(realm = ?path.realm, account_id = path.account_id),
)]
#[handler]
pub async fn post(
    path: Path<PathSegments>,
    Form(update_preferences): Form<UpdateDisplayPreferences>,
    cookies: &CookieJar,
) -> poem::Result<Redirect> {
    let cookie_preferences = UpdateDisplayPreferences::from(cookies);
    cookies::Builder::new(UpdateDisplayPreferences::COOKIE_NAME)
        .value(&DisplayPreferences::from(cookie_preferences + update_preferences))
        .expires_in(Duration::weeks(4))
        .add_to(cookies);
    Ok(Redirect::see_other(format!("/{}/{}", path.realm, path.account_id)))
}

#[allow(clippy::too_many_arguments)]
#[instrument(
    skip_all,
    level = "info",
    fields(realm = ?path.realm, account_id = path.account_id),
)]
#[handler]
pub async fn get(
    path: Path<PathSegments>,
    cookies: &CookieJar,
    mongodb: Data<&mongodb::Database>,
    info_cache: Data<&AccountInfoCache>,
    tanks_cache: Data<&AccountTanksCache>,
    tracking_code: Data<&TrackingCode>,
    real_ip: RealIp,
    locale: Locale,
    none_match: Option<TypedHeader<IfNoneMatch>>,
) -> poem::Result<Response> {
    let start_instant = Instant::now();

    let view_model =
        ViewModel::new(real_ip.0, path, cookies, *mongodb, *info_cache, *tanks_cache).await?;

    let etag = format!(
        concat!("\"", crate_version!(), "-{}-{}-{}\""),
        view_model.actual_info.last_battle_time.timestamp(),
        view_model.period.as_secs(),
        locale.text("html-lang")?,
    );
    if let Some(TypedHeader(none_match)) = none_match {
        if !none_match.precondition_passes(&ETag::from_str(&etag).context("failed to parse ETag")?)
        {
            info!("not modified");
            return Ok(StatusCode::NOT_MODIFIED.into_response());
        }
    }

    let vehicles_thead = html! {
        tr {
            th {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-truck-monster {} }
                    span { (locale.text("title-vehicle")?) }
                }
            }

            th { (locale.text("title-type")?) }

            th.has-text-right {
                a data-sort="battles" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-battles")?) }
                    }
                }
            }

            th {
                a data-sort="wins" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-wins")?) }
                    }
                }
            }

            th.has-text-right {
                a data-sort="win-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-victory-ratio")?) }
                    }
                }
            }

            th {
                a data-sort="true-win-rate-mean" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span {
                            abbr title=(locale.text("title-victory-ratio-interval-abbr")?) {
                                (locale.text("title-victory-ratio-interval")?)
                            }
                        }
                    }
                }
            }

            th {
                a data-sort="frags-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-frags-per-battle")?) }
                    }
                }
            }

            th {
                a data-sort="true-gold" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span {
                            abbr title=(locale.text("title-gold-booster-interval-abbr")?) {
                                (locale.text("title-gold-booster-interval")?)
                            }
                        }
                    }
                }
            }

            th {
                a data-sort="damage-ratio" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-damage-ratio")?) }
                    }
                }
            }

            th.has-text-right {
                a data-sort="damage-dealt" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-damage-dealt")?) }
                    }
                }
            }

            th.has-text-right {
                a data-sort="damage-per-battle" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-damage-dealt-per-battle")?) }
                    }
                }
            }

            th.has-text-right {
                a data-sort="survived-battles" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-survived")?) }
                    }
                }
            }

            th {
                a data-sort="survival-rate" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { (locale.text("title-survival-ratio")?) }
                    }
                }
            }
        }
    };
    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-bottom lang=(locale.text("html-lang")?) {
            head {
                script type="module" defer { (r##"
                    'use strict';
                    
                    import { initSortableTable } from '/static/table.js?v5';
                    
                    (function () {
                        const vehicles = document.getElementById('vehicles');
                        if (vehicles != null) {
                            initSortableTable(vehicles, 'battles');
                        }
                    })();
                "##) }

                script type="module" defer { (r##"
                    'use strict';
                    import { init } from '/static/navbar.js?v1';
                    init();
                "##) }

                (headers())
                link rel="canonical" href=(format!("/{}/{}", view_model.realm, view_model.actual_info.id));
                title { (view_model.realm.to_emoji()) " " (view_model.actual_info.nickname) " – " (locale.text("page-title-index")?) }
            }
            body {
                (tracking_code.0)

                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.container {
                        div.navbar-brand {
                            (home_button(&locale)?)

                            div.navbar-item title="Последний бой" {
                                time.(if view_model.actual_info.has_recently_played() { "has-text-success-dark" } else if !view_model.actual_info.is_active() { "has-text-danger-dark" } else { "" })
                                    datetime=(view_model.actual_info.last_battle_time.to_rfc3339())
                                    title=(view_model.actual_info.last_battle_time) {
                                        (datetime(view_model.actual_info.last_battle_time, Tense::Past))
                                    }
                            }

                            div.navbar-item title=(locale.text("title-total-battles-hint")?) {
                                span.icon-text {
                                    span.icon { i.fas.fa-sort-numeric-up-alt {} }
                                    span { (view_model.actual_info.stats.n_total_battles()) }
                                }
                            }

                            div.navbar-item title=(locale.text("title-account-age-hint")?) {
                                span.icon-text {
                                    @if view_model.actual_info.is_account_birthday() {
                                        span.icon title=(locale.text("title-account-happy-birthday")?) { i.fas.fa-birthday-cake.has-text-danger {} }
                                    } @else {
                                        span.icon { i.far.fa-calendar-alt {} }
                                    }
                                    span title=(view_model.actual_info.created_at) {
                                        (datetime(view_model.actual_info.created_at, Tense::Present))
                                    }
                                }
                            }
                        }
                        div.navbar-menu.is-active {
                            div.navbar-end {
                                form.navbar-item action="/search" method="GET" {
                                    (
                                        AccountSearch::new(view_model.realm, &locale)
                                            .has_user_secret(view_model.actual_info.is_prerelease_account())
                                            .try_into_markup()?
                                    )
                                }
                            }
                        }
                    }
                }

                section.section.has-background-info-light."pt-5" {
                    p.subtitle.has-text-weight-medium { (view_model.realm.to_emoji()) (PreEscaped("&nbsp;")) (view_model.actual_info.nickname) }

                    div.container {
                        div.columns.is-multiline {
                            div class=(view_model.rating_snapshots.is_empty().then_some("column is-3-tablet is-3-desktop is-2-widescreen").unwrap_or("column is-5-tablet is-4-desktop is-3-widescreen")) {
                                div.card {
                                    header.card-header {
                                        p.card-header-title {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span.icon.has-text-warning { i.fa-solid.fa-star-half-stroke {} }
                                                span { (locale.text("title-rating")?) }
                                            }
                                        }
                                    }
                                    div.card-content {
                                        div.level.is-mobile {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { (locale.text("title-now")?) }
                                                    @let rating = view_model.actual_info.stats.rating.mm_rating.display_rating();
                                                    p.title title=(rating) { (rating) }
                                                }
                                            }
                                            @if !view_model.rating_snapshots.is_empty() {
                                                div.level-item.has-text-centered {
                                                    div id="rating-chart" {}
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
                                                span { (locale.text("title-victory-ratio")?) }
                                            }
                                        }
                                    }
                                    div.card-content {
                                        div.level.is-mobile {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { (locale.text("title-random-battles")?) }
                                                    p.title {
                                                        (PercentageItem::from(view_model.actual_info.stats.random.current_win_rate()).precision(2))
                                                    }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { (locale.text("title-rating-battles")?) }
                                                    p.title {
                                                        (PercentageItem::from(view_model.actual_info.stats.rating.basic.current_win_rate()).precision(2))
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
                                                span.icon.has-text-warning-dark { i.fa-solid.fa-solid.fa-house-damage {} }
                                                span { (locale.text("title-average-damage")?) }
                                            }
                                        }
                                    }
                                    div.card-content {
                                        div.level.is-mobile {
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { (locale.text("title-random-battles")?) }
                                                    p.title {
                                                        (DamageItem::new(
                                                            view_model.actual_info.stats.random.average_damage_dealt(),
                                                            view_model.actual_info.stats.random.damage_ratio(),
                                                        ))
                                                    }
                                                }
                                            }
                                            div.level-item.has-text-centered {
                                                div {
                                                    p.heading { (locale.text("title-rating-battles")?) }
                                                    p.title {
                                                        (DamageItem::new(
                                                            view_model.actual_info.stats.rating.basic.average_damage_dealt(),
                                                            view_model.actual_info.stats.rating.basic.damage_ratio(),
                                                        ))
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
                    nav.tabs.is-boxed.has-text-weight-medium {
                        div.container {
                            ul {
                                (render_period_li(view_model.period, from_hours(8), &locale.text("title-period-8-hours")?))
                                (render_period_li(view_model.period, from_hours(12), &locale.text("title-period-12-hours")?))
                                (render_period_li(view_model.period, from_days(1), &locale.text("title-period-24-hours")?))
                                (render_period_li(view_model.period, from_days(2), &locale.text("title-period-2-days")?))
                                (render_period_li(view_model.period, from_days(3), &locale.text("title-period-3-days")?))
                                (render_period_li(view_model.period, from_days(7), &locale.text("title-period-1-week")?))
                                (render_period_li(view_model.period, from_days(14), &locale.text("title-period-2-weeks")?))
                                (render_period_li(view_model.period, from_days(21), &locale.text("title-period-3-weeks")?))
                                (render_period_li(view_model.period, from_months(1), &locale.text("title-period-1-month")?))
                                (render_period_li(view_model.period, from_months(2), &locale.text("title-period-2-months")?))
                                (render_period_li(view_model.period, from_months(3), &locale.text("title-period-3-months")?))
                            }
                        }
                    }

                    div.container {
                        @if view_model.stats_delta.rating.n_battles != 0 {
                            div.columns.is-multiline.has-background-warning-light id="rating-columns" {
                                div.column."is-4-tablet"."is-4-desktop"."is-3-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-warning { i.fa-solid.fa-star-half-stroke {} }
                                                    span { (locale.text("title-rating")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-warning href="#rating-columns" { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-change")?) }
                                                        @let delta = view_model.stats_delta.rating.delta();
                                                        p.title.(SemaphoreClass::<_, f64>::new(delta)) title=(delta) {
                                                            (format!("{:+.0}", delta))
                                                        }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-per-battle")?) }
                                                        @let delta_per_battle = view_model.stats_delta.rating.delta_per_battle();
                                                        p.title.(SemaphoreClass::new(delta_per_battle)) title=(delta_per_battle) {
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
                                                    span { (locale.text("title-rating-battles")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-warning href="#rating-columns" { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-total")?) }
                                                        p.title { (view_model.stats_delta.rating.n_battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-wins")?) }
                                                        p.title { (view_model.stats_delta.rating.n_wins) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                div.column."is-3-tablet"."is-3-desktop"."is-2-widescreen" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-warning-dark { i.fa-solid.fa-house-damage {} }
                                                    span { (locale.text("title-damage")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-warning href="#rating-columns" { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-per-battle")?) }
                                                        p.title {
                                                            (DamageItem::new(
                                                                view_model.stats_delta.rating.average_damage_dealt(),
                                                                view_model.stats_delta.rating.damage_ratio(),
                                                            ))
                                                        }
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
                                                    span { (locale.text("title-victory-ratio")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-warning href="#rating-columns" { i.fa-solid.fa-trophy {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-average-masculine")?) }
                                                        p.title {
                                                            (PercentageItem::from(view_model.stats_delta.rating.current_win_rate()))
                                                        }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-interval")?) }
                                                        p.title.is-white-space-nowrap {
                                                            (IntervalItem(view_model.stats_delta.rating.true_win_rate()?))
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
                                    p { (PreEscaped(locale.text("message-not-played-rating")?)) }
                                }
                            }
                        }

                        @if view_model.stats_delta.random.n_battles != 0 {
                            div.columns.is-multiline id="random-columns" {
                                div.column."is-6-tablet"."is-4-desktop" {
                                    div.card {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-link { i.fa-solid.fa-sort-numeric-up-alt {} }
                                                    span { (locale.text("title-random-battles")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-grey-light href="#random-columns" { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-total")?) }
                                                        p.title { (view_model.stats_delta.random.n_battles) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-wins")?) }
                                                        p.title { (view_model.stats_delta.random.n_wins) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-survived")?) }
                                                        p.title { (view_model.stats_delta.random.n_survived_battles) }
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
                                                    span.icon.has-text-warning-dark { i.fa-solid.fa-house-damage {} }
                                                    span { (locale.text("title-damage-dealt")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-grey-light href="#random-columns" { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-total")?) }
                                                        p.title { (HumanFloat(view_model.stats_delta.random.damage_dealt as f64)) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-per-battle")?) }
                                                        p.title {
                                                            (Float::from(view_model.stats_delta.random.average_damage_dealt()))
                                                            @let damage_ratio = view_model.stats_delta.random.damage_ratio();
                                                            span.has-text-grey."is-size-4" { " (" }
                                                            span."is-size-4".(SemaphoreClass::new(damage_ratio).threshold(1.0)) {
                                                                (Float::from(damage_ratio).precision(1))
                                                            }
                                                            span.has-text-grey."is-size-4" { "×)" }
                                                        }
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
                                                    span.icon { i.fa-solid.fa-skull-crossbones {} }
                                                    span { (locale.text("title-destroyed")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-grey-light href="#random-columns" { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-total")?) }
                                                        p.title { (view_model.stats_delta.random.n_frags) }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-per-battle")?) }
                                                        p.title { (Float::from(view_model.stats_delta.random.frags_per_battle()).precision(1)) }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div.columns.is-multiline {
                                div.column."is-8-tablet"."is-6-desktop"."is-4-widescreen" {
                                    @let period_win_rate = view_model.stats_delta.random.true_win_rate()?;
                                    div.card.(partial_cmp_class(period_win_rate.partial_cmp(&view_model.actual_info.stats.random.current_win_rate()))) {
                                        header.card-header {
                                            p.card-header-title {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon.has-text-info { i.fa-solid.fa-percentage {} }
                                                    span { (locale.text("title-victory-ratio")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-grey-light href="#random-columns" { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-average-masculine")?) }
                                                        p.title {
                                                            (PercentageItem::from(view_model.stats_delta.random.current_win_rate()))
                                                        }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-interval")?) }
                                                        p.title.is-white-space-nowrap {
                                                            (IntervalItem(period_win_rate))
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
                                                    span { (locale.text("title-survival-ratio")?) }
                                                }
                                            }
                                            p.card-header-icon {
                                                a.icon.has-text-grey-light href="#random-columns" { i.fa-solid.fa-dice {} }
                                            }
                                        }
                                        div.card-content {
                                            div.level.is-mobile {
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-average-feminine")?) }
                                                        p.title {
                                                            (PercentageItem::from(view_model.stats_delta.random.survival_rate()))
                                                        }
                                                    }
                                                }
                                                div.level-item.has-text-centered {
                                                    div {
                                                        p.heading { (locale.text("title-interval")?) }
                                                        p.title.is-white-space-nowrap {
                                                            (IntervalItem(view_model.stats_delta.random.true_survival_rate()?))
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }

                                @if view_model.stats_delta.random.n_shots != 0 {
                                    div.column."is-4-tablet"."is-3-desktop"."is-2-widescreen" {
                                        div.card {
                                            header.card-header {
                                                p.card-header-title {
                                                    span.icon-text.is-flex-wrap-nowrap {
                                                        span.icon.has-text-warning-dark { i.fa-solid.fa-bullseye {} }
                                                        span { (locale.text("title-hits")?) }
                                                    }
                                                }
                                                p.card-header-icon {
                                                    a.icon.has-text-grey-light href="#random-columns" { i.fa-solid.fa-dice {} }
                                                }
                                            }
                                            div.card-content {
                                                div.level.is-mobile {
                                                    div.level-item.has-text-centered {
                                                        div {
                                                            p.heading { (locale.text("title-on-average")?) }
                                                            p.title {
                                                                (PercentageItem::from(view_model.stats_delta.random.hit_rate()))
                                                            }
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
                                    p { (PreEscaped(locale.text("message-not-played-random")?)) }
                                }
                            }
                        }

                        @if !view_model.stats_delta.tanks.is_empty() {
                            div.box {
                                div.table-container {
                                    table.table.is-hoverable.is-striped.is-fullwidth id="vehicles" {
                                        thead { (vehicles_thead) }
                                        tbody {
                                            @for tank in &view_model.stats_delta.tanks {
                                                (render_tank_tr(tank, view_model.actual_info.stats.random.current_win_rate(), &locale)?)
                                            }
                                        }
                                        @if view_model.stats_delta.tanks.len() >= 25 {
                                            tfoot { (vehicles_thead) }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                (footer(&locale)?)

                nav.navbar.is-fixed-bottom.has-shadow role="navigation" aria-label="dropdown navigation" {
                    div.navbar-brand {
                        a.navbar-burger role="button" data-target="bottomNavbar" aria-label="menu" aria-expanded="false" {
                            span aria-hidden="true" {}
                            span aria-hidden="true" {}
                            span aria-hidden="true" {}
                        }
                    }
                    div.navbar-menu id="bottomNavbar" {
                        div.navbar-item.has-dropdown.has-dropdown-up.is-hoverable {
                            @let current_win_rate = Float::from(100.0 * view_model.actual_info.stats.random.current_win_rate()).precision(2);
                            a.navbar-link {
                                span.icon.has-text-info { i.fa-solid.fa-percentage {} }
                                (current_win_rate)
                                span.has-text-grey { "%" }
                            }
                            div.navbar-dropdown {
                                div.navbar-item {
                                    (locale.text("navbar-item-target-victory-ratio")?)
                                }
                                hr.navbar-divider;
                                a.navbar-item {
                                    strong { (locale.text("navbar-item-current-masculine")?) }
                                    (PreEscaped("&nbsp;"))
                                    span.has-text-grey { " (" (current_win_rate) "%)" }
                                }
                            }
                        }

                        div.navbar-item.has-dropdown.has-dropdown-up.is-hoverable {
                            a.navbar-link {
                                span.icon.has-text-info { i.fa-solid.fa-p {} }
                                span { "90" }
                                span.has-text-grey { "%" }
                            }
                            div.navbar-dropdown {
                                div.navbar-item {
                                    (locale.text("navbar-item-confidence-level")?)
                                }
                                hr.navbar-divider;
                                a.navbar-item {
                                    span { "90" }
                                    span.has-text-grey { "%" }
                                }
                            }
                        }
                    }
                }

                @if !view_model.rating_snapshots.is_empty() {
                    script src="https://cdn.jsdelivr.net/npm/apexcharts" {}
                    script {
                        (PreEscaped("
                            'use strict';
                            const mode = (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) ? 'dark' : 'light';
                            new ApexCharts(document.getElementById('rating-chart'), {
                                chart: {
                                    type: 'line',
                                    width: 100,
                                    height: 57,
                                    sparkline: {enabled: true},
                                    animations: {enabled: false},
                                    background: 'transparent',
                                },
                                colors: ['hsl(204, 71%, 39%)'],
                                series: [{name: '', data: [
                        "))
                        @for snapshot in &view_model.rating_snapshots {
                            "[" (snapshot.date.timestamp_millis()) "," (snapshot.close_rating.display_rating()) "],"
                        }
                        (PreEscaped("]}],
                                xaxis: {type: 'datetime'},
                                tooltip: {
                                    fixed: {enabled: true, offsetY: 70},
                                    marker: {show: false},
                                    x: {format: 'MMM d'},
                                },
                                stroke: {width: 3, curve: 'straight'},
                                annotations: {yaxis: [
                                    {y: 5000, borderColor: 'hsl(217, 71%, 53%)'},
                                    {y: 4000, borderColor: 'hsl(141, 71%, 48%)'},
                                    {y: 3000, borderColor: 'hsl(48, 100%, 67%)'},
                                ]},
                                theme: {mode: mode},
                            }).render();
                        "))
                    }
                }
            }
        }
    };

    let response = Html(markup.into_string())
        .with_header("Cache-Control", "public, max-age=30, stale-while-revalidate=3600")
        .with_header("ETag", etag)
        .into_response();
    info!(elapsed = ?start_instant.elapsed(), "finished");
    Ok(response)
}

fn render_tank_tr(
    snapshot: &database::TankSnapshot,
    account_win_rate: f64,
    locale: &Locale,
) -> Result<Markup> {
    let markup = html! {
        @let vehicle = get_vehicle(snapshot.tank_id);
        @let true_win_rate = snapshot.stats.true_win_rate()?;
        @let win_rate_ordering = true_win_rate.partial_cmp(&account_win_rate);

        tr.(partial_cmp_class(win_rate_ordering)) {
            (vehicle_th(&vehicle, locale)?)

            td.has-text-centered.is-white-space-nowrap {
                @match vehicle.type_ {
                    wargaming::TankType::Light => (locale.text("tank-type-light")?),
                    wargaming::TankType::Medium => (locale.text("tank-type-medium")?),
                    wargaming::TankType::Heavy => (locale.text("tank-type-heavy")?),
                    wargaming::TankType::AT => (locale.text("tank-type-at")?),
                    wargaming::TankType::Unknown => "",
                }
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

            td.is-white-space-nowrap.is-flex
                data-sort="true-win-rate-mean"
                data-value=(true_win_rate.mean())
            {
                span.icon-text.is-flex-wrap-nowrap."is-flex-grow-1".is-justify-content-space-around {
                    (partial_cmp_icon(win_rate_ordering, locale)?)
                    strong { span { (render_percentage(true_win_rate.lower())) } }
                    span.icon.has-text-grey-light title=(true_win_rate.mean()) { i.fa-solid.fa-ellipsis {} }
                    strong { span { (render_percentage(true_win_rate.upper())) } }
                }
            }

            @let frags_per_battle = snapshot.stats.frags_per_battle();
            td.has-text-centered data-sort="frags-per-battle" data-value=(frags_per_battle) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon { i.fas.fa-skull-crossbones.has-text-grey-light {} }
                    span { (render_float(frags_per_battle, 1)) }
                }
            }

            @let expected_gold = true_win_rate * (vehicle.tier as f64) + 10.0;
            td.is-white-space-nowrap data-sort="true-gold" data-value=(expected_gold.mean()) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-warning-dark { i.fas.fa-coins {} }
                    span {
                        strong { (render_float(expected_gold.mean(), 1)) }
                        span.has-text-grey {
                            (format!(" ±{:.1}", expected_gold.margin()))
                        }
                    }
                }
            }

            @let damage_ratio = snapshot.stats.damage_ratio();
            td.has-text-centered data-sort="damage-ratio" data-value=(damage_ratio) {
                span.icon-text.is-flex-wrap-nowrap {
                    span.icon.has-text-grey { i.fa-solid.fa-divide {} }
                    strong.(SemaphoreClass::new(damage_ratio).threshold(1.0)) { (Float::from(damage_ratio).precision(2)) }
                }
            }

            td.has-text-right data-sort="damage-dealt" data-value=(snapshot.stats.damage_dealt) {
                (HumanFloat(snapshot.stats.damage_dealt as f64))
            }

            @let damage_per_battle = snapshot.stats.average_damage_dealt();
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
