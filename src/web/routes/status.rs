use maud::{html, PreEscaped, DOCTYPE};
use redis::aio::ConnectionManager as Redis;
use rocket::response::content::Html;
use rocket::State;

use crate::cf::N_FACTORS;
use crate::logging::clear_user;
use crate::redis::get_all_vehicle_factors;
use crate::tankopedia::get_vehicle;
use crate::web::partials::{
    footer, headers, home_button, render_f64, sign_class, tier_td, vehicle_th,
};
use crate::web::TrackingCode;

#[rocket::get("/status")]
pub async fn get(
    tracking_code: &State<TrackingCode>,
    redis: &State<Redis>,
) -> crate::web::result::Result<Html<String>> {
    clear_user();

    let mut redis = Redis::clone(redis);
    let vehicle_factors = get_all_vehicle_factors(&mut redis).await?;

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
                    h1.title { "Collaborative Filtering" }

                    h2.title."is-4" { "Vehicle Latent Factors" }
                    div.box {
                        div.table-container {
                            table#vehicle-factors.table.is-hoverable.is-striped.is-fullwidth {
                                thead {
                                    th { "Vehicle" }
                                    th {
                                        a data-sort="tier" {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span { "Tier" }
                                            }
                                        }
                                    }
                                    th {
                                        a data-sort="factor-0" {
                                            span.icon-text.is-flex-wrap-nowrap {
                                                span { "Bias" }
                                            }
                                        }
                                    }
                                    @for i in 1..(N_FACTORS + 1) {
                                        th {
                                            a data-sort=(format!("factor-{}", i)) {
                                                span.icon-text.is-flex-wrap-nowrap {
                                                    span { "Factor #" (i) }
                                                }
                                            }
                                        }
                                    }
                                }
                                tbody {
                                    @for (tank_id, factors) in vehicle_factors.into_iter() {
                                        tr {
                                            @let vehicle = get_vehicle(tank_id);
                                            (vehicle_th(&vehicle))
                                            (tier_td(vehicle.tier))
                                            @for (i, factor) in factors.into_iter().enumerate() {
                                                td.(sign_class(factor)) data-sort=(format!("factor-{}", i)) data-value=(factor) {
                                                    (render_f64(factor, 4))
                                                }
                                            }
                                        }
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
                        initSortableTable(document.getElementById("vehicle-factors"), "factor-0");
                    })();
                """#))
            }
        }
    };

    Ok(Html(markup.into_string()))
}
