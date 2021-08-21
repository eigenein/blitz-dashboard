use std::net::IpAddr;
use std::str::FromStr;
use std::time::Duration as StdDuration;

use maud::PreEscaped;
use rocket::http::{Status, StatusClass};
use rocket::{routes, Request};

use routes::r#static;

use crate::opts::WebOpts;
use crate::wargaming::cache::account::info::AccountInfoCache;
use crate::wargaming::cache::account::tanks::AccountTanksCache;
use crate::wargaming::WargamingApi;

mod error;
mod fairings;
mod partials;
mod response;
mod result;
mod routes;

/// Run the web app.
pub async fn run(opts: WebOpts) -> crate::Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "web"));

    log::info!("Listening on {}:{}.", opts.host, opts.port);
    let api = WargamingApi::new(
        &opts.connections.application_id,
        StdDuration::from_millis(1500),
    )?;
    rocket::custom(to_config(&opts)?)
        .manage(AccountInfoCache::new(api.clone()))
        .manage(AccountTanksCache::new(api.clone()))
        .manage(api)
        .manage(crate::database::open(&opts.connections.database_uri).await?)
        .manage(TrackingCode::new(&opts))
        .mount("/", routes![r#static::get_site_manifest])
        .mount("/", routes![r#static::get_favicon])
        .mount("/", routes![r#static::get_favicon_16x16])
        .mount("/", routes![r#static::get_favicon_32x32])
        .mount("/", routes![r#static::get_android_chrome_192x192])
        .mount("/", routes![r#static::get_android_chrome_512x512])
        .mount("/", routes![r#static::get_apple_touch_icon])
        .mount("/", routes![r#static::get_player_js])
        .mount("/", routes![r#static::get_robots_txt])
        .mount("/", routes![r#static::get_theme_css])
        .mount("/", routes![routes::index::get])
        .mount("/", routes![routes::search::get])
        .mount("/", routes![routes::player::get])
        .mount("/", routes![routes::random::get])
        .mount("/", routes![routes::error::get_error])
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
            log::warn!("{} {}: {}", request.method(), request.uri(), status);
        }
        StatusClass::ServerError => {
            log::error!("{} {}: {}", request.method(), request.uri(), status);
        }
        _ => {}
    }
    rocket::response::status::Custom(status, ())
}

fn to_config(opts: &WebOpts) -> crate::Result<rocket::Config> {
    Ok(rocket::Config {
        address: IpAddr::from_str(&opts.host)?,
        port: opts.port,
        log_level: rocket::log::LogLevel::Off,
        ..Default::default()
    })
}

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
