use simplelog::{ColorChoice, ConfigBuilder, LevelFilter, TermLogger, TerminalMode};

pub fn init(debug: bool) -> anyhow::Result<()> {
    TermLogger::init(
        if !debug {
            LevelFilter::Info
        } else {
            LevelFilter::Debug
        },
        ConfigBuilder::new()
            .set_target_level(LevelFilter::Off)
            .set_location_level(LevelFilter::Off)
            .set_time_level(LevelFilter::Off)
            .add_filter_allow_str("blitz_dashboard")
            .set_thread_level(LevelFilter::Off)
            .build(),
        TerminalMode::Stderr,
        ColorChoice::Auto,
    )?;
    Ok(())
}
