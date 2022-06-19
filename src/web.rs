use std::net::IpAddr;
use std::str::FromStr;

use maud::PreEscaped;
use rocket::http::{Status, StatusClass};
use rocket::{routes, Request};
use views::r#static;

use crate::helpers::redis;
use crate::opts::WebOpts;
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::WargamingApi;

mod error;
mod fairings;
mod partials;
mod response;
mod result;
mod views;

/// Run the web app.
pub async fn run(opts: WebOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "web"));
    info!(host = opts.host.as_str(), port = opts.port, "starting up…");

    let api = WargamingApi::new(
        &opts.connections.application_id,
        opts.connections.api_timeout,
        opts.connections.max_api_permits,
    )?;
    let mongodb = crate::database::mongodb::open(&opts.connections.internal.mongodb_uri).await?;
    let redis = redis::connect(
        &opts.connections.internal.redis_uri,
        opts.connections.internal.redis_pool_size,
    )
    .await?;
    let _ = rocket::custom(to_config(&opts)?)
        .manage(AccountInfoCache::new(api.clone(), redis.clone()))
        .manage(AccountTanksCache::new(api.clone(), redis.clone()))
        .manage(api)
        .manage(mongodb)
        .manage(TrackingCode::new(&opts))
        .manage(redis)
        .mount("/", routes![r#static::get_site_manifest])
        .mount("/", routes![r#static::get_favicon])
        .mount("/", routes![r#static::get_favicon_16x16])
        .mount("/", routes![r#static::get_favicon_32x32])
        .mount("/", routes![r#static::get_android_chrome_192x192])
        .mount("/", routes![r#static::get_android_chrome_512x512])
        .mount("/", routes![r#static::get_apple_touch_icon])
        .mount("/", routes![r#static::get_table_js])
        .mount("/", routes![r#static::get_robots_txt])
        .mount("/", routes![r#static::get_theme_css])
        .mount("/", routes![r#static::get_cn_svg])
        .mount("/", routes![r#static::get_de_svg])
        .mount("/", routes![r#static::get_eu_svg])
        .mount("/", routes![r#static::get_fr_svg])
        .mount("/", routes![r#static::get_gb_svg])
        .mount("/", routes![r#static::get_jp_svg])
        .mount("/", routes![r#static::get_su_svg])
        .mount("/", routes![r#static::get_us_svg])
        .mount("/", routes![r#static::get_xx_svg])
        .mount("/", routes![views::index::get])
        .mount("/", routes![views::search::get])
        .mount("/", routes![views::player::get])
        .mount("/", routes![views::error::get_error])
        .register("/", rocket::catchers![default_catcher])
        .attach(fairings::SecurityHeaders)
        .launch()
        .await?;
    Ok(())
}

#[rocket::catch(default)]
fn default_catcher(status: Status, request: &Request<'_>) -> rocket::response::status::Custom<()> {
    match status.class() {
        StatusClass::ClientError => {
            warn!(
                method = %request.method(),
                uri = %request.uri(),
                status = status.code,
                "client error {}",
                status.code,
            );
        }
        StatusClass::ServerError => {
            error!(
                method = %request.method(),
                uri = %request.uri(),
                status = status.code,
                "server error {}",
                status.code,
            );
        }
        _ => {}
    }
    rocket::response::status::Custom(status, ())
}

fn to_config(opts: &WebOpts) -> Result<rocket::Config> {
    Ok(rocket::Config {
        address: IpAddr::from_str(&opts.host)?,
        port: opts.port,
        log_level: rocket::log::LogLevel::Off,
        ..Default::default()
    })
}

#[must_use]
pub struct TrackingCode(PreEscaped<String>);

impl TrackingCode {
    fn new(opts: &WebOpts) -> Self {
        let mut extra_html_headers = Vec::new();
        if let Some(counter) = &opts.yandex_metrika {
            extra_html_headers.push(format!(
                r#"<!-- Yandex.Metrika counter --> <script async type="text/javascript"> (function(m,e,t,r,i,k,a){{m[i]=m[i]||function(){{(m[i].a=m[i].a||[]).push(arguments)}}; m[i].l=1*new Date();k=e.createElement(t),a=e.getElementsByTagName(t)[0],k.async=1,k.src=r,a.parentNode.insertBefore(k,a)}}) (window, document, "script", "https://mc.yandex.ru/metrika/tag.js", "ym"); ym({}, "init", {{ clickmap:true, trackLinks:true, accurateTrackBounce:true, trackHash:true }}); </script> <noscript><div><img src="https://mc.yandex.ru/watch/{}" style="position:absolute; left:-9999px;" alt=""/></div></noscript> <!-- /Yandex.Metrika counter -->"#,
                counter, counter,
            ));
        };
        if let Some(measurement_id) = &opts.gtag {
            extra_html_headers.push(format!(
                r#"<!-- Global site tag (gtag.js) - Google Analytics --> <script async src="https://www.googletagmanager.com/gtag/js?id=G-S1HXCH4JPZ"></script> <script>window.dataLayer = window.dataLayer || []; function gtag(){{dataLayer.push(arguments);}} gtag('js', new Date()); gtag('config', '{}'); </script>"#,
                measurement_id,
            ));
        };
        Self(PreEscaped(extra_html_headers.join("")))
    }
}
