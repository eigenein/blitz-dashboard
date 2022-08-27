use maud::{html, DOCTYPE};
use poem::i18n::Locale;
use poem::web::{Data, Html, Path};
use poem::{handler, IntoResponse, Response};
use reqwest::StatusCode;

use crate::prelude::*;
use crate::web::partials::*;
use crate::web::tracking_code::TrackingCode;
use crate::{tankopedia, wargaming};

#[instrument(skip_all)]
#[handler]
pub async fn get(
    Path(tank_id): Path<wargaming::TankId>,
    tracking_code: Data<&TrackingCode>,
    locale: Locale,
    Data(client): Data<&crate::trainer::client::Client>,
) -> Result<Response> {
    let vehicle = tankopedia::get_vehicle(tank_id);
    let vehicle_response = match client.get_vehicle(tank_id).await? {
        Some(vehicle_response) => vehicle_response,
        _ => {
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };
    let max_similarity = vehicle_response
        .similar_vehicles
        .iter()
        .filter(|(another_id, _)| *another_id != tank_id)
        .map(|(_, similarity)| similarity)
        .copied()
        .max_by(|lhs, rhs| lhs.total_cmp(rhs))
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
                            strong.(SemaphoreClass::new(vehicle_response.victory_ratio).threshold(0.5)) {
                                (Float::from(100.0 * vehicle_response.victory_ratio).precision(1)) "%"
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
                                        @for (another_id, similarity) in vehicle_response.similar_vehicles {
                                            tr {
                                                th style="width: 25rem" { (vehicle_title(&tankopedia::get_vehicle(another_id), &locale)?) }
                                                td style="width: 1px" {
                                                    a.is-family-monospace href=(format!("/analytics/vehicles/{}", another_id)) {
                                                        (Float::from(similarity).precision(6))
                                                    }
                                                }
                                                td style="vertical-align: middle" {
                                                    @if another_id != tank_id {
                                                        progress.progress.is-success
                                                            title=(similarity)
                                                            max=(max_similarity)
                                                            value=(similarity) {
                                                                (similarity)
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

                (footer(&locale)?)
            }
        }
    };
    Ok(Html(markup.into_string()).into_response())
}
