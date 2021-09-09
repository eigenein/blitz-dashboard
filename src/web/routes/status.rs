use maud::{html, DOCTYPE};
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
use itertools::Itertools;
use std::cmp::Ordering;

#[rocket::get("/status")]
pub async fn get(
    tracking_code: &State<TrackingCode>,
    redis: &State<Redis>,
) -> crate::web::result::Result<Html<String>> {
    clear_user();

    let mut redis = Redis::clone(redis);
    // TODO: this should be cached.
    let vehicle_factors: Vec<(i32, Vec<f64>)> = get_all_vehicle_factors(&mut redis)
        .await?
        .into_iter()
        .sorted_unstable_by(|(_, left), (_, right)| {
            right[0].partial_cmp(&left[0]).unwrap_or(Ordering::Equal)
        })
        .collect();

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
                            table.table.is-hoverable.is-striped.is-fullwidth {
                                thead {
                                    th { "Vehicle" }
                                    th { "Tier" }
                                    th { "Bias" }
                                    @for i in 0..N_FACTORS {
                                        th.is-white-space-nowrap { "Factor #" (i) }
                                    }
                                }
                                tbody {
                                    @for (tank_id, factors) in vehicle_factors {
                                        tr {
                                            @let vehicle = get_vehicle(tank_id);
                                            (vehicle_th(&vehicle))
                                            (tier_td(vehicle.tier))
                                            @for factor in factors {
                                                td.(sign_class(factor)) {
                                                    (render_f64(factor, 3))
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
        }
    };

    Ok(Html(markup.into_string()))
}
