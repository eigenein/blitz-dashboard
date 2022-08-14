pub mod models;

use chrono_humanize::Tense;
use maud::{html, Markup, DOCTYPE};
use poem::i18n::Locale;
use poem::web::{Data, Html, Query, Redirect};
use poem::{handler, IntoResponse, Response};
use tracing::instrument;

use self::models::*;
use crate::helpers::sentry::clear_user;
use crate::math::traits::*;
use crate::prelude::*;
use crate::wargaming;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::{AccountInfo, Realm, WargamingApi};
use crate::web::partials::*;
use crate::web::TrackingCode;

const COLUMN_CLASS: &str = "is-12-tablet is-8-desktop is-6-widescreen";

#[instrument(skip_all, level = "info", fields(query = ?params.query.0))]
#[handler]
pub async fn get(
    params: Query<QueryParams>,
    tracking_code: Data<&TrackingCode>,
    api: Data<&WargamingApi>,
    account_info_cache: Data<&AccountInfoCache>,
    locale: Locale,
) -> poem::Result<Response> {
    clear_user();

    let account_ids: Vec<wargaming::AccountId> = api
        .search_accounts(params.realm, &params.query.0)
        .await?
        .iter()
        .map(|account| account.id)
        .collect();
    let mut accounts: Vec<AccountInfo> = api
        .get_account_info(params.realm, &account_ids)
        .await?
        .into_iter()
        .filter_map(|(_, info)| info)
        .collect();
    if accounts.len() == 1 {
        let account_info = accounts.first().unwrap();
        account_info_cache.put(params.realm, account_info).await?;
        return Ok(
            Redirect::temporary(format!("/{}/{}", params.realm, account_info.id)).into_response()
        );
    }
    let exact_match = accounts
        .iter()
        .position(|account| account.nickname.to_lowercase() == params.query.0)
        .map(|index| accounts.remove(index));
    if let Some(exact_match) = &exact_match {
        account_info_cache.put(params.realm, exact_match).await?;
    }
    accounts.sort_unstable_by(|left, right| right.last_battle_time.cmp(&left.last_battle_time));

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang=(locale.text("html-lang")?) {
            head {
                (headers())
                title { (params.query.0) " – " (locale.text("page-title-search")?) }
            }
        }
        body {
            (tracking_code.0)
            nav.navbar.has-shadow.is-fixed-top role="navigation" aria-label="main navigation" {
                div.navbar-item.is-expanded.columns.is-centered {
                    div.column.is-flex.is-flex-direction-row.(COLUMN_CLASS) {
                        a."px-4"."is-flex"."is-align-self-center" href="/" {
                            img src="/android-chrome-192x192.png" width="28" height="28" alt=(locale.text("alt-home")?);
                        }
                        form."is-flex-grow-1" action="/search" method="GET" {
                            (
                                AccountSearch::new(params.realm, &locale)
                                    .value(&params.query.0)
                                    .try_into_markup()?
                            )
                        }
                    }
                }
            }

            section.section {
                div.columns.is-centered {
                    div.column.(COLUMN_CLASS) {
                        @if accounts.is_empty() {
                            div.box {
                                p.content {
                                    (locale.text("message-no-players-found")?)
                                }
                                p {
                                    a class="button is-info" href="https://ru.wargaming.net/registration/ru/" {
                                        (locale.text("button-create-account")?)
                                    }
                                }
                            }
                        } @else {
                            div.menu {
                                @if let Some(exact_match) = &exact_match {
                                    p.menu-label { (locale.text("title-exact-match")?) }
                                    ul.menu-list {
                                        (account_item(params.realm, exact_match, &locale)?)
                                    }
                                }

                                @if exact_match.is_some() {
                                    p.menu-label { (locale.text("title-other-results")?) }
                                }
                                ul.menu-list {
                                    @for account in &accounts {
                                        (account_item(params.realm, account, &locale)?)
                                    }
                                }
                            }
                        }
                    }
                }
            }

            (footer(&locale)?)
        }
    };

    Ok(Html(markup.into_string()).into_response())
}

fn account_item(realm: Realm, account_info: &AccountInfo, locale: &Locale) -> Result<Markup> {
    let markup = html! {
        li {
            a href=(format!("/{}/{}", realm, account_info.id)) {
                p."is-size-5"."py-1" {
                    span.has-text-link { (account_info.nickname) }
                }
                p."is-size-6"."py-1".has-text-grey {
                    (locale.text("title-last-played")?)
                    " "
                    strong { (datetime(account_info.last_battle_time, Tense::Past)) }
                }
                p."is-size-6"."py-1" {
                    span.icon-text."mr-4" title=(locale.text("title-battles")?) {
                        span.icon.has-text-link { i.fa-solid.fa-star-half-stroke {} }
                        strong { (account_info.stats.n_total_battles()) }
                    }
                    span.icon-text."mr-4" {
                        span.icon.has-text-info { i.fa-solid.fa-percentage {} }
                        (Float::from(account_info.stats.random.current_win_rate() * 100.0).precision(2))
                    }
                    span.icon-text."mr-4" {
                        span.icon.has-text-warning-dark { i.fa-solid.fa-house-damage {} }
                        (Float::from(account_info.stats.random.average_damage_dealt()))
                    }
                    span.icon-text."mr-4" {
                        span.icon.has-text-warning { i.fa-solid.fa-star-half-stroke {} }
                        (account_info.stats.rating.mm_rating.display_rating())
                    }
                }
            }
        }
    };
    Ok(markup)
}
