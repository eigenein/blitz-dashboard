use std::cmp::Ordering;

use itertools::Itertools;
use maud::{html, DOCTYPE};
use redis::aio::ConnectionManager as Redis;
use redis::AsyncCommands;
use rocket::response::content::Html;
use rocket::response::status::NotFound;
use rocket::{uri, State};

use crate::cf::{cosine_similarity, pearson_coefficient};
use crate::logging::clear_user;
use crate::redis::get_all_vehicle_factors;
use crate::tankopedia::get_vehicle;
use crate::web::partials::{
    footer, headers, home_button, sign_class, tier_td, vehicle_th, vehicle_title,
};
use crate::web::response::Response;
use crate::web::routes::status::vehicle::rocket_uri_macro_get as rocket_uri_macro_get_vehicle;
use crate::web::TrackingCode;

#[rocket::get("/status/vehicle/<tank_id>")]
pub async fn get(
    tank_id: i32,
    tracking_code: &State<TrackingCode>,
    redis: &State<Redis>,
) -> crate::web::result::Result<Response> {
    clear_user();

    let mut redis = Redis::clone(redis);
    let cache_key = format!("html::status::vehicle::{}", tank_id);
    if let Some(cached_response) = redis.get(&cache_key).await? {
        return Ok(Response::Html(Html(cached_response)));
    }

    let vehicles_factors = get_all_vehicle_factors(&mut redis).await?;
    let vehicle_factors = match vehicles_factors.get(&tank_id) {
        Some(factors) => &factors[1..],
        None => return Ok(Response::NotFound(NotFound(()))),
    };

    let vehicle = get_vehicle(tank_id);
    let vehicle_title = vehicle_title(&vehicle);

    let tables: Vec<(Vec<(i32, f64)>, &'static str)> = [cosine_similarity, pearson_coefficient]
        .iter()
        .map(|f| {
            vehicles_factors
                .iter()
                .map(|(tank_id, other_factors)| (*tank_id, f(vehicle_factors, &other_factors[1..])))
                .sorted_unstable_by(|(_, left), (_, right)| {
                    right
                        .abs()
                        .partial_cmp(&left.abs())
                        .unwrap_or(Ordering::Equal)
                })
                .take(40)
                .sorted_unstable_by(|(_, left), (_, right)| {
                    right.partial_cmp(left).unwrap_or(Ordering::Equal)
                })
                .collect()
        })
        .zip(["Косинусное сходство", "r-Пирсона"])
        .collect();

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                (headers())
                title { (vehicle.name) " – Я статист!" }
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
                    h1.title { (vehicle_title) }

                    div.columns.is-multiline {
                        @for (table, title) in &tables {
                            div.column."is-12"."is-6-widescreen" {
                                div.box {
                                    h2.title."is-4" { (title) }
                                    table.table.is-hoverable.is-striped.is-fullwidth {
                                        thead {
                                            th { "Техника" }
                                            th.has-text-centered { "Ур." }
                                            th { "Тип" }
                                            th { "Корр." }
                                        }
                                        tbody {
                                            @for (tank_id, coefficient) in table {
                                                @let other_vehicle = get_vehicle(*tank_id);
                                                tr.(sign_class(*coefficient)) {
                                                    (vehicle_th(&other_vehicle))
                                                    (tier_td(other_vehicle.tier))
                                                    td {
                                                        (format!("{:?}", other_vehicle.type_))
                                                    }
                                                    td {
                                                        a href=(uri!(get_vehicle(tank_id = tank_id))) {
                                                            span.icon-text.is-flex-wrap-nowrap {
                                                                (format!("{:+.3}", coefficient))
                                                                span.icon { { i.fas.fa-link {} } }
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
                    }
                }
            }

            (footer())
        }
    };

    let response = markup.into_string();
    redis.set_ex(&cache_key, &response, 60).await?;
    Ok(Response::Html(Html(response)))
}
