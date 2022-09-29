mod account_search;
mod float;
mod human_float;
mod semaphore;

use chrono::{DateTime, Utc};
use chrono_humanize::{Accuracy, HumanTime, Tense};
use maud::{html, Markup};
use phf::phf_set;
use poem::i18n::Locale;

pub use self::account_search::*;
pub use self::float::*;
pub use self::human_float::*;
pub use self::semaphore::*;
use crate::prelude::*;
use crate::wargaming::models::tank_id::to_client_id;

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
        link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.1.2/css/all.min.css" integrity="sha512-1sCRPdkRXhBV2PBLUdRb4tMg1w2YPf37qatUFeS7zlBy7jJI8Lf4VHwWfZZfpXtYSLy85pkm9GaYVYMfw5BC1A==" crossorigin="anonymous" referrerpolicy="no-referrer";
        @if let Some(span) = sentry::configure_scope(|scope| scope.get_span()) {
            @for (key, value) in span.iter_headers() {
                meta name=(key) content=(value);
            }
        }
        script src="https://js.sentry-cdn.com/975bd87a20414620b4ab4d59e9698604.min.js" crossorigin="anonymous" {}
    }
}

#[must_use]
pub fn datetime(value: DateTime<Utc>, tense: Tense) -> Markup {
    html! {
        time
            datetime=(value.to_rfc3339())
            title=(maud::display(value)) { (HumanTime::from(value).to_text_en(Accuracy::Rough, tense)) }
    }
}

pub fn footer(locale: &Locale) -> Result<Markup> {
    let markup = html! {
        footer.footer {
            div.container {
                div.columns {
                    div.column."is-3" {
                        p.title."is-6" { (locale.text("footer-title-about")?) }

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
                                span.icon { i.fas.fa-heart.has-text-danger {} }
                                span {
                                    (locale.text("footer-title-created-with")?)
                                    " "
                                    a href="https://www.rust-lang.org/" { "Rust" }
                                    " "
                                    (locale.text("preposition-and")?)
                                    " "
                                    a href="https://bulma.io/" { "Bulma" }
                                }
                            }
                        }

                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-id-badge.has-text-success {} }
                                span {
                                    (locale.text("footer-title-source-licensed")?)
                                    " "
                                    a href="https://opensource.org/licenses/MIT" { "MIT" }
                                }
                            }
                        }
                    }

                    div.column."is-2" {
                        p.title."is-6" { (locale.text("footer-title-support")?) }

                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-comments.has-text-info {} }
                                span { a href="https://github.com/eigenein/blitz-dashboard/discussions" {
                                    (locale.text("footer-title-discussions")?)
                                } }
                            }
                        }

                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fab.fa-github.has-text-danger {} }
                                span { a href="https://github.com/eigenein/blitz-dashboard/issues" {
                                    (locale.text("footer-title-issues")?)
                                } }
                            }
                        }

                        p."mt-1" {
                            span.icon-text.is-flex-wrap-nowrap {
                                span.icon { i.fas.fa-code-branch.has-text-success {} }
                                span { a href="https://github.com/eigenein/blitz-dashboard/pulls" {
                                    (locale.text("footer-title-pull-requests")?)
                                } }
                            }
                        }
                    }
                }
            }
        }
    };
    Ok(markup)
}

pub fn home_button(locale: &Locale) -> Result<Markup> {
    let markup = html! {
        a.navbar-item href="/" {
            img src="/android-chrome-192x192.png" width="28" height="28" alt=(locale.text("alt-home")?);
        }
    };
    Ok(markup)
}

pub fn vehicle_th(vehicle: &wargaming::Vehicle, locale: &Locale) -> Result<Markup> {
    let markup = html! {
        th.is-white-space-nowrap {
            (vehicle_title(vehicle, locale)?)
        }
    };
    Ok(markup)
}

pub fn vehicle_title(vehicle: &wargaming::Vehicle, locale: &Locale) -> Result<Markup> {
    let flag = match vehicle.nation {
        wargaming::Nation::China => "flag-icon-cn",
        wargaming::Nation::Europe => "flag-icon-eu",
        wargaming::Nation::France => "flag-icon-fr",
        wargaming::Nation::Germany => "flag-icon-de",
        wargaming::Nation::Japan => "flag-icon-jp",
        wargaming::Nation::Other => "flag-icon-xx",
        wargaming::Nation::Uk => "flag-icon-gb",
        wargaming::Nation::Usa => "flag-icon-us",
        wargaming::Nation::Ussr => "flag-icon-su",
    };
    let name_class = if vehicle.is_premium {
        if COLLECTIBLE_VEHICLE_IDS.contains(&vehicle.tank_id) {
            "has-text-info-dark"
        } else {
            "has-text-warning-dark"
        }
    } else if vehicle.type_ == wargaming::TankType::Unknown {
        "has-text-grey"
    } else {
        ""
    };

    let markup = html! {
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
                        title=(locale.text("title-open-in-blitzhangar")?)
                        href=(format!("https://blitzhangar.com/tank/{}", external_id))
                        target="_blank"
                        rel="noopener noreferrer"
                    {
                        i.fas.fa-external-link-alt.has-text-grey-light {}
                    }
                }
            }
        }

    };
    Ok(markup)
}

#[must_use]
pub fn render_float(value: f64, precision: usize) -> Markup {
    html! {
        span title=(value) {
            (format!("{:.1$}", value, precision))
        }
    }
}

pub static TIER_MARKUP: phf::Map<wargaming::Tier, &'static str> = phf::phf_map! {
    1_u8 => "Ⅰ",
    2_u8 => "Ⅱ",
    3_u8 => "Ⅲ",
    4_u8 => "Ⅳ",
    5_u8 => "Ⅴ",
    6_u8 => "Ⅵ",
    7_u8 => "Ⅶ",
    8_u8 => "Ⅷ",
    9_u8 => "Ⅸ",
    10_u8 => "Ⅹ",
};

static COLLECTIBLE_VEHICLE_IDS: phf::Set<wargaming::TankId> = phf_set! {
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
