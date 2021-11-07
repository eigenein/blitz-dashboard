use std::io::Cursor;

use rocket::http::{ContentType, Status};
use rocket::response::content::Html;
use rocket::response::Responder;
use rocket::{Request, Response};

pub struct Static(pub ContentType, pub &'static [u8]);

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Static {
    fn respond_to(self, _request: &'r Request) -> Result<Response<'static>, Status> {
        Response::build()
            .header(self.0)
            .sized_body(self.1.len(), Cursor::new(self.1))
            .raw_header("Cache-Control", "public, max-age=31536000, immutable")
            .ok()
    }
}

pub struct CachedHtml(pub &'static str, pub String);

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for CachedHtml {
    fn respond_to(self, request: &'r Request) -> Result<Response<'static>, Status> {
        Response::build()
            .merge(Html(self.1).respond_to(request)?)
            .raw_header("Cache-Control", self.0)
            .ok()
    }
}
