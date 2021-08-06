use rocket::response::content::Html;
use rocket::response::Redirect;
use rocket::Responder;

#[allow(clippy::large_enum_variant)]
#[derive(Responder)]
pub enum Response {
    Html(Html<String>),
    Redirect(Redirect),
}
