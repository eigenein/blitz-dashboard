use chrono::{DateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup};

use crate::models::{Nation, TankType, Vehicle};
use crate::web::routes::search::{MAX_QUERY_LENGTH, MIN_QUERY_LENGTH};

pub fn account_search(
    class: &str,
    value: &str,
    has_autofocus: bool,
    has_user_secret: bool,
) -> Markup {
    html! {
        div.field.has-addons {
            div.control {
                span.select.is-rounded.(class) {
                    select {
                        option title="Россия" { "🇷🇺" }
                    }
                }
            }
            div.control.has-icons-left.is-expanded.(conditional_class(has_user_secret, "has-icons-right")) {
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
                    size=(&(MAX_QUERY_LENGTH + 2))
                    autofocus[has_autofocus]
                    required;
                span.icon.is-left.(class) { i class="fas fa-user" {} }
                @if has_user_secret {
                    span.icon.is-right.(class) { i class="fas fa-user-secret" {} }
                }
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
        link rel="stylesheet" href="https://unpkg.com/bulma-prefers-dark";
        link rel="stylesheet" href="/static/theme.css?v4";
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
    }
}

pub fn datetime(value: DateTime<Utc>, tense: Tense) -> Markup {
    html! {
        time
            datetime=(value.to_rfc3339())
            title=(value) { (HumanTime::from(value).to_text_en(Accuracy::Rough, tense)) }
    }
}

pub fn footer() -> Markup {
    html! {
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
                                        "Blitz Dashboard " (crate::CRATE_VERSION)
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
                }
            }
        }
    }
}

pub fn home_button() -> Markup {
    html! {
        a.button.is-link.is-rounded href="/" {
            span.icon { i.fas.fa-home {} }
            span { "На главную" }
        }
    }
}

pub fn conditional_class(condition: bool, class: &'static str) -> &'static str {
    if condition {
        class
    } else {
        ""
    }
}

pub fn vehicle_th(vehicle: &Vehicle) -> Markup {
    let flag = match vehicle.nation {
        Nation::China => "🇨🇳",
        Nation::Europe => "🇪🇺",
        Nation::France => "🇫🇷",
        Nation::Germany => "🇩🇪",
        Nation::Japan => "🇯🇵",
        Nation::Other => "🏳",
        Nation::Uk => "🇬🇧",
        Nation::Usa => "🇺🇸",
        Nation::Ussr => "🇷🇺",
    };
    let name_class = if vehicle.is_premium {
        "has-text-warning-dark"
    } else if vehicle.type_ == TankType::Unknown {
        "has-text-grey"
    } else {
        ""
    };
    html! {
        th.is-white-space-nowrap title=(vehicle.tank_id) {
            span."mx-1" { (flag) }
            strong."mx-1".(name_class) { (vehicle.name) }
        }
    }
}

pub fn render_f64(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
    }
}

pub fn margin_class(value: f64, level_success: f64, level_warning: f64) -> &'static str {
    match value {
        _ if value < level_success => "",
        _ if value < level_warning => "has-text-warning-dark",
        _ => "has-text-danger",
    }
}

pub fn sign_class(value: f64) -> &'static str {
    if value > 0.0 {
        "has-background-success-light"
    } else if value < 0.0 {
        "has-background-danger-light"
    } else {
        ""
    }
}

pub static TIER_MARKUP: phf::Map<i32, &'static str> = phf::phf_map! {
    1_i32 => "Ⅰ",
    2_i32 => "Ⅱ",
    3_i32 => "Ⅲ",
    4_i32 => "Ⅳ",
    5_i32 => "Ⅴ",
    6_i32 => "Ⅵ",
    7_i32 => "Ⅶ",
    8_i32 => "Ⅷ",
    9_i32 => "Ⅸ",
    10_i32 => "Ⅹ",
};

pub fn tier_td(tier: i32) -> Markup {
    html! {
        td.has-text-centered data-sort="tier" data-value=(tier) {
            @if let Some(markup) = TIER_MARKUP.get(&tier) {
                strong { (markup) }
            }
        }
    }
}
