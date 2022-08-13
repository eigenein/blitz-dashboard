use std::net::IpAddr;
use std::str::FromStr;
use std::time;

use poem::listener::TcpListener;
use poem::middleware::{CatchPanic, CookieJarManager, Tracing};
use poem::{get, EndpointExt, Route, Server};
use views::r#static;

use crate::helpers::redis;
use crate::opts::WebOpts;
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::WargamingApi;
use crate::web::middleware::{ErrorMiddleware, SecurityHeadersMiddleware, SentryMiddleware};
use crate::web::tracking_code::TrackingCode;

mod cookies;
mod i18n;
mod middleware;
mod partials;
mod tracking_code;
mod views;

/// Run the web app.
pub async fn run(opts: WebOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "web"));
    info!(host = opts.host.as_str(), port = opts.port, "starting up…");

    let connections = &opts.connections;
    let api = WargamingApi::new(
        &connections.application_id,
        connections.api_timeout,
        connections.max_api_rps,
    )?;
    let mongodb = crate::database::mongodb::open(&connections.internal.mongodb_uri).await?;
    let redis =
        redis::connect(&connections.internal.redis_uri, connections.internal.redis_pool_size)
            .await?;

    let app = Route::new()
        .at("/site.webmanifest", get(r#static::get_site_manifest))
        .at("/favicon.ico", get(r#static::get_favicon))
        .at("/favicon-16x16.png", get(r#static::get_favicon_16x16))
        .at("/favicon-32x32.png", get(r#static::get_favicon_32x32))
        .at("/android-chrome-192x192.png", get(r#static::get_android_chrome_192x192))
        .at("/android-chrome-512x512.png", get(r#static::get_android_chrome_512x512))
        .at("/apple-touch-icon.png", get(r#static::get_apple_touch_icon))
        .at("/static/table.js", get(r#static::get_table_js))
        .at("/static/theme.css", get(r#static::get_theme_css))
        .at("/robots.txt", get(r#static::get_robots_txt))
        .at("/static/flags/cn.svg", get(r#static::get_cn_svg))
        .at("/static/flags/de.svg", get(r#static::get_de_svg))
        .at("/static/flags/eu.svg", get(r#static::get_eu_svg))
        .at("/static/flags/fr.svg", get(r#static::get_fr_svg))
        .at("/static/flags/gb.svg", get(r#static::get_gb_svg))
        .at("/static/flags/jp.svg", get(r#static::get_jp_svg))
        .at("/static/flags/su.svg", get(r#static::get_su_svg))
        .at("/static/flags/us.svg", get(r#static::get_us_svg))
        .at("/static/flags/xx.svg", get(r#static::get_xx_svg))
        .at("/", get(views::index::get))
        .at("/search", get(views::search::get))
        .at("/:realm/:account_id", get(views::player::get))
        .at("/error", get(views::error::get_error))
        .at("/random", get(views::random::get_random))
        .at("/sitemaps/:realm/sitemap.txt", get(views::sitemaps::get_sitemap))
        .at("/analytics/vehicles/:vehicle_id", get(views::gone::get))
        .data(mongodb)
        .data(i18n::build_resources()?)
        .data(TrackingCode::new(&opts)?)
        .data(AccountInfoCache::new(api.clone(), redis.clone()))
        .data(AccountTanksCache::new(api.clone(), redis.clone()))
        .data(redis)
        .data(api)
        .with(Tracing)
        .with(CatchPanic::new())
        .with(ErrorMiddleware)
        .with(SecurityHeadersMiddleware)
        .with(SentryMiddleware)
        .with(CookieJarManager::new());

    Server::new(TcpListener::bind((IpAddr::from_str(&opts.host)?, opts.port)))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
            },
            Some(time::Duration::from_secs(3)),
        )
        .await
        .map_err(Error::from)
}
