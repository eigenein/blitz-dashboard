use std::ops::Range;

use maud::{html, Markup, DOCTYPE};

pub mod footer;

pub const SEARCH_QUERY_LENGTH: Range<usize> = MIN_QUERY_LENGTH..(MAX_QUERY_LENGTH + 1);
const MIN_QUERY_LENGTH: usize = 3;
const MAX_QUERY_LENGTH: usize = 24;

pub fn account_search(class: &str, nickname: &str, has_autofocus: bool) -> Markup {
    html! {
        div class="field has-addons" {
            div class="control" {
                span."select"."is-rounded".(class) {
                    select {
                        option { "🇷🇺 RU" }
                    }
                }
            }
            div class="control has-icons-left is-expanded" {
                input."input"."is-rounded".(class)
                    type="text"
                    name="search"
                    value=(nickname)
                    placeholder="Nickname"
                    autocomplete="nickname"
                    pattern="\\w+"
                    autocapitalize="none"
                    minlength=(MIN_QUERY_LENGTH)
                    maxlength=(MAX_QUERY_LENGTH)
                    autofocus[has_autofocus]
                    required;
                span.icon.is-left.(class) {
                    i class="fas fa-user" {}
                }
            }
            div.control {
                input.button.is-rounded.is-link.(class) type="submit" value="Search";
            }
        }
    }
}

pub fn icon_text(class: &str, text: &str) -> Markup {
    html! {
        span class="icon-text" {
            span.icon { i class=(class) {} }
            span { (text) }
        }
    }
}

pub fn head(title: Option<&str>) -> Markup {
    html! {
        title { @if let Some(title) = title { (title) " – " } "Blitz Dashboard" }
        meta name="viewport" content="width=device-width, initial-scale=1";
        meta charset="UTF-8";
        link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png";
        link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png";
        link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png";
        link rel="manifest" href="/site.webmanifest";
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

pub fn header(nickname: &str) -> Markup {
    html! {
        nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
            div.container {
                div."navbar-brand" {
                    div.navbar-item {
                        div.buttons {
                            a.button.is-link href="/" {
                                span.icon { i.fas.fa-home {} }
                                span { "Home" }
                            }
                        }
                    }
                }
                div."navbar-menu" {
                    div.navbar-end {
                        form.navbar-item action="/" method="GET" {
                            (account_search("", nickname, false))
                        }
                    }
                }
            }
        }
    }
}
