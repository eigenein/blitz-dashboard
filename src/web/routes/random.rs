use rocket::response::Redirect;
use rocket::State;
use sqlx::PgPool;

use crate::database::retrieve_random_account_id;
use crate::web::routes::player::get_account_url;

/// Selects a "random" user and redirects the player view.
#[rocket::get("/random")]
pub async fn get(database: &State<PgPool>) -> crate::web::result::Result<Option<Redirect>> {
    match retrieve_random_account_id(database).await? {
        Some(account_id) => Ok(Some(Redirect::temporary(get_account_url(account_id)))),
        None => Ok(None),
    }
}
