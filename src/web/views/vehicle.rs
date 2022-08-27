use maud::{html, DOCTYPE};
use poem::i18n::Locale;
use poem::web::{Data, Html, Path};
use poem::{handler, IntoResponse, Response};

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
) -> Result<Response> {
    let vehicle = tankopedia::get_vehicle(tank_id);
    let similar_vehicles = Vec::<(wargaming::TankId, f64)>::new();
    let max_similarity = 0.0;

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
                            strong.(SemaphoreClass::new(0.0).threshold(0.5)) {
                                (Float::from(0.0).precision(1)) "%"
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
                                        @for (tank_id, similarity) in similar_vehicles {
                                            tr {
                                                th style="width: 25rem" { (vehicle_title(&tankopedia::get_vehicle(tank_id), &locale)?) }
                                                td style="width: 1px" {
                                                    a.is-family-monospace href=(format!("/analytics/vehicles/{}", tank_id)) {
                                                        (Float::from(similarity).precision(6))
                                                    }
                                                }
                                                td style="vertical-align: middle" {
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

                (footer(&locale)?)
            }
        }
    };
    Ok(Html(markup.into_string()).into_response())
}
