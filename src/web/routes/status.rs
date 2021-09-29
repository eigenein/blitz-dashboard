use maud::{html, Markup, PreEscaped, DOCTYPE};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use rocket::response::content::Html;
use rocket::{uri, State};

use crate::logging::clear_user;
use crate::tankopedia::get_vehicle;
use crate::trainer::get_all_vehicle_factors;
use crate::trainer::vector::Vector;
use crate::web::partials::{
    footer, headers, home_button, render_f64, sign_class, tier_td, vehicle_th,
};
use crate::web::routes::status::vehicle::rocket_uri_macro_get as rocket_uri_macro_get_vehicle_status;
use crate::web::TrackingCode;

pub mod vehicle;

#[rocket::get("/status")]
pub async fn get(
    tracking_code: &State<TrackingCode>,
    redis: &State<MultiplexedConnection>,
) -> crate::web::result::Result<Html<String>> {
    clear_user();

    let mut redis = MultiplexedConnection::clone(redis);
    const CACHE_KEY: &str = "html::status";
    if let Some(cached_response) = redis.get(CACHE_KEY).await? {
        return Ok(Html(cached_response));
    }

    let vehicle_factors = get_all_vehicle_factors(&mut redis).await?;
    let n_factors = vehicle_factors
        .values()
        .map(|factors| factors.0.len())
        .max()
        .unwrap_or(0);

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                (headers())
                title { "Состояние приложения – Я же статист!" }
            }
        }
        body {
            (tracking_code.0)
            nav.navbar.has-shadow.is-fixed-top role="navigation" aria-label="main navigation" {
                div.container {
                    div.navbar-brand {
                        div.navbar-item {
                            div.buttons { (home_button()) }
                        }
                    }
                }
            }

            section.section {
                div.container {
                    h1.title { "Машинное обучение" }

                    div.box {
                        h2.title."is-4" { "Признаки техники" }
                        div.table-container {
                            table#vehicle-factors.table.is-hoverable.is-striped.is-fullwidth {
                                (thead(n_factors))
                                tbody {
                                    @for (tank_id, factors) in vehicle_factors.into_iter() {
                                        (tr(tank_id, &factors, n_factors))
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

pub fn thead(n_factors: usize) -> Markup {
    html! {
        thead {
            th { "Техника" }
            th { }
            th {
                a data-sort="tier" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Уровень" }
                    }
                }
            }
            th {
                a data-sort="magnitude" {
                    span.icon-text.is-flex-wrap-nowrap {
                        span { "Модуль"  }
                    }
                }
            }
            @for i in 0..n_factors {
                th {
                    a data-sort=(format!("factor-{}", i)) {
                        span.icon-text.is-flex-wrap-nowrap {
                            span { "#" (i) }
                        }
                    }
                }
            }
        }
    }
}

pub fn tr(tank_id: i32, factors: &Vector, n_factors: usize) -> Markup {
    html! {
        tr {
            @let vehicle = get_vehicle(tank_id);
            (vehicle_th(&vehicle))
            td.has-text-centered {
                a href=(uri!(get_vehicle_status(tank_id = tank_id))) {
                    span.icon-text.is-flex-wrap-nowrap {
                        span.icon { { i.fas.fa-link {} } }
                    }
                }
            }
            (tier_td(vehicle.tier))

            @let magnitude = factors.norm();
            td data-sort="magnitude" data-value=(magnitude) { (render_f64(magnitude, 4)) }

            @for i in 0..n_factors {
                @if let Some(factor) = factors.0.get(i).copied() {
                    td.(sign_class(factor)) data-sort=(format!("factor-{}", i)) data-value=(factor) {
                        (format!("{:+.4}", factor))
                    }
                }
            }
        }
    }
}
