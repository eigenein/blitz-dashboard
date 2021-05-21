use crate::wargaming::WargamingApi;
use maud::html;
use serde::Deserialize;

mod logging;
mod utils;

#[derive(Clone)]
struct State {
    api: WargamingApi,
}

/// User search query.
#[derive(Deserialize)]
struct IndexQuery {
    #[serde(default = "String::default")]
    search: String,
}

/// Run the web app.
pub async fn run(host: &str, port: u16, application_id: String) -> anyhow::Result<()> {
    let mut app = tide::with_state(State {
        api: WargamingApi::new(application_id),
    });
    app.with(tide_compress::CompressMiddleware::new());
    app.with(logging::RequestLogMiddleware);
    app.at("/").get(get_index);
    log::info!("Listening on {}:{}.", host, port);
    app.listen((host, port)).await?;
    Ok(())
}

/// Home page that allows searching for a user.
async fn get_index(request: tide::Request<State>) -> tide::Result {
    let query: IndexQuery = request.query()?;
    let state = request.state();

    let accounts = if query.search.len() >= 3 {
        Some(state.api.search_accounts(&query.search).await?)
    } else {
        None
    };

    utils::respond_with_body(html! {
        section class="hero is-fullheight" {
            div class="hero-body" {
                div class="container" {
                    div class="columns" {
                        div class="column is-8 is-offset-2" {
                            form action="/" method="GET" {
                                div class="field has-addons" {
                                    div class="control" {
                                        span class="select is-medium is-rounded" {
                                            select disabled {
                                                option { "🇷🇺 RU" }
                                                option { "🇪🇺 EU" }
                                                option { "🇺🇸 NA" }
                                                option { "🇨🇳 AS" }
                                            }
                                        }
                                    }
                                    div class="control has-icons-left is-expanded" {
                                        input class="input is-medium is-rounded" type="text" value=(query.search) name="search" placeholder="Username or user ID" autocomplete="nickname" minlength="3" maxlength="24" autofocus required;
                                        span class="icon is-medium is-left" {
                                            i class="fas fa-user" {}
                                        }
                                    }
                                    div class="control" {
                                        input class="button is-medium is-rounded is-info" type="submit" value="Search";
                                    }
                                }
                            }
                            @if let Some(accounts) = accounts {
                                div class="tags mt-4" {
                                    @for account in accounts {
                                        span class="tag is-success is-rounded" title=(account.id) {
                                            (account.nickname)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    })
}
