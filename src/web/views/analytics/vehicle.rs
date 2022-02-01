use maud::{html, PreEscaped, DOCTYPE};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use rocket::State;
use tracing::instrument;

use crate::aggregator::persistence::retrieve_vehicle_chart;
use crate::helpers::sentry::clear_user;
use crate::tankopedia::get_vehicle;
use crate::wargaming::tank_id::TankId;
use crate::web::partials::{footer, headers, home_button, vehicle_title};
use crate::web::response::CustomResponse;
use crate::web::{DisableCaches, TrackingCode};

#[instrument(skip_all, name = "vehicle::get", fields(tank_id = tank_id))]
#[rocket::get("/analytics/vehicles/<tank_id>")]
pub async fn get(
    tank_id: TankId,
    tracking_code: &State<TrackingCode>,
    redis: &State<MultiplexedConnection>,
    disable_caches: &State<DisableCaches>,
) -> crate::web::result::Result<CustomResponse> {
    clear_user();

    let mut redis = MultiplexedConnection::clone(redis);
    let cache_key = format!("html::analytics::vehicles::{}", tank_id);
    if !disable_caches.0 {
        if let Some(cached_response) = redis.get(&cache_key).await? {
            return Ok(CustomResponse::Html(cached_response));
        }
    }

    let vehicle = get_vehicle(tank_id);
    let chart = retrieve_vehicle_chart(&mut redis, tank_id).await?;

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

            @if let Some(chart) = chart {
                section.section {
                    div.container {
                        div.card {
                            header.card-header {
                                p.card-header-title {
                                    span.icon-text {
                                        span.icon { i.fas.fa-percentage {} }
                                        span { "Процент побед" }
                                    }
                                }
                            }
                            div.card-content style="height: 75vh" {
                                canvas id="chart" {}
                                script
                                    src="https://cdnjs.cloudflare.com/ajax/libs/Chart.js/3.7.0/chart.min.js"
                                    integrity="sha512-TW5s0IT/IppJtu76UbysrBH9Hy/5X41OTAbQuffZFU6lQ1rdcLHzpU5BzVvr/YFykoiMYZVWlr/PX1mDcfM9Qg=="
                                    crossorigin="anonymous"
                                    referrerpolicy="no-referrer" {
                                }
                                script src="https://cdn.jsdelivr.net/npm/luxon@^2" {}
                                script src="https://cdn.jsdelivr.net/npm/chartjs-adapter-luxon@^1" {}
                                script {
                                    (PreEscaped(format!(r###"
                                        "use strict";
                                        
                                        const defaults = Chart.defaults;
    
                                        if (window.matchMedia("(prefers-color-scheme: dark)").matches) {{
                                            defaults.color = "#b5b5b5";
                                            defaults.scale.grid.color = "hsl(0, 0%, 14%)";
                                            defaults.elements.line.borderColor = "hsl(204, 71%, 39%)";
                                            defaults.elements.line.backgroundColor = "hsla(204, 71%, 39%, 25%)";
                                        }} else {{
                                            defaults.color = "#4a4a4a";
                                            defaults.scale.grid.color = "hsl(0, 0%, 96%)";
                                            defaults.elements.line.borderColor = "hsl(204, 86%, 53%)";
                                            defaults.elements.line.backgroundColor = "hsla(204, 86%, 53%, 25%)";
                                        }}
                                        
                                        defaults.borderColor = defaults.scale.grid.tickColor = "rgba(0, 0, 0, 0)";
                                        defaults.elements.point.backgroundColor = defaults.elements.line.backgroundColor;
                                        defaults.elements.point.borderColor = defaults.elements.line.borderColor;
                                        defaults.elements.point.radius = 1;
                                        defaults.font.family = 'BlinkMacSystemFont,-apple-system,"Segoe UI",Roboto,Oxygen,Ubuntu,Cantarell,"Fira Sans","Droid Sans","Helvetica Neue",Helvetica,Arial,sans-serif';
                                        defaults.font.size = 14;
                                        defaults.scale.ticks.autoSkipPadding = 15;
                                        defaults.scale.ticks.maxRotation = 0;
                                        defaults.plugins.legend.labels.boxWidth = defaults.font.size;
                                        // defaults.plugins.legend.position = "top";
                                        
                                        new Chart(document.getElementById("chart"), {});
                                    "###, chart)))
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
    Ok(CustomResponse::Html(response))
}
