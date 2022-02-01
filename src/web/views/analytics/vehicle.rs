use maud::{html, DOCTYPE};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use rocket::State;
use tracing::instrument;

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

    let markup = html! {
        (DOCTYPE)
        html.has-navbar-fixed-top lang="en" {
            head {
                script defer src="https://cdnjs.cloudflare.com/ajax/libs/Chart.js/3.7.0/chart.min.js" integrity="sha512-TW5s0IT/IppJtu76UbysrBH9Hy/5X41OTAbQuffZFU6lQ1rdcLHzpU5BzVvr/YFykoiMYZVWlr/PX1mDcfM9Qg==" crossorigin="anonymous" referrerpolicy="no-referrer" {}
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

            (footer())
        }
    };

    let response = markup.into_string();
    redis.set_ex(&cache_key, &response, 60).await?;
    Ok(CustomResponse::Html(response))
}
