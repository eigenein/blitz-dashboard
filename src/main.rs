mod logging;
mod opts;
mod wargaming;
mod web;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = opts::parse();
    logging::init()?;
    web::run(&opts.host, opts.port, opts.application_id.clone()).await?;
    Ok(())
}
