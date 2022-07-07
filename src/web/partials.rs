use chrono::{DateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup, PreEscaped};
use phf::phf_set;

use crate::wargaming::models::tank_id::to_client_id;
use crate::wargaming::{Nation, Realm, TankId, TankType, Vehicle};
use crate::web::views::search::{MAX_QUERY_LENGTH, MIN_QUERY_LENGTH};

#[must_use]
pub fn account_search(
    class: &str,
    realm: Realm,
    value: &str,
    has_autofocus: bool,
    has_user_secret: bool,
) -> Markup {
    html! {
        div.field.has-addons {
            div.control {
                span.select.is-rounded.(class) {
                    select name="realm" {
                        option title="Россия" value=(Realm::Russia.to_str()) selected[realm == Realm::Russia] { "🇷🇺" }
                        option title="Европа" value=(Realm::Europe.to_str()) selected[realm == Realm::Europe] { "🇪🇺" }
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
                    size="20"
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
pub fn headers() -> Markup {
    html! {
        meta name="viewport" content="width=device-width, initial-scale=1";
        meta charset="UTF-8";
        link rel="apple-touch-icon" sizes="180x180" href="/apple-touch-icon.png";
        link rel="icon" type="image/png" sizes="32x32" href="/favicon-32x32.png";
        link rel="icon" type="image/png" sizes="16x16" href="/favicon-16x16.png";
        link rel="manifest" href="/site.webmanifest";
        link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma@0.9.4/css/bulma.min.css" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href="https://cdn.jsdelivr.net/npm/bulma-prefers-dark@0.1.0-beta.1/css/bulma-prefers-dark.min.css" crossorigin="anonymous" referrerpolicy="no-referrer";
        link rel="stylesheet" href=(concat!("/static/theme.css?v", clap::crate_version!()));
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.1.1/css/all.min.css" integrity="sha512-KfkfwYDsLkIlwQp6LFnl8zNdLGxu9YAA1QvwINks4PhcElQSvqcyVLLD9aMhXd13uQjoXtEKNosOWaZqXgel0g==" crossorigin="anonymous" referrerpolicy="no-referrer";
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
                                }
                            }
                        }

                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fa-regular.fa-copyright.has-text-warning {} }
                                span { a href="https://github.com/eigenein" { "@eigenein" } }
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
    if condition { class } else { "" }
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
                        rel="noopener noreferrer"
                    {
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

#[must_use]
pub fn sign_class(value: f64) -> PreEscaped<&'static str> {
    if value > 0.0 {
        PreEscaped("has-text-success")
    } else if value < 0.0 {
        PreEscaped("has-text-danger")
    } else {
        PreEscaped("")
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

static COLLECTIBLE_VEHICLE_IDS: phf::Set<TankId> = phf_set! {
    113_u32,
    1537_u32,
    1617_u32,
    2577_u32,
    2881_u32,
    2913_u32,
    3105_u32,
    3201_u32,
    4449_u32,
    4705_u32,
    4721_u32,
    4945_u32,
    4977_u32,
    4993_u32,
    5233_u32,
    6161_u32,
    6417_u32,
    6481_u32,
    7041_u32,
    7537_u32,
    7713_u32,
    8049_u32,
    8209_u32,
    8257_u32,
    8305_u32,
    8561_u32,
    8817_u32,
    8833_u32,
    9009_u32,
    9345_u32,
    10017_u32,
    10529_u32,
    11265_u32,
    13329_u32,
    15137_u32,
    15889_u32,
    16145_u32,
    17217_u32,
    17745_u32,
    19009_u32,
    20817_u32,
    21073_u32,
    21249_u32,
    21281_u32,
    22033_u32,
    22049_u32,
    22273_u32,
    22785_u32,
    22801_u32,
    23553_u32,
    23569_u32,
    24081_u32,
    24097_u32,
    24609_u32,
    24849_u32,
    51201_u32,
    52065_u32,
    52241_u32,
    52257_u32,
    52481_u32,
    52993_u32,
    55057_u32,
    60929_u32,
    63585_u32,
    63841_u32,
    64849_u32,
};
