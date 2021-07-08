use std::ops::Range;

use chrono::{DateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use clap::crate_version;
use humantime::format_duration;
use maud::{html, Markup};

use crate::web::state::State;

pub const SEARCH_QUERY_LENGTH: Range<usize> = MIN_QUERY_LENGTH..(MAX_QUERY_LENGTH + 1);
const MIN_QUERY_LENGTH: usize = 3;
const MAX_QUERY_LENGTH: usize = 24;

pub fn account_search(class: &str, value: &str, has_autofocus: bool) -> Markup {
    html! {
        div.field.has-addons {
            div.control {
                span.select.is-rounded.(class) {
                    select {
                        option title="Россия" { "🇷🇺" }
                    }
                }
            }
            div.control.has-icons-left.is-expanded {
                input.input.is-rounded.(class)
                    type="search"
                    name="query"
                    value=(value)
                    placeholder="Никнейм"
                    autocomplete="nickname"
                    pattern="\\w+"
                    autocapitalize="none"
                    minlength=(MIN_QUERY_LENGTH)
                    maxlength=(MAX_QUERY_LENGTH)
                    onclick="this.select();"
                    spellcheck="false"
                    autocorrect="off"
                    aria-label="search"
                    aria-haspopup="false"
                    size=(MAX_QUERY_LENGTH)
                    autofocus[has_autofocus]
                    required;
                span.icon.is-left.(class) { i class="fas fa-user" {} }
            }
            div.control {
                button.button.is-rounded.is-link.(class) type="submit" {
                    span.icon.is-hidden-desktop { i.fas.fa-search {} }
                    span.is-hidden-touch { "Поиск" }
                };
            }
        }
    }
}

pub fn icon_text(class: &str, text: &str) -> Markup {
    html! {
        span.icon-text.is-flex-wrap-nowrap {
            span.icon { i class=(class) {} }
            span { (text) }
        }
    }
}

pub fn headers() -> Markup {
    html! {
        meta name="viewport" content="width=device-width, initial-scale=1";
        meta charset="UTF-8";
        link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png";
        link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png";
        link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png";
        link rel="manifest" href="/site.webmanifest";
        link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.3/css/bulma.min.css" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href="https://unpkg.com/bulma-prefers-dark";
        style { ".is-white-space-nowrap { white-space: nowrap !important; }" }
    }
}

pub fn datetime(value: DateTime<Utc>, tense: Tense) -> Markup {
    html! {
        time
            datetime=(value.to_rfc3339())
            title=(value) { (HumanTime::from(value).to_text_en(Accuracy::Rough, tense)) }
    }
}

pub async fn footer(state: &State) -> crate::Result<Markup> {
    let account_count = state.retrieve_account_count().await?;
    let account_snapshot_count = state.retrieve_account_snapshot_count().await?;
    let tank_snapshot_count = state.retrieve_tank_snapshot_count().await?;
    let crawler_lag = state.retrieve_crawler_lag().await?;

    let markup = html! {
        footer.footer {
            div.container {
                div.columns {
                    div.column."is-3" {
                        p.title."is-6" { "О проекте" }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-home.has-text-info {} }
                                span {
                                    a href="https://github.com/eigenein/blitz-dashboard" {
                                        "Blitz Dashboard " (crate_version!())
                                    }
                                    " © "
                                    a href="https://github.com/eigenein" { "@eigenein" }
                                }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-heart.has-text-danger {} }
                                span {
                                    "Создан с помощью " a href="https://www.rust-lang.org/" { "Rust" }
                                    " и " a href="https://bulma.io/" { "Bulma" }
                                }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-id-badge.has-text-success {} }
                                span { "Исходный код лицензирован " a href="https://opensource.org/licenses/MIT" { "MIT" } }
                            }
                        }
                    }

                    div.column."is-2" {
                        p.title."is-6" { "Поддержка" }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-comments.has-text-info {} }
                                span { a href="https://github.com/eigenein/blitz-dashboard/discussions" { "Обсуждения" } }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fab.fa-github.has-text-danger {} }
                                span { a href="https://github.com/eigenein/blitz-dashboard/issues" { "Задачи и баги" } }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-code-branch.has-text-success {} }
                                span { a href="https://github.com/eigenein/blitz-dashboard/pulls" { "Пул-реквесты" } }
                            }
                        }
                    }

                    div.column."is-3" {
                        p.title."is-6" { "Статистика" }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-user.has-text-info {} }
                                span { strong { (account_count) } " аккаунтов" }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-portrait.has-text-info {} }
                                span { strong { (account_snapshot_count) } " снимков аккаунтов" }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-truck-monster.has-text-info {} }
                                span { strong { (tank_snapshot_count) } " снимков танков" }
                            }
                        }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-clock.has-text-info {} }
                                span { "Лаг робота " strong { (format_duration(crawler_lag.to_std()?)) } }
                            }
                        }
                    }
                }
            }
        }
    };

    Ok(markup)
}
