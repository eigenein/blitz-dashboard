use rocket::response::content::Html;
use rocket::response::status::{BadRequest, NotFound};
use rocket::response::{Redirect, Responder};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum Response {
    BadRequest(BadRequest<()>),
    Html(Html<String>),
    NotFound(NotFound<()>),
    Redirect(Redirect),
}
