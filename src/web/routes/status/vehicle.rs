use std::cmp::Ordering;

use itertools::Itertools;
use maud::{html, DOCTYPE};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use rocket::response::content::Html;
use rocket::response::status::NotFound;
use rocket::{uri, State};

use crate::logging::clear_user;
use crate::tankopedia::get_vehicle;
use crate::trainer::get_all_vehicle_factors;
use crate::web::partials::{footer, headers, home_button, tier_td, vehicle_th};
use crate::web::response::Response;
use crate::web::routes::status::vehicle::rocket_uri_macro_get as rocket_uri_macro_get_vehicle;
use crate::web::routes::status::{thead as status_thead, tr as status_tr};
use crate::web::TrackingCode;

#[rocket::get("/status/vehicle/<tank_id>")]
pub async fn get(
    tank_id: i32,
    tracking_code: &State<TrackingCode>,
    redis: &State<MultiplexedConnection>,
) -> crate::web::result::Result<Response> {
    clear_user();

    let mut redis = MultiplexedConnection::clone(redis);
    let cache_key = format!("html::status::vehicle::{}", tank_id);
    if let Some(cached_response) = redis.get(&cache_key).await? {
        return Ok(Response::Html(Html(cached_response)));
    }

    let vehicles_factors = get_all_vehicle_factors(&mut redis).await?;
    let vehicle_factors = match vehicles_factors.get(&tank_id) {
        Some(factors) => factors,
        None => return Ok(Response::NotFound(NotFound(()))),
    };

    let vehicle = get_vehicle(tank_id);
    let table: Vec<(i32, f64)> = vehicles_factors
        .iter()
        .map(|(tank_id, other_factors)| {
            (*tank_id, vehicle_factors.cosine_similarity(other_factors))
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
                        div.navbar-item {
                            div.buttons { (home_button()) }
                        }
                    }
                }
            }

            section.section."p-0"."m-5" {
                div.container {
                    div.card {
                        header.card-header {
                            p.card-header-title { "Похожая техника" }
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
                                                (tier_td(other_vehicle.tier))
                                                td.(if vehicle.type_ == other_vehicle.type_ { "has-background-success-light" } else { "" }) {
                                                    (format!("{:?}", other_vehicle.type_))
                                                }
                                                td {
                                                    a href=(uri!(get_vehicle(tank_id = tank_id))) {
                                                        span.icon-text.is-flex-wrap-nowrap {
                                                            (format!("{:+.4}", coefficient))
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

            section.section."p-0"."m-5" {
                div.container {
                    div.card {
                        header.card-header {
                            p.card-header-title { "Скрытые признаки" }
                        }
                        div.card-content {
                            div.table-container {
                                table.table.is-hoverable.is-striped.is-fullwidth {
                                    (status_thead(vehicle_factors.0.len()))
                                    tbody {
                                        (status_tr(tank_id, vehicle_factors, vehicle_factors.0.len()))
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
