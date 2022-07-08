use poem::{handler, IntoResponse};

const CACHE_CONTROL: &str = "public, max-age=31536000, immutable";

#[handler]
pub async fn get_site_manifest() -> impl IntoResponse {
    include_bytes!("static/site.webmanifest")
        .with_content_type("application/json")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_favicon() -> impl IntoResponse {
    include_bytes!("static/favicon.ico")
        .with_content_type("image/vnd.microsoft.icon")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_favicon_16x16() -> impl IntoResponse {
    include_bytes!("static/favicon-16x16.png")
        .with_content_type("image/png")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_favicon_32x32() -> impl IntoResponse {
    include_bytes!("static/favicon-32x32.png")
        .with_content_type("image/png")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_android_chrome_192x192() -> impl IntoResponse {
    include_bytes!("static/android-chrome-192x192.png")
        .with_content_type("image/png")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_android_chrome_512x512() -> impl IntoResponse {
    include_bytes!("static/android-chrome-512x512.png")
        .with_content_type("image/png")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_apple_touch_icon() -> impl IntoResponse {
    include_bytes!("static/apple-touch-icon.png")
        .with_content_type("image/png")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_table_js() -> impl IntoResponse {
    include_bytes!("static/table.js")
        .with_content_type("application/javascript")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_theme_css() -> impl IntoResponse {
    include_bytes!("static/theme.css")
        .with_content_type("text/css")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_robots_txt() -> impl IntoResponse {
    include_bytes!("static/robots.txt")
        .with_content_type("text/plain")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_cn_svg() -> impl IntoResponse {
    include_bytes!("static/flags/cn.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_de_svg() -> impl IntoResponse {
    include_bytes!("static/flags/de.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_eu_svg() -> impl IntoResponse {
    include_bytes!("static/flags/eu.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_fr_svg() -> impl IntoResponse {
    include_bytes!("static/flags/fr.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_gb_svg() -> impl IntoResponse {
    include_bytes!("static/flags/gb.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_jp_svg() -> impl IntoResponse {
    include_bytes!("static/flags/jp.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_su_svg() -> impl IntoResponse {
    include_bytes!("static/flags/su.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_us_svg() -> impl IntoResponse {
    include_bytes!("static/flags/us.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}

#[handler]
pub async fn get_xx_svg() -> impl IntoResponse {
    include_bytes!("static/flags/xx.svg")
        .with_content_type("image/svg+xml")
        .with_header("Cache-Control", CACHE_CONTROL)
}
