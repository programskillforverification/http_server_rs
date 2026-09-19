use http_server::server::Server;

fn main() -> anyhow::Result<()> {
    // Log level is set by RUST_LOG (e.g. RUST_LOG=debug); defaults to info
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    Server::new("127.0.0.1", 8080).run()?;
    Ok(())
}
