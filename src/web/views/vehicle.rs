use itertools::Itertools;
use maud::{html, DOCTYPE};
use mongodb::bson::doc;
use poem::http::StatusCode;
use poem::i18n::Locale;
use poem::web::{Data, Html, Path};
use poem::{handler, IntoResponse, Response};

use crate::database::mongodb::traits::TypedDocument;
use crate::prelude::*;
use crate::web::partials::*;
use crate::web::tracking_code::TrackingCode;
use crate::{database, tankopedia, wargaming};

#[instrument(skip_all)]
#[handler]
pub async fn get(
    Path(tank_id): Path<wargaming::TankId>,
    tracking_code: Data<&TrackingCode>,
    locale: Locale,
    db: Data<&mongodb::Database>,
) -> Result<Response> {
    let model = match database::VehicleModel::collection(&db)
        .find_one(doc! { "_id": tank_id }, None)
        .await?
    {
        Some(model) => model,
        _ => {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };

    let vehicle = tankopedia::get_vehicle(tank_id);
    let similar_vehicles = model
        .similar
        .into_iter()
        .sorted_unstable_by(|vehicle_1, vehicle_2| {
            vehicle_2.similarity.total_cmp(&vehicle_1.similarity)
        })
        .collect_vec();
    let max_similarity = similar_vehicles
        .iter()
        .map(|vehicle| vehicle.similarity)
        .max_by(|similarity_1, similarity_2| similarity_1.total_cmp(similarity_2))
        .unwrap_or_default();

    let markup = html! {
        (DOCTYPE)
        html lang=(locale.text("html-lang")?) {
            head {
                (headers())
                title { (vehicle.name) }
            }
            body {
                (*tracking_code)

                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.navbar-brand {
                        (home_button(&locale)?)

                        div.navbar-item {
                            (vehicle_title(&vehicle, &locale)?)
                        }

                        div.navbar-item {
                            span.(SemaphoreClass::new(model.victory_ratio).threshold(0.5)) {
                                strong { (Float::from(100.0 * model.victory_ratio)) } "%"
                            }
                        }
                    }
                }

                section.section {
                    div.container {
                        div.box {
                            div.table-container {
                                table.table.is-hoverable.is-striped.is-fullwidth {
                                    tbody {
                                        @for similar_vehicle in similar_vehicles {
                                            tr {
                                                th style="width: 25rem" { (vehicle_title(&tankopedia::get_vehicle(similar_vehicle.tank_id), &locale)?) }
                                                td style="width: 1px" {
                                                    a.is-family-monospace href=(format!("/analytics/vehicles/{}", similar_vehicle.tank_id)) {
                                                        (Float::from(similar_vehicle.similarity).precision(6))
                                                    }
                                                }
                                                td style="vertical-align: middle" {
                                                    progress.progress.is-success
                                                        title=(similar_vehicle.similarity)
                                                        max=(max_similarity)
                                                        value=(similar_vehicle.similarity) {
                                                            (similar_vehicle.similarity)
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

                (footer(&locale)?)
            }
        }
    };
    Ok(Html(markup.into_string()).into_response())
}
