use std::net::IpAddr;
use std::str::FromStr;

use poem::listener::TcpListener;
use poem::middleware::{AddData, CatchPanic, Tracing};
use poem::{get, EndpointExt, Route, Server};
use views::r#static;

use crate::helpers::redis;
use crate::opts::WebOpts;
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::WargamingApi;
use crate::web::middleware::{ErrorMiddleware, SecurityHeaders};
use crate::web::tracking_code::TrackingCode;

mod middleware;
mod partials;
mod tracking_code;
mod views;

/// Run the web app.
pub async fn run(opts: WebOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "web"));
    info!(host = opts.host.as_str(), port = opts.port, "starting up…");

    let api = WargamingApi::new(
        &opts.connections.application_id,
        opts.connections.api_timeout,
        opts.connections.max_api_rps,
    )?;
    let mongodb = crate::database::mongodb::open(&opts.connections.internal.mongodb_uri).await?;
    let redis = redis::connect(
        &opts.connections.internal.redis_uri,
        opts.connections.internal.redis_pool_size,
    )
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
        .with(Tracing)
        .with(AddData::new(mongodb))
        .with(AddData::new(TrackingCode::new(&opts)?))
        .with(AddData::new(AccountInfoCache::new(api.clone(), redis.clone())))
        .with(AddData::new(AccountTanksCache::new(api.clone(), redis.clone())))
        .with(AddData::new(redis))
        .with(AddData::new(api))
        .with(CatchPanic::new())
        .with(ErrorMiddleware)
        .with(SecurityHeaders);
    Server::new(TcpListener::bind((IpAddr::from_str(&opts.host)?, opts.port)))
        .run(app)
        .await
        .map_err(Error::from)
}
