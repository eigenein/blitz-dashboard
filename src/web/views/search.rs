pub mod models;

use chrono_humanize::Tense;
use maud::{html, Markup, DOCTYPE};
use poem::web::{Data, Html, Query, Redirect};
use poem::{handler, IntoResponse, Response};
use tracing::instrument;

use self::models::*;
use crate::helpers::sentry::clear_user;
use crate::wargaming;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::{AccountInfo, Realm, WargamingApi};
use crate::web::partials::{account_search, datetime, footer, headers, home_button};
use crate::web::TrackingCode;

#[instrument(skip_all, level = "info", fields(query = ?params.query.0))]
#[handler]
pub async fn get(
    params: Query<Params>,
    tracking_code: Data<&TrackingCode>,
    api: Data<&WargamingApi>,
    account_info_cache: Data<&AccountInfoCache>,
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
        .position(|account| account.nickname == params.query.0)
        .map(|index| accounts.remove(index));
    if let Some(exact_match) = &exact_match {
        account_info_cache.put(params.realm, exact_match).await?;
    }
    accounts.sort_unstable_by(|left, right| right.last_battle_time.cmp(&left.last_battle_time));

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="ru" {
            head {
                (headers())
                title { (params.query.0) " – Поиск статистов" }
            }
        }
        body {
            (tracking_code.0)
            nav.navbar.has-shadow.is-fixed-top role="navigation" aria-label="main navigation" {
                div.container {
                    div.navbar-brand {
                        (home_button())
                    }
                    div.navbar-menu {
                        div.navbar-end {
                            form.navbar-item action="/search" method="GET" {
                                (account_search("", params.realm, &params.query.0, false, false))
                            }
                        }
                    }
                }
            }

            section.section."p-0"."m-6" {
                div.container {
                    div.columns.is-centered {
                        div.column."is-6-widescreen"."is-10-tablet" {
                            @if accounts.is_empty() {
                                div.box {
                                    p.content {
                                        "Не найдено ни одного аккаунта с подобным именем."
                                    }
                                    p {
                                        a class="button is-info" href="https://ru.wargaming.net/registration/ru/" {
                                            "Создать аккаунт"
                                        }
                                    }
                                }
                            }
                            @if let Some(exact_match) = exact_match {
                                h1.title.block."is-4" { "Точное совпадение" }
                                (account_card(params.realm, &exact_match))
                                h1.title.block."is-4" { "Другие результаты" }
                            }
                            @for account in &accounts {
                                (account_card(params.realm, account))
                            }
                        }
                    }
                }
            }

            (footer())
        }
    };

    Ok(Html(markup.into_string()).into_response())
}

fn account_card(realm: Realm, account_info: &AccountInfo) -> Markup {
    html! {
        div.box {
            p.title."is-5" {
                a href=(format!("/{}/{}", realm, account_info.id)) { (account_info.nickname) }
            }
            p.subtitle."is-6" {
                span.icon-text.has-text-grey {
                    span.icon { i.far.fa-dot-circle {} }
                    span { (datetime(account_info.last_battle_time, Tense::Past)) }
                }
            }
        }
    }
}
