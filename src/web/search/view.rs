use chrono_humanize::Tense;
use maud::{html, DOCTYPE};
use tide::{Redirect, StatusCode};

use crate::web::partials::{account_search, datetime, footer, headers};
use crate::web::player::view::get_account_url;
use crate::web::responses::html;
use crate::web::search::models::ViewModel;
use crate::web::state::State;

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = ViewModel::new(&request).await?;
    let state = request.state();
    let footer = footer(state).await?;

    if model.accounts.len() == 1 {
        return Ok(
            Redirect::temporary(get_account_url(model.accounts.first().unwrap().basic.id)).into(),
        );
    }

    Ok(html(
        StatusCode::Ok,
        html! {
            (DOCTYPE)
            html lang="en" {
                head {
                    (headers(&state.extra_html_headers))
                    title { (model.query) " – Поиск статистов" }
                }
            }
            body {
                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.container {
                        div.navbar-brand {
                            div.navbar-item {
                                div.buttons {
                                    a.button.is-link href="/" {
                                        span.icon { i.fas.fa-home {} }
                                        span { "На главную" }
                                    }
                                }
                            }
                        }
                        div.navbar-menu {
                            div.navbar-end {
                                form.navbar-item action="/search" method="GET" {
                                    (account_search("", &model.query, false))
                                }
                            }
                        }
                    }
                }

                @for account in &model.accounts {
                    section.section."p-0"."m-4" {
                        div.container {
                            div.columns.is-centered {
                                div.column."is-6-widescreen"."is-10-tablet" {
                                    div.card {
                                        div.card-content {
                                            p.title."is-5" {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span.icon { i.fas.fa-user {} }
                                                    span { a href=(get_account_url(account.basic.id)) { (account.nickname) } }
                                                }
                                            }
                                        }
                                        footer.card-footer {
                                            p.card-footer-item {
                                                span.icon-text.has-text-grey.is-flex-wrap-nowrap {
                                                    span.icon { i.fas.fa-sort-numeric-up-alt {} }
                                                    span { (account.statistics.all.battles) " боев" }
                                                }
                                            }
                                            p.card-footer-item {
                                                span.icon-text.has-text-grey.is-flex-wrap-nowrap {
                                                    span.icon { i.fas.fa-calendar-day {} }
                                                    span { (datetime(account.basic.last_battle_time, Tense::Past)) }
                                                }
                                            }
                                            p.card-footer-item {
                                                span.icon-text.has-text-grey.is-flex-wrap-nowrap {
                                                    span.icon { i.fas.fa-birthday-cake {} }
                                                    span { (datetime(account.created_at, Tense::Present)) }
                                                }
                                            }
                                            p.card-footer-item {
                                                span.icon-text.has-text-grey.is-flex-wrap-nowrap {
                                                    span.icon { i.far.fa-id-badge {} }
                                                    span { (account.basic.id) }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                (footer)
            }
        },
    ))
}
