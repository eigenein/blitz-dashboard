use rocket::get;
use rocket::http::ContentType;

use crate::web::responders::Static;

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

#[get("/static/table.js")]
pub async fn get_table_js() -> Static {
    Static(ContentType::JavaScript, include_bytes!("static/table.js"))
}

#[get("/static/theme.css")]
pub async fn get_theme_css() -> Static {
    Static(ContentType::CSS, include_bytes!("static/theme.css"))
}

#[get("/robots.txt")]
pub async fn get_robots_txt() -> Static {
    Static(ContentType::Text, include_bytes!("static/robots.txt"))
}
