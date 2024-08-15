mod config;
mod machines;

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    let config_path = {
        let mut args: Vec<String> = std::env::args().collect();

        match args.len() {
            1 => "config.yaml".to_owned(),
            2 => args.remove(1),
            _ => anyhow::bail!("Usage: {} [CONFIG]", args[0]),
        }
    };

    // Read the config file.
    // The file will be re-read if it changed on disk at many points in the program,
    // allowing changes to be made while jobs are being executed.
    let config = config::Config::new(config_path)?;

    // Pretend to use the `Config` methods to prevent dead_code warnings.
    let _cfg = config.get();

    Ok(())
}
