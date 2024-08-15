mod auth;
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

    // We use a private key to authenticate as a GitHub application
    // and derive installation tokens from it.
    // Use a central registry of cached installation tokens for efficiency.
    let auth = auth::Auth::new(&config)?;

    log::info!("Startup complete. Handling requests");

    // Notify systemd that we are ready to handle requests.
    // This allows us to use the `Type=notify` systemd service type.
    if let Err(e) = sd_notify::notify(true, &[sd_notify::NotifyState::Ready]) {
        log::info!("Failed to notify systemd about service startup: {e}");
    }

    // Pretend to use the `Auth` methods to prevent dead_code warnings.
    let _ = auth.app();
    auth.update_user("hnez", octocrab::models::InstallationId(0));
    let _ = auth.user("hnez");

    Ok(())
}
