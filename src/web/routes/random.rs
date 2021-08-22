use rocket::response::Redirect;
use rocket::{uri, State};
use sqlx::PgPool;

use crate::database::retrieve_random_account_id;
use crate::web::routes::player::rocket_uri_macro_get as rocket_uri_macro_get_player;

/// Selects a "random" user and redirects the player view.
#[rocket::get("/random")]
pub async fn get(database: &State<PgPool>) -> crate::web::result::Result<Option<Redirect>> {
    match retrieve_random_account_id(database).await? {
        Some(account_id) => Ok(Some(Redirect::temporary(uri!(get_player(
            account_id = account_id,
            period = _,
        ))))),
        None => Ok(None),
    }
}
