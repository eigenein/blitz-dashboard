use rocket::response::content::Html;
use rocket::response::status::BadRequest;
use rocket::response::{Redirect, Responder};

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum Response {
    Html(Html<String>),
    Redirect(Redirect),
    BadRequest(BadRequest<()>),
}
