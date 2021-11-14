use std::io::Cursor;

use maud::Markup;
use rocket::http::{ContentType, Status};
use rocket::response::content::Html;
use rocket::response::status::{BadRequest, NotFound};
use rocket::response::{Redirect, Responder};
use rocket::{Request, Response};

#[allow(clippy::large_enum_variant)]
pub enum CustomResponse {
    BadRequest,
    Html(Markup),
    RawHtml(String),
    CachedHtml(&'static str, Markup),
    NotFound,
    Redirect(Redirect),
    Static(ContentType, &'static [u8]),
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for CustomResponse {
    fn respond_to(self, request: &'r Request) -> Result<Response<'static>, Status> {
        match self {
            CustomResponse::BadRequest => BadRequest::<()>(None).respond_to(request),
            CustomResponse::NotFound => NotFound(()).respond_to(request),
            CustomResponse::Html(markup) => Html(markup.into_string()).respond_to(request),
            CustomResponse::RawHtml(content) => Html(content).respond_to(request),
            CustomResponse::Redirect(redirect) => redirect.respond_to(request),
            CustomResponse::CachedHtml(cache_control, markup) => Response::build()
                .merge(Html(markup.into_string()).respond_to(request)?)
                .raw_header("Cache-Control", cache_control)
                .ok(),
            CustomResponse::Static(content_type, blob) => Response::build()
                .header(content_type)
                .sized_body(blob.len(), Cursor::new(blob))
                .raw_header("Cache-Control", "public, max-age=31536000, immutable")
                .ok(),
        }
    }
}
