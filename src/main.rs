mod logging;
mod opts;
mod web;

#[async_std::main]
async fn main() -> tide::Result<()> {
    let opts = opts::parse();
    logging::init()?;
    web::run(&opts.host, opts.port).await?;
    Ok(())
}
