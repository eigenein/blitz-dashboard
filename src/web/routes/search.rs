use std::ops::Range;

use chrono_humanize::Tense;
use maud::{html, Markup, DOCTYPE};
use rocket::response::content::Html;
use rocket::response::status::BadRequest;
use rocket::response::Redirect;
use rocket::{uri, State};

use crate::logging::clear_user;
use crate::models::AccountInfo;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::WargamingApi;
use crate::web::partials::{account_search, datetime, footer, headers, home_button};
use crate::web::response::Response;
use crate::web::routes::player::rocket_uri_macro_get as rocket_uri_macro_get_player;
use crate::web::TrackingCode;

const SEARCH_QUERY_LENGTH: Range<usize> = MIN_QUERY_LENGTH..(MAX_QUERY_LENGTH + 1);
pub const MIN_QUERY_LENGTH: usize = 3;
pub const MAX_QUERY_LENGTH: usize = 24;

#[rocket::get("/search?<query>")]
pub async fn get(
    query: String,
    tracking_code: &State<TrackingCode>,
    api: &State<WargamingApi>,
    account_info_cache: &State<AccountInfoCache>,
) -> crate::web::result::Result<Response> {
    clear_user();

    if !SEARCH_QUERY_LENGTH.contains(&query.len()) {
        return Ok(Response::BadRequest(BadRequest(None)));
    }

    let account_ids: Vec<i32> = api
        .search_accounts(&query)
        .await?
        .iter()
        .map(|account| account.id)
        .collect();
    let mut accounts: Vec<AccountInfo> = api
        .get_account_info(&account_ids)
        .await?
        .into_iter()
        .filter_map(|(_, info)| info)
        .collect();
    if accounts.len() == 1 {
        let account_info = accounts.first().unwrap();
        account_info_cache.put(account_info).await?;
        return Ok(Response::Redirect(Redirect::temporary(uri!(get_player(
            account_id = account_info.base.id,
            period = _,
        )))));
    }
    let exact_match = accounts
        .iter()
        .position(|account| account.nickname == query)
        .map(|index| accounts.remove(index));
    if let Some(exact_match) = &exact_match {
        account_info_cache.put(exact_match).await?;
    }
    accounts.sort_unstable_by(|left, right| {
        right.base.last_battle_time.cmp(&left.base.last_battle_time)
    });

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                (headers())
                title { (query) " – Поиск статистов" }
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
                                (account_search("", &query, false, false))
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
                                (account_card(&exact_match))
                                h1.title.block."is-4" { "Другие результаты" }
                            }
                            @for account in &accounts {
                                (account_card(account))
                            }
                        }
                    }
                }
            }

            (footer())
        }
    };

    Ok(Response::Html(Html(markup.into_string())))
}

fn account_card(account_info: &AccountInfo) -> Markup {
    html! {
        div.box {
            p.title."is-5" {
                a href=(uri!(get_player(account_id = account_info.base.id, period = _))) { (account_info.nickname) }
            }
            p.subtitle."is-6" {
                span.icon-text.has-text-grey {
                    span.icon { i.far.fa-dot-circle {} }
                    span { (datetime(account_info.base.last_battle_time, Tense::Past)) }
                }
            }
        }
    }
}
