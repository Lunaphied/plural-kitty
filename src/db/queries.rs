#![allow(dead_code)]

use anyhow::Context;
use sqlx::Row;

use super::models::ActivatorInfo;
use super::models::Identity;
use super::POOL;

pub async fn read(room_id: &str, event_id: &str) -> sqlx::Result<bool> {
    let read = sqlx::query("SELECT true FROM read_msgs WHERE room_id = $1 AND event_id = $2")
        .bind(room_id)
        .bind(event_id)
        .fetch_optional(&*POOL)
        .await?
        .is_some();
    if !read {
        sqlx::query("INSERT INTO read_msgs(room_id, event_id) 
                     VALUES ($1, $2) ON CONFLICT (room_id) DO 
                     UPDATE SET event_id = $2 WHERE read_msgs.room_id = $1")
            .bind(room_id)
            .bind(event_id)
            .execute(&*POOL).await?;
    }
    Ok(read)
}

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

pub async fn get_activators(mxid: &str) -> sqlx::Result<Vec<ActivatorInfo>> {
    sqlx::query_as("SELECT * FROM activators WHERE mxid = $1;")
        .bind(mxid)
        .fetch_all(&*POOL)
        .await
}

pub async fn get_name_for_activator(mxid: &str, activator: &str) -> sqlx::Result<Option<String>> {
    sqlx::query("SELECT name FROM activators WHERE mxid = $1 AND value = $2")
        .bind(mxid)
        .bind(activator)
        .map(|row| row.get::<String, usize>(0))
        .fetch_optional(&*POOL)
        .await
}

pub async fn add_activator(mxid: &str, name: &str, activator: &str) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO activators (mxid, name, value) VALUES ($1, $2, $3);")
        .bind(mxid)
        .bind(name)
        .bind(activator)
        .execute(&*POOL)
        .await
        .map(|_| ())
}

pub async fn create_activator(mxid: &str, name: &str, activator: &str) -> sqlx::Result<()> {
    sqlx::query("INSERT INTO activators (mxid, name, value) VALUES ($1, $2, $3);")
        .bind(mxid)
        .bind(name)
        .bind(activator)
        .execute(&*POOL)
        .await
        .map(|_| ())
}

pub async fn get_identity(mxid: &str, name: &str) -> anyhow::Result<Identity> {
    let mut identity: Identity =
        sqlx::query_as("SELECT * FROM identities WHERE mxid = $1 AND name = $2;")
            .bind(mxid)
            .bind(name)
            .fetch_one(&*POOL)
            .await
            .context("Error getting identities")?;
    identity.activators =
        sqlx::query("SELECT value FROM activators WHERE mxid = $1 AND name = $2;")
            .bind(mxid)
            .bind(name)
            .map(|row| row.get::<String, usize>(0))
            .fetch_all(&*POOL)
            .await
            .context("Error getting activators")?;
    Ok(identity)
}

pub async fn list_identities(mxid: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query("SELECT name FROM identities WHERE mxid = $1;")
        .bind(mxid)
        .map(|row| row.get::<String, usize>(0))
        .fetch_all(&*POOL)
        .await
}

pub async fn set_current_identity(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query("UPDATE users SET current_ident = $2 WHERE mxid = $1;")
        .bind(mxid)
        .bind(name)
        .execute(&*POOL)
        .await
        .map(|_| ())
}
