use std::io::Cursor;

use maud::Markup;
use rocket::http::{ContentType, Status};
use rocket::response::content::RawHtml;
use rocket::response::{Redirect, Responder};
use rocket::{Request, Response};

#[allow(clippy::large_enum_variant, dead_code)]
pub enum CustomResponse {
    Html(String),
    CachedMarkup(&'static str, Markup),
    CachedHtml(&'static str, String),
    Redirect(Redirect),
    Static(ContentType, &'static [u8]),
    Status(Status),
}

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for CustomResponse {
    fn respond_to(self, request: &'r Request) -> Result<Response<'static>, Status> {
        match self {
            CustomResponse::Html(content) => RawHtml(content).respond_to(request),
            CustomResponse::Redirect(redirect) => redirect.respond_to(request),
            CustomResponse::CachedMarkup(cache_control, markup) => Response::build()
                .merge(RawHtml(markup.into_string()).respond_to(request)?)
                .raw_header("Cache-Control", cache_control)
                .ok(),
            CustomResponse::CachedHtml(cache_control, content) => Response::build()
                .merge(RawHtml(content).respond_to(request)?)
                .raw_header("Cache-Control", cache_control)
                .ok(),
            CustomResponse::Static(content_type, blob) => Response::build()
                .header(content_type)
                .sized_body(blob.len(), Cursor::new(blob))
                .raw_header("Cache-Control", "public, max-age=31536000, immutable")
                .ok(),
            CustomResponse::Status(status) => Response::build().status(status).ok(),
        }
    }
}
