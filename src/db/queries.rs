use sqlx::Row;

use super::models::Identity;
use super::POOL;

pub async fn create_user(mxid: &str) -> sqlx::Result<bool> {
    sqlx::query("INSERT INTO users (mxid) VALUES ($1) ON CONFLICT DO NOTHING;")
        .bind(mxid)
        .execute(&*POOL)
        .await
        .map(|res| res.rows_affected() > 0)
}

pub async fn create_identity(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO identities (mxid, name) VALUES ($1, $2);")
        .bind(mxid)
        .bind(name)
        .execute(&*POOL)
        .await
        .map(|_| ())
}

pub async fn add_display_name(mxid: &str, name: &str, display_name: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE identities SET display_name = $3 WHERE mxid = $1 AND name = $2;")
        .bind(mxid)
        .bind(name)
        .bind(display_name)
        .execute(&*POOL)
        .await
        .map(|_| ())
}

pub async fn add_avatar(mxid: &str, name: &str, avatar: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE identities SET avatar = $3 WHERE mxid = $1 AND name = $2;")
        .bind(mxid)
        .bind(name)
        .bind(avatar)
        .execute(&*POOL)
        .await
        .map(|_| ())
}

pub async fn create_activator(mxid: &str, name: &str, activator: &str) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO activators (mxid, name, value) VALUES ($1, $2, $3);")
        .bind(mxid)
        .bind(name)
        .execute(&*POOL)
        .await
        .map(|_| ())
}

pub async fn get_idenity(mxid: &str, name: &str) -> sqlx::Result<Identity> {
    let mut identity: Identity = sqlx::query_as(
        "SELECT (mxid, name, display_name, avatar) FROM identities WHERE mxid = $1 AND name = $2;",
    )
    .bind(mxid)
    .bind(name)
    .fetch_one(&*POOL)
    .await?;
    identity.activators =
        sqlx::query("SELECT value FROM activators WHERE mxid = $1 AND name = $2;")
            .bind(mxid)
            .bind(name)
            .map(|row| row.get::<String, usize>(0))
            .fetch_all(&*POOL)
            .await?;
    Ok(identity)
}
