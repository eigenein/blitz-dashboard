use crate::api::wargaming::models::AccountId;
use crate::web::components::account_search;
use crate::web::views::player::get_account_url;
use clap::crate_version;
use maud::{html, Markup, DOCTYPE};

pub fn head(title: Option<&str>) -> Markup {
    html! {
        title { @if let Some(title) = title { (title) " – " } "Blitz Dashboard" }
        meta name="viewport" content="width=device-width, initial-scale=1";
        meta charset="UTF-8";
        link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.2/css/bulma.min.css" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href="https://unpkg.com/bulma-prefers-dark";
    }
}

pub fn document(title: Option<&str>, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head { (head(title)) }
            body { (body) }
        }
    }
}

pub fn header(account_id: AccountId) -> Markup {
    html! {
        nav.navbar.is-light role="navigation" aria-label="main navigation" {
            div.container {
                div."navbar-brand" {
                    a."navbar-item" href="/" {
                        span.icon { i."fas"."fa-home" {} }
                        span { "Home" }
                    }
                    a.navbar-item href=(get_account_url(account_id)) {
                        span.icon { i.fas.fa-users {} }
                        span { "Player" }
                    }
                }
                div."navbar-menu" {
                    div.navbar-end {
                        form.navbar-item action="/" method="GET" {
                            (account_search("is-small", false))
                        }
                    }
                }
            }
        }
    }
}

pub fn footer() -> Markup {
    html! {
        footer.footer {
            div.content.has-text-centered {
                p {
                    strong {
                        a href="https://github.com/eigenein/blitz-dashboard" {
                            "Blitz Dashboard " (crate_version!())
                        }
                    } " by "
                    a href="https://github.com/eigenein" { "@eigenein" } "."
                    " Made with " a href="https://www.rust-lang.org/" { "Rust" }
                    " and " a href="https://bulma.io/" { "Bulma" } "."
                    " The source code is licensed "
                    a href="https://opensource.org/licenses/MIT" { "MIT" } "."
                }
            }
        }
    }
}
