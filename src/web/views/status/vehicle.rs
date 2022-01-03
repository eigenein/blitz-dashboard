use std::cmp::Ordering;

use itertools::Itertools;
use maud::{html, DOCTYPE};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use rocket::http::Status;
use rocket::{uri, State};

use crate::logging::clear_user;
use crate::math::vector::cosine_similarity;
use crate::tankopedia::get_vehicle;
use crate::trainer::model::get_all_vehicle_factors;
use crate::wargaming::tank_id::TankId;
use crate::web::partials::{
    factors_table, footer, headers, home_button, tier_td, vehicle_th, vehicle_title,
};
use crate::web::response::CustomResponse;
use crate::web::views::status::vehicle::rocket_uri_macro_get as rocket_uri_macro_get_vehicle;
use crate::web::{DisableCaches, TrackingCode};

#[rocket::get("/status/vehicle/<tank_id>")]
pub async fn get(
    tank_id: TankId,
    tracking_code: &State<TrackingCode>,
    redis: &State<MultiplexedConnection>,
    disable_caches: &State<DisableCaches>,
) -> crate::web::result::Result<CustomResponse> {
    clear_user();

    let mut redis = MultiplexedConnection::clone(redis);
    let cache_key = format!("html::status::vehicle::{}", tank_id);
    if !disable_caches.0 {
        if let Some(cached_response) = redis.get(&cache_key).await? {
            return Ok(CustomResponse::Html(cached_response));
        }
    }

    let vehicles_factors = get_all_vehicle_factors(&mut redis).await?;
    let vehicle_factors = match vehicles_factors.get(&tank_id) {
        Some(factors) => factors,
        None => return Ok(CustomResponse::Status(Status::NotFound)),
    };

    let vehicle = get_vehicle(tank_id);
    let table: Vec<(u16, f64)> = vehicles_factors
        .iter()
        .filter(|(other_tank_id, _)| **other_tank_id != tank_id)
        .map(|(tank_id, other_factors)| {
            (*tank_id, cosine_similarity(vehicle_factors, other_factors))
        })
        .sorted_unstable_by(|(_, left), (_, right)| {
            right.partial_cmp(left).unwrap_or(Ordering::Equal)
        })
        .take(40)
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
                        (home_button())
                        div.navbar-item {
                            (vehicle_title(&vehicle))
                        }
                    }
                }
            }

            section.section."p-0"."m-5" {
                div.container {
                    div.card {
                        header.card-header {
                            p.card-header-title {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-truck-monster {} }
                                    span { "Похожая техника" }
                                }
                            }
                        }
                        div.card-content {
                            div.table-container {
                                table.table.is-hoverable.is-striped.is-fullwidth {
                                    thead {
                                        th { "Техника" }
                                        th.has-text-centered { "Уровень" }
                                        th { "Тип" }
                                        th { "Схожесть" }
                                    }
                                    tbody {
                                        @for (tank_id, coefficient) in table {
                                            @let other_vehicle = get_vehicle(tank_id);
                                            tr {
                                                (vehicle_th(&other_vehicle))
                                                (tier_td(other_vehicle.tier, (vehicle.tier == other_vehicle.tier).then(|| "has-background-success-light")))
                                                td.(if vehicle.type_ == other_vehicle.type_ { "has-background-success-light" } else { "" }) {
                                                    (format!("{:?}", other_vehicle.type_))
                                                }
                                                td {
                                                    a href=(uri!(get_vehicle(tank_id = tank_id))) { (format!("{:.4}", coefficient)) }
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

            section.section."p-0"."m-5" {
                div.container {
                    div.card {
                        header.card-header {
                            p.card-header-title {
                                span.icon-text.is-flex-wrap-nowrap {
                                    span.icon { i.fas.fa-superscript {} }
                                    span { "Скрытые признаки" }
                                }
                            }
                        }
                        div.card-content {
                            (factors_table(vehicle_factors))
                        }
                    }
                }
            }

            (footer())
        }
    };

    let response = markup.into_string();
    redis.set_ex(&cache_key, &response, 60).await?;
    Ok(CustomResponse::Html(response))
}
