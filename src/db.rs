use anyhow::Context;
use sqlx::postgres::PgPoolOptions;
use sqlx::Pool;
use sqlx::Postgres;

use crate::config::CONFIG;
use crate::late_init::LateInit;

pub mod models;
pub mod queries;

static PK_POOL: LateInit<Pool<Postgres>> = LateInit::new();
static SYNAPSE_POOL: LateInit<Pool<Postgres>> = LateInit::new();

pub async fn init() -> anyhow::Result<()> {
    let db_opts = CONFIG.bot.db.db_con_opts().await?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_opts.clone())
        .await
        .context(format!(
            "Error connection to plural kitty DB at `{db_opts:?}`"
        ))?;
    sqlx::migrate!().run(&pool).await?;
    PK_POOL.init(pool);
    let db_opts = CONFIG.synapse.db.db_con_opts().await?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(db_opts.clone())
        .await
        .context(format!("Error connection to synapse DB at `{db_opts:?}`"))?;
    SYNAPSE_POOL.init(pool);
    Ok(())
}
