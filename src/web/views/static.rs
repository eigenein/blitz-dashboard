use poem::{handler, IntoResponse};

struct Static(&'static str, &'static [u8]);

impl IntoResponse for Static {
    #[inline]
    fn into_response(self) -> poem::Response {
        self.1
            .with_content_type(self.0)
            .with_header("Cache-Control", "public, max-age=31536000, immutable")
            .into_response()
    }
}

#[inline]
#[handler]
pub async fn get_site_manifest() -> impl IntoResponse {
    Static("application/json", include_bytes!("static/site.webmanifest"))
}

#[handler]
pub async fn get_favicon() -> impl IntoResponse {
    Static("image/vnd.microsoft.icon", include_bytes!("static/favicon.ico"))
}

#[inline]
#[handler]
pub async fn get_favicon_16x16() -> impl IntoResponse {
    Static("image/png", include_bytes!("static/favicon-16x16.png"))
}

#[inline]
#[handler]
pub async fn get_favicon_32x32() -> impl IntoResponse {
    Static("image/png", include_bytes!("static/favicon-32x32.png"))
}

#[inline]
#[handler]
pub async fn get_android_chrome_192x192() -> impl IntoResponse {
    Static("image/png", include_bytes!("static/android-chrome-192x192.png"))
}

#[inline]
#[handler]
pub async fn get_android_chrome_512x512() -> impl IntoResponse {
    Static("image/png", include_bytes!("static/android-chrome-512x512.png"))
}

#[inline]
#[handler]
pub async fn get_apple_touch_icon() -> impl IntoResponse {
    Static("image/png", include_bytes!("static/apple-touch-icon.png"))
}

#[inline]
#[handler]
pub async fn get_table_js() -> impl IntoResponse {
    Static("application/javascript", include_bytes!("static/table.js"))
}

#[inline]
#[handler]
pub async fn get_navbar_js() -> impl IntoResponse {
    Static("application/javascript", include_bytes!("static/navbar.js"))
}

#[inline]
#[handler]
pub async fn get_theme_css() -> impl IntoResponse {
    Static("text/css", include_bytes!("static/theme.css"))
}

#[inline]
#[handler]
pub async fn get_robots_txt() -> impl IntoResponse {
    Static("text/plain", include_bytes!("static/robots.txt"))
}

#[inline]
#[handler]
pub async fn get_cn_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/cn.svg"))
}

#[inline]
#[handler]
pub async fn get_de_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/de.svg"))
}

#[inline]
#[handler]
pub async fn get_eu_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/eu.svg"))
}

#[inline]
#[handler]
pub async fn get_fr_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/fr.svg"))
}

#[inline]
#[handler]
pub async fn get_gb_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/gb.svg"))
}

#[inline]
#[handler]
pub async fn get_jp_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/jp.svg"))
}

#[inline]
#[handler]
pub async fn get_su_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/su.svg"))
}

#[inline]
#[handler]
pub async fn get_us_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/us.svg"))
}

#[inline]
#[handler]
pub async fn get_xx_svg() -> impl IntoResponse {
    Static("image/svg+xml", include_bytes!("static/flags/xx.svg"))
}
