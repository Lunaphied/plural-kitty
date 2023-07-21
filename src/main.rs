mod bot;
mod config;
mod db;
mod late_init;
mod relay;

use anyhow::Context;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let bot_client = match bot::create_client().await {
        Ok(client) => client,
        Err(e) => {
            tracing::error!("{e:#}");
            std::process::exit(1);
        }
    };
    db::init().await.context("Error connecting to bot DB")?;
    tokio::spawn(async {
        if let Err(e) = bot::init(bot_client).await {
            tracing::error!("Bot error: {e:#}");
            std::process::exit(1);
        }
    });
    relay::init().await
}
