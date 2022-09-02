use std::net::IpAddr;
use std::str::FromStr;
use std::time;

use poem::listener::TcpListener;
use poem::middleware::{CatchPanic, CookieJarManager, Tracing};
use poem::{get, Endpoint, EndpointExt, Route, Server};
use views::r#static;

use crate::helpers::redis;
use crate::opts::WebOpts;
use crate::prelude::*;
use crate::wargaming::cache::account::{AccountInfoCache, AccountTanksCache};
use crate::wargaming::WargamingApi;
use crate::web::middleware::{ErrorMiddleware, SecurityHeadersMiddleware, SentryMiddleware};
use crate::web::tracking_code::TrackingCode;
use crate::web::views::player::Testers;

mod cookies;
mod i18n;
pub mod middleware;
mod partials;

#[cfg(test)]
mod test;

mod tracking_code;
mod views;

/// Run the web app.
pub async fn run(opts: WebOpts) -> Result {
    sentry::configure_scope(|scope| scope.set_tag("app", "web"));
    info!(host = opts.host.as_str(), port = opts.port, trainer_testers = ?opts.trainer_testers, "starting up…");

    let app_data = AppData::initialize_from_opts(&opts).await?;
    let app = create_app(app_data).await?;
    Server::new(TcpListener::bind((IpAddr::from_str(&opts.host)?, opts.port)))
        .run_with_graceful_shutdown(
            app,
            async move {
                let _ = tokio::signal::ctrl_c().await;
            },
            Some(time::Duration::from_secs(3)),
        )
        .await?;

    Ok(())
}

struct AppData {
    api: WargamingApi,
    mongodb: mongodb::Database,
    redis: fred::pool::RedisPool,
    trainer_client: crate::trainer::client::Client,
    tracking_code: TrackingCode,
    testers: Testers,
}

impl AppData {
    async fn initialize_from_opts(opts: &WebOpts) -> Result<Self> {
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
        let tracking_code = TrackingCode::new(opts)?;
        let trainer_client = crate::trainer::client::Client::new(&opts.trainer_base_url)?;
        let testers = Testers {
            trainer_testers: opts.trainer_testers.iter().copied().collect(),
        };

        Ok(Self {
            api,
            mongodb,
            redis,
            tracking_code,
            trainer_client,
            testers,
        })
    }
}

#[instrument(skip_all)]
async fn create_app(data: AppData) -> Result<impl Endpoint> {
    let app = create_standalone_app()
        .await?
        .data(data.mongodb)
        .data(data.tracking_code)
        .data(AccountInfoCache::new(data.api.clone(), data.redis.clone()))
        .data(AccountTanksCache::new(data.api.clone(), data.redis.clone()))
        .data(data.redis)
        .data(data.api)
        .data(data.trainer_client)
        .data(data.testers);
    Ok(app)
}

#[instrument(skip_all)]
async fn create_standalone_app() -> Result<impl Endpoint> {
    let app = Route::new()
        .at("/site.webmanifest", get(r#static::get_site_manifest))
        .at("/favicon.ico", get(r#static::get_favicon))
        .at("/favicon-16x16.png", get(r#static::get_favicon_16x16))
        .at("/favicon-32x32.png", get(r#static::get_favicon_32x32))
        .at("/android-chrome-192x192.png", get(r#static::get_android_chrome_192x192))
        .at("/android-chrome-512x512.png", get(r#static::get_android_chrome_512x512))
        .at("/apple-touch-icon.png", get(r#static::get_apple_touch_icon))
        .at("/static/table.js", get(r#static::get_table_js))
        .at("/static/navbar.js", get(r#static::get_navbar_js))
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
        .at("/:realm/:account_id", get(views::player::get).post(views::player::post))
        .at("/error", get(views::error::get_error))
        .at("/random", get(views::random::get_random))
        .at("/sitemaps/:realm/sitemap.txt", get(views::sitemaps::get_sitemap))
        .data(i18n::build_resources()?)
        .with(Tracing)
        .with(CatchPanic::new())
        .with(ErrorMiddleware)
        .with(SecurityHeadersMiddleware)
        .with(SentryMiddleware)
        .with(CookieJarManager::new());
    Ok(app)
}
