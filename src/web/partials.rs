use chrono::{DateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup};
use phf::phf_set;
use rocket::uri;

use crate::models::{Nation, TankType, Vehicle};
use crate::wargaming::tank_id::to_client_id;
use crate::web::views::analytics::rocket_uri_macro_get as rocket_uri_macro_get_vehicles_analytics;
use crate::web::views::search::{MAX_QUERY_LENGTH, MIN_QUERY_LENGTH};

#[must_use]
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

#[must_use]
pub fn icon_text(class: &str, text: &str) -> Markup {
    html! {
        span.icon-text.is-flex-wrap-nowrap {
            span.icon { i class=(class) {} }
            span { (text) }
        }
    }
}

#[must_use]
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
        link rel="stylesheet" href=(concat!("/static/theme.css?v", structopt::clap::crate_version!()));
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/5.15.3/css/all.min.css" integrity="sha512-iBBXm8fW90+nuLcSKlbmrPcLa0OT92xO1BIsZ+ywDWZCvqsWgccV3gFoRBv0z+8dLJgyAHIhR35VZc2oM/gI1w==" crossorigin="anonymous" referrerpolicy="no-referrer";
    }
}

#[must_use]
pub fn datetime(value: DateTime<Utc>, tense: Tense) -> Markup {
    html! {
        time
            datetime=(value.to_rfc3339())
            title=(value) { (HumanTime::from(value).to_text_en(Accuracy::Rough, tense)) }
    }
}

#[must_use]
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
                                        "Blitz Dashboard"
                                    }
                                    " "
                                    a href=(format!("https://github.com/eigenein/blitz-dashboard/releases/tag/{}", crate::CRATE_VERSION)) {
                                        (crate::CRATE_VERSION)
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

                    div.column."is-2" {
                        p.title."is-6" { "Аналитика" }
                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-truck-monster.has-text-info {} }
                                span { a href=(uri!(get_vehicles_analytics())) { "Танки" } }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[must_use]
pub fn home_button() -> Markup {
    html! {
        a.navbar-item href="/" {
            img src="/android-chrome-192x192.png" width="28" height="28" alt="На главную";
        }
    }
}

#[must_use]
pub fn conditional_class(condition: bool, class: &'static str) -> &'static str {
    if condition {
        class
    } else {
        ""
    }
}

#[must_use]
pub fn vehicle_th(vehicle: &Vehicle) -> Markup {
    html! {
        th.is-white-space-nowrap {
            (vehicle_title(vehicle))
        }
    }
}

#[must_use]
pub fn vehicle_title(vehicle: &Vehicle) -> Markup {
    let flag = match vehicle.nation {
        Nation::China => "flag-icon-cn",
        Nation::Europe => "flag-icon-eu",
        Nation::France => "flag-icon-fr",
        Nation::Germany => "flag-icon-de",
        Nation::Japan => "flag-icon-jp",
        Nation::Other => "flag-icon-xx",
        Nation::Uk => "flag-icon-gb",
        Nation::Usa => "flag-icon-us",
        Nation::Ussr => "flag-icon-su",
    };
    let name_class = if vehicle.is_premium {
        if COLLECTIBLE_VEHICLE_IDS.contains(&vehicle.tank_id) {
            "has-text-info-dark"
        } else {
            "has-text-warning-dark"
        }
    } else if vehicle.type_ == TankType::Unknown {
        "has-text-grey"
    } else {
        ""
    };

    html! {
        span.icon-text.is-flex-wrap-nowrap title=(vehicle.tank_id) {
            span.flag-icon.(flag) {}
            span {
                @if let Some(tier) = TIER_MARKUP.get(&vehicle.tier) {
                    strong."mx-1" { (tier) }
                }
                strong."mx-1".(name_class) { (vehicle.name) }
            }
            @if let Ok(external_id) = to_client_id(vehicle.tank_id) {
                span.icon {
                    a
                        title="Открыть в Blitz Ангар"
                        href=(format!("https://blitzhangar.com/ru/tank/{}", external_id))
                        target="_blank"
                        rel="noopener noreferrer" {
                            i.fas.fa-external-link-alt.has-text-grey-light {}
                        }
                }
            }
        }

    }
}

#[must_use]
pub fn render_float(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
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

#[must_use]
pub fn tier_td(tier: i32, class: Option<&str>) -> Markup {
    html! {
        td.has-text-centered.(class.unwrap_or("")) data-sort="tier" data-value=(tier) {
            @if let Some(markup) = TIER_MARKUP.get(&tier) {
                strong { (markup) }
            }
        }
    }
}

static COLLECTIBLE_VEHICLE_IDS: phf::Set<u16> = phf_set! {
    113_u16,
    1537_u16,
    1617_u16,
    2577_u16,
    2881_u16,
    2913_u16,
    3105_u16,
    3201_u16,
    4449_u16,
    4705_u16,
    4721_u16,
    4945_u16,
    4977_u16,
    4993_u16,
    5233_u16,
    6161_u16,
    6417_u16,
    6481_u16,
    7041_u16,
    7537_u16,
    7713_u16,
    8049_u16,
    8209_u16,
    8257_u16,
    8305_u16,
    8561_u16,
    8817_u16,
    8833_u16,
    9009_u16,
    9345_u16,
    10017_u16,
    10529_u16,
    11265_u16,
    13329_u16,
    15137_u16,
    15889_u16,
    16145_u16,
    17217_u16,
    17745_u16,
    19009_u16,
    20817_u16,
    21073_u16,
    21249_u16,
    21281_u16,
    22033_u16,
    22049_u16,
    22273_u16,
    22785_u16,
    22801_u16,
    23553_u16,
    23569_u16,
    24081_u16,
    24097_u16,
    24609_u16,
    24849_u16,
    51201_u16,
    52065_u16,
    52241_u16,
    52257_u16,
    52481_u16,
    52993_u16,
    55057_u16,
    60929_u16,
    63585_u16,
    63841_u16,
    64849_u16,
};
