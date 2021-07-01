use maud::{html, DOCTYPE};
use tide::{Redirect, StatusCode};

use crate::web::partials::footer::Footer;
use crate::web::partials::{account_search, headers};
use crate::web::player::view::get_account_url;
use crate::web::responses::html;
use crate::web::search::models::ViewModel;
use crate::web::state::State;

pub async fn get(request: tide::Request<State>) -> tide::Result {
    let model = ViewModel::new(&request).await?;
    let footer = Footer::new(request.state()).await?;

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
                    (headers())
                    title { (model.query) " – Поиск статистов" }
                }
            }
            body {
                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.container {
                        div."navbar-brand" {
                            div.navbar-item {
                                div.buttons {
                                    a.button.is-link href="/" {
                                        span.icon { i.fas.fa-home {} }
                                        span { "На главную" }
                                    }
                                }
                            }
                            form.navbar-item action="/search" method="GET" {
                                (account_search("", &model.query, false))
                            }
                        }
                    }
                }

                section.section {
                    div.container {
                        div.columns {
                            div.column."is-6"."is-offset-3" {
                                @for account in &model.accounts {
                                    div.box {
                                        p.title."is-5" {
                                            a href=(get_account_url(account.basic.id)) { (account.nickname) }
                                        }
                                        p.subtitle.has-text-grey."is-6" { "#" (account.basic.id) }
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
