use rocket::response::content::Html;
use rocket::response::status::{BadRequest, NotFound};
use rocket::response::{Redirect, Responder};

use crate::web::responders::CachedHtml;

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum Response {
    BadRequest(BadRequest<()>),
    Html(Html<String>),
    CachedHtml(CachedHtml),
    NotFound(NotFound<()>),
    Redirect(Redirect),
}
