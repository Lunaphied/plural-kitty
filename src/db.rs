use sqlx::postgres::PgPoolOptions;
use sqlx::Pool;
use sqlx::Postgres;

use crate::config::CONFIG;
use crate::late_init::LateInit;

pub mod models;
pub mod queries;

static POOL: LateInit<Pool<Postgres>> = LateInit::new();

pub async fn init() -> anyhow::Result<()> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&CONFIG.bot.db.db_uri())
        .await?;
    POOL.init(pool);
    Ok(())
}
