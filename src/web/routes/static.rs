use rocket::get;
use rocket::http::ContentType;

use crate::web::response::CustomResponse;

#[get("/site.webmanifest")]
pub async fn get_site_manifest() -> CustomResponse {
    CustomResponse::Static(ContentType::JSON, include_bytes!("static/site.webmanifest"))
}

#[get("/favicon.ico")]
pub async fn get_favicon() -> CustomResponse {
    CustomResponse::Static(
        ContentType::new("image", "vnd.microsoft.icon"),
        include_bytes!("static/favicon.ico"),
    )
}

#[get("/favicon-16x16.png")]
pub async fn get_favicon_16x16() -> CustomResponse {
    CustomResponse::Static(ContentType::PNG, include_bytes!("static/favicon-16x16.png"))
}

#[get("/favicon-32x32.png")]
pub async fn get_favicon_32x32() -> CustomResponse {
    CustomResponse::Static(ContentType::PNG, include_bytes!("static/favicon-32x32.png"))
}

#[get("/android-chrome-192x192.png")]
pub async fn get_android_chrome_192x192() -> CustomResponse {
    CustomResponse::Static(
        ContentType::PNG,
        include_bytes!("static/android-chrome-192x192.png"),
    )
}

#[get("/android-chrome-512x512.png")]
pub async fn get_android_chrome_512x512() -> CustomResponse {
    CustomResponse::Static(
        ContentType::PNG,
        include_bytes!("static/android-chrome-512x512.png"),
    )
}

#[get("/apple-touch-icon.png")]
pub async fn get_apple_touch_icon() -> CustomResponse {
    CustomResponse::Static(
        ContentType::PNG,
        include_bytes!("static/apple-touch-icon.png"),
    )
}

#[get("/static/table.js")]
pub async fn get_table_js() -> CustomResponse {
    CustomResponse::Static(ContentType::JavaScript, include_bytes!("static/table.js"))
}

#[get("/static/theme.css")]
pub async fn get_theme_css() -> CustomResponse {
    CustomResponse::Static(ContentType::CSS, include_bytes!("static/theme.css"))
}

#[get("/robots.txt")]
pub async fn get_robots_txt() -> CustomResponse {
    CustomResponse::Static(ContentType::Text, include_bytes!("static/robots.txt"))
}

#[get("/static/flags/cn.svg")]
pub async fn get_cn_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/cn.svg"))
}

#[get("/static/flags/de.svg")]
pub async fn get_de_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/de.svg"))
}

#[get("/static/flags/eu.svg")]
pub async fn get_eu_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/eu.svg"))
}

#[get("/static/flags/fr.svg")]
pub async fn get_fr_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/fr.svg"))
}

#[get("/static/flags/gb.svg")]
pub async fn get_gb_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/gb.svg"))
}

#[get("/static/flags/jp.svg")]
pub async fn get_jp_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/jp.svg"))
}

#[get("/static/flags/ru.svg")]
pub async fn get_ru_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/ru.svg"))
}

#[get("/static/flags/us.svg")]
pub async fn get_us_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/us.svg"))
}

#[get("/static/flags/xx.svg")]
pub async fn get_xx_svg() -> CustomResponse {
    CustomResponse::Static(ContentType::SVG, include_bytes!("static/flags/xx.svg"))
}
