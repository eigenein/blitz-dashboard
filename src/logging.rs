use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

pub fn init() -> anyhow::Result<()> {
    TermLogger::init(
        LevelFilter::Debug,
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Error)
            .set_location_level(LevelFilter::Off)
            .set_time_level(LevelFilter::Off)
            .add_filter_allow_str("blitz_dashboard")
            .build(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )?;
    Ok(())
}
