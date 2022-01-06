use maud::{html, Markup, PreEscaped, DOCTYPE};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use rocket::response::content::Html;
use rocket::{uri, State};

use crate::aggregator::model::retrieve_vehicle_win_rates;
use crate::logging::clear_user;
use crate::math::statistics::ConfidenceInterval;
use crate::tankopedia::get_vehicle;
use crate::wargaming::tank_id::TankId;
use crate::web::partials::{footer, headers, home_button, tier_td, vehicle_th};
use crate::web::views::analytics::vehicle::rocket_uri_macro_get as rocket_uri_macro_get_vehicle_analytics;
use crate::web::views::bulma::*;
use crate::web::{DisableCaches, TrackingCode};

pub mod vehicle;

#[rocket::get("/analytics/vehicles")]
pub async fn get(
    tracking_code: &State<TrackingCode>,
    redis: &State<MultiplexedConnection>,
    disable_caches: &State<DisableCaches>,
) -> crate::web::result::Result<Html<String>> {
    clear_user();

    let mut redis = MultiplexedConnection::clone(redis);
    const CACHE_KEY: &str = "html::analytics::vehicles";
    if !disable_caches.0 {
        if let Some(cached_response) = redis.get(CACHE_KEY).await? {
            return Ok(Html(cached_response));
        }
    }

    let live_win_rates = retrieve_vehicle_win_rates(&mut redis).await?;

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                (headers())
                title { "Аналитика по танкам – Я же статист!" }
            }
        }
        body {
            (tracking_code.0)
            nav.navbar.has-shadow.is-fixed-top role="navigation" aria-label="main navigation" {
                div.container {
                    div.navbar-brand {
                        (home_button())
                    }
                }
            }

            section.section {
                div.container {
                    div.box {
                        div.table-container {
                            table.table.is-hoverable.is-striped.is-fullwidth id="vehicle-factors" {
                                thead {
                                    th { "Техника" }

                                    th { }

                                    th.has-text-centered {
                                        a data-sort="tier" {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span { "Уровень" }
                                            }
                                        }
                                    }

                                    th.is-white-space-nowrap {
                                        sup title="В разработке" { strong.has-text-danger-dark { "ɑ" } }
                                        a data-sort="live-win-rate" {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span { abbr title="Средний процент побед этого танка по всему региону за последние несколько часов (сортировка по нижней границе интервала)" { "Live WR" } }
                                            }
                                        }
                                    }
                                }

                                tbody {
                                    @for (tank_id, win_rate) in live_win_rates.into_iter() {
                                        (tr(tank_id, win_rate))
                                    }
                                }
                            }
                        }
                    }
                }
            }

            (footer())

            script type="module" {
                (PreEscaped(r#"""
                    "use strict";
                    
                    import { initSortableTable } from "/static/table.js?v5";
                    
                    (function () {
                        initSortableTable(document.getElementById("vehicle-factors"), "tier");
                    })();
                """#))
            }
        }
    };

    let response = markup.into_string();
    redis.set_ex(CACHE_KEY, &response, 60).await?;
    Ok(Html(response))
}

#[must_use]
pub fn tr(tank_id: TankId, live_win_rate: ConfidenceInterval) -> Markup {
    html! {
        tr {
            @let vehicle = get_vehicle(tank_id);

            (vehicle_th(&vehicle))

            td.has-text-centered {
                a href=(uri!(get_vehicle_analytics(tank_id = tank_id))) {
                    span.icon-text.is-flex-wrap-nowrap {
                        span.icon { { i.fas.fa-link {} } }
                    }
                }
            }

            (tier_td(vehicle.tier, None))

            td.is-white-space-nowrap data-sort="live-win-rate" data-value=(live_win_rate.lower()) {
                span.icon-text.is-flex-wrap-nowrap {
                    (Icon::ChartArea.into_span().color(Color::GreyLight))
                    span {
                        strong title=(live_win_rate.mean) {
                            (format!("{:.1}%", live_win_rate.mean * 100.0))
                        }
                        span.has-text-grey { (format!(" ±{:.1}", live_win_rate.margin * 100.0)) }
                    }
                }
            }
        }
    }
}
