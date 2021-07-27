use std::io::Cursor;

use rocket::get;
use rocket::http::{ContentType, Status};
use rocket::response::Responder;
use rocket::{Request, Response};

#[get("/site.webmanifest")]
pub async fn get_site_manifest() -> Static {
    Static(ContentType::JSON, include_bytes!("static/site.webmanifest"))
}

#[get("/favicon.ico")]
pub async fn get_favicon() -> Static {
    Static(
        ContentType::new("image", "vnd.microsoft.icon"),
        include_bytes!("static/favicon.ico"),
    )
}

#[get("/favicon-16x16.png")]
pub async fn get_favicon_16x16() -> Static {
    Static(ContentType::PNG, include_bytes!("static/favicon-16x16.png"))
}

#[get("/favicon-32x32.png")]
pub async fn get_favicon_32x32() -> Static {
    Static(ContentType::PNG, include_bytes!("static/favicon-32x32.png"))
}

#[get("/android-chrome-192x192.png")]
pub async fn get_android_chrome_192x192() -> Static {
    Static(
        ContentType::PNG,
        include_bytes!("static/android-chrome-192x192.png"),
    )
}

#[get("/android-chrome-512x512.png")]
pub async fn get_android_chrome_512x512() -> Static {
    Static(
        ContentType::PNG,
        include_bytes!("static/android-chrome-512x512.png"),
    )
}

#[get("/apple-touch-icon.png")]
pub async fn get_apple_touch_icon() -> Static {
    Static(
        ContentType::PNG,
        include_bytes!("static/apple-touch-icon.png"),
    )
}

#[get("/static/player.js")]
pub async fn get_player_js() -> Static {
    Static(ContentType::JavaScript, include_bytes!("static/player.js"))
}

pub struct Static(ContentType, &'static [u8]);

#[rocket::async_trait]
impl<'r> Responder<'r, 'static> for Static {
    fn respond_to(self, _request: &'r Request) -> Result<Response<'static>, Status> {
        Response::build()
            .header(self.0)
            .sized_body(self.1.len(), Cursor::new(self.1))
            .raw_header("Cache-Control", "public, max-age=2592000, immutable")
            .ok()
    }
}
