use std::ops::Range;

use maud::{html, Markup};

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
