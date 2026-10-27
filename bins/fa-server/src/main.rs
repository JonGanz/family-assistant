//! fa-server: the Family Assistant server and its operator CLI.

mod cli;
mod init;
mod mail;
mod ops;
mod serve;
mod wiring;

use anyhow::Result;
use clap::Parser;

use cli::{Cli, Command};

#[tokio::main]
async fn main() -> Result<()> {
    // Development convenience, shell-independent: fill unset variables from `.env.dev` in the
    // working directory, if there is one. Real environment variables always win.
    let _ = dotenvy::from_filename(".env.dev");
    let cli = Cli::parse();
    match cli.command {
        Command::GenOpenapi { out } => ops::gen_openapi(&out),
        Command::Init { secrets_dir } => init::run(secrets_dir.as_deref()),
        command => {
            let cfg = fa_config::ProcessConfig::load()?;
            serve::init_logging(&cfg);
            let store = fa_store::Store::connect(&cfg.database_url, 10).await?;
            match command {
                Command::Serve { mode } => serve::run(cfg, store, mode).await,
                Command::Migrate => {
                    store.migrate().await?;
                    println!("migrations applied");
                    Ok(())
                }
                Command::Config { action } => ops::config(&store, cli.household, action).await,
                Command::User { action } => ops::user(&store, cli.household, action).await,
                Command::Device { action } => {
                    let cfg = serve::layered_config(&cfg, &store).await;
                    ops::device(&store, &cfg, cli.household, action).await
                }
                Command::Pin { action } => ops::pin(&store, cli.household, action).await,
                Command::Calendar { action } => {
                    let cfg = serve::layered_config(&cfg, &store).await;
                    ops::calendar(&cfg, &store, cli.household, action).await
                }
                Command::Weather { action } => {
                    let cfg = serve::layered_config(&cfg, &store).await;
                    ops::weather(&cfg, &store, cli.household, action).await
                }
                Command::Notify { action } => {
                    let cfg = serve::layered_config(&cfg, &store).await;
                    ops::notify(&cfg, &store, cli.household, action).await
                }
                Command::Tasks { action } => ops::tasks(&store, cli.household, action).await,
                Command::Mail { action } => ops::mail(&cfg, &store, cli.household, action).await,
                Command::GenOpenapi { .. } | Command::Init { .. } => {
                    unreachable!("handled above")
                }
            }
        }
    }
}
