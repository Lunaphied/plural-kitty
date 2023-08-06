mod bot;
mod config;
mod db;
mod late_init;
mod proxy;

use std::sync::atomic::Ordering;

use anyhow::Context;

const ALLOWED_FAILURES: u32 = 10;

fn main() {
    tracing_subscriber::fmt::init();
    if let Err(e) = init() {
        tracing::error!("Error during initalization: {e:#}");
        std::process::exit(1);
    }
    // Start the bot
    std::thread::spawn(|| {
        run_daemon("bot", bot::init, || bot::STARTED.load(Ordering::SeqCst));
    });
    // Start the proxy
    run_daemon("proxy", proxy::init, || {
        proxy::STARTED.load(Ordering::SeqCst)
    });
}

#[tokio::main]
async fn init() -> anyhow::Result<()> {
    db::init().await.context("Error connecting to bot DB")?;
    Ok(())
}

fn run_daemon(name: &'static str, f: impl Fn() -> anyhow::Result<()>, started: impl Fn() -> bool) {
    let mut fails = 0;
    loop {
        if let Err(e) = f() {
            if started() {
                if fails <= ALLOWED_FAILURES {
                    tracing::error!("{name} failed <restarting>: {e:#}");
                    fails += 1;
                } else {
                    tracing::error!(
                        "{name} failed more than {ALLOWED_FAILURES} times <exiting>: {e:#}"
                    );
                    std::process::exit(1);
                }
            } else {
                tracing::error!("{name} failed to start <exiting>: {e:#}");
                std::process::exit(1);
            }
        }
    }
}
