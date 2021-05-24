use maud::{html, Markup};
use std::ops::Range;

pub const SEARCH_QUERY_LENGTH: Range<usize> = MIN_QUERY_LENGTH..(MAX_QUERY_LENGTH + 1);
const MIN_QUERY_LENGTH: usize = 3;
const MAX_QUERY_LENGTH: usize = 24;

/// Documentation: <https://bulma.io/documentation/components/card/>.
pub fn card(title: Option<Markup>, content: Markup) -> Markup {
    html! {
        div.card {
            @if let Some(title) = title {
                header class="card-header" {
                    p class="card-header-title" { (title) }
                }
            }
            div class="card-content" {
                (content)
            }
        }
    }
}

pub fn search_accounts() -> Markup {
    html! {
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
                input
                    class="input is-medium is-rounded"
                    type="text"
                    name="search"
                    placeholder="Player nickname"
                    autocomplete="nickname"
                    pattern="\\w+"
                    minlength=(MIN_QUERY_LENGTH)
                    maxlength=(MAX_QUERY_LENGTH)
                    autofocus
                    required;
                span class="icon is-medium is-left" {
                    i class="fas fa-user" {}
                }
            }
            div class="control" {
                input class="button is-medium is-rounded is-link" type="submit" value="Search";
            }
        }
    }
}
