use maud::{html, PreEscaped, DOCTYPE};
use poem::http::StatusCode;
use poem::i18n::Locale;
use poem::web::{Data, Html, Path};
use poem::{handler, IntoResponse, Response};

use crate::helpers::sentry::clear_user;
use crate::math::sigmoid;
use crate::prelude::*;
use crate::tankopedia::get_vehicle;
use crate::web::partials::*;
use crate::web::tracking_code::TrackingCode;

#[instrument(level = "info", skip_all)]
#[handler]
pub async fn get_regression(
    Path((realm, source_vehicle_id, target_vehicle_id)): Path<(
        wargaming::Realm,
        wargaming::TankId,
        wargaming::TankId,
    )>,
    locale: Locale,
    tracking_code: Data<&TrackingCode>,
    Data(trainer_client): Data<&crate::trainer::Client>,
) -> poem::Result<Response> {
    clear_user();

    let regression = match trainer_client
        .get_regression(realm, source_vehicle_id, target_vehicle_id)
        .await?
    {
        Some(regression) => regression,
        _ => {
            info!(?realm, source_vehicle_id, target_vehicle_id, "not found");
            return Ok(StatusCode::NOT_FOUND.into_response());
        }
    };
    let source_vehicle = get_vehicle(source_vehicle_id);
    let target_vehicle = get_vehicle(target_vehicle_id);

    let markup = html! {
        (DOCTYPE)
        html lang=(locale.text("html-lang")?) {
            head {
                (headers())
                title { (source_vehicle.name) " vs " (target_vehicle.name) " – " (locale.text("page-title-index")?) }
            }
            body {
                (*tracking_code)

                nav.navbar.has-shadow role="navigation" aria-label="main navigation" {
                    div.navbar-brand {
                        (home_button(&locale)?)
                    }
                }

                section.section {
                    div.box {
                        div id="regression-chart" {}

                        script src="https://cdn.jsdelivr.net/npm/apexcharts" {}
                        script {
                            (PreEscaped("
                                'use strict';
                                const mode = (window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches) ? 'dark' : 'light';
                                new ApexCharts(document.getElementById('regression-chart'), {
                                    chart: {
                                        width: '100%',
                                        height: 500,
                                        animations: {enabled: false},
                                        background: 'transparent',
                                        type: 'line',
                                    },
                                    fill: {type: 'solid'},
                                    markers: {size: [6, 0]},
                                    tooltip: {shared: false, intersect: true},
                                    theme: {mode: mode},
                                    legend: {show: false},
                            "))

                            (PreEscaped("xaxis: {type: 'numeric', tickAmount: 'dataPoints', min: 0, max: 100,"))
                            (PreEscaped("title: {text: '")) (source_vehicle.name) (PreEscaped("'},"))
                            (PreEscaped("},"))
                            (PreEscaped("yaxis: {min: 0, max: 100,"))
                            (PreEscaped("title: {text: '")) (target_vehicle.name) (PreEscaped("'},"))
                            (PreEscaped("},"))
                            (PreEscaped("series: ["))

                            (PreEscaped("{name: 'Target', type: 'scatter', data: ["))
                            @for (x, y) in regression.x.iter().copied().zip(regression.y.iter().copied()) {
                                @let x = 100.0 * sigmoid(x);
                                @let y = 100.0 * sigmoid(y);
                                (PreEscaped("{x:")) (format!("{:.2}", x)) (PreEscaped(",y:")) (format!("{:.2}", y)) (PreEscaped("},"))
                            }
                            (PreEscaped("]},"))

                            (PreEscaped("{name: 'Regression', type: 'line', data: ["))
                            @for i in 0..=20 {
                                @let x = i as f64 / 20.0;
                                @let y = 100.0 * sigmoid(regression.predict(x));
                                @let x = 100.0 * x;
                                (PreEscaped("{x:")) (format!("{:.2}", x)) (PreEscaped(",y:")) (format!("{:.2}", y)) (PreEscaped("},"))
                            }
                            (PreEscaped("]},"))

                            (PreEscaped("],
                                }).render();
                            "))
                        }
                    }
                }

                (footer(&locale)?)
            }
        }
    };

    Ok(Html(markup).into_response())
}
