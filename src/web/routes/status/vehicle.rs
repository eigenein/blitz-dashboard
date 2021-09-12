use redis::aio::ConnectionManager as Redis;
use rocket::response::content::Html;
use rocket::State;

use crate::logging::clear_user;
use crate::web::TrackingCode;

#[rocket::get("/status/vehicle/<tank_id>")]
pub async fn get(
    tank_id: i32,
    tracking_code: &State<TrackingCode>,
    redis: &State<Redis>,
) -> crate::web::result::Result<Html<String>> {
    clear_user();

    Ok(Html(String::new()))
}
