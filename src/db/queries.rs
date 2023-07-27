#![allow(dead_code)]

use anyhow::Context;

use super::models::{ActivatorInfo, Identity};
use super::POOL;

pub async fn read_msgs(room_id: &str, event_id: &str) -> sqlx::Result<bool> {
    let read = sqlx::query!(
        "SELECT true FROM read_msgs WHERE room_id = $1 AND event_id = $2",
        room_id,
        event_id
    )
    .fetch_optional(&*POOL)
    .await?
    .is_some();
    if !read {
        sqlx::query!(
            "INSERT INTO read_msgs(room_id, event_id) 
                     VALUES ($1, $2) ON CONFLICT (room_id) DO 
                     UPDATE SET event_id = $2 WHERE read_msgs.room_id = $1",
            room_id,
            event_id
        )
        .execute(&*POOL)
        .await?;
    }
    Ok(read)
}

pub async fn create_user(mxid: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "INSERT INTO users (mxid) VALUES ($1) ON CONFLICT DO NOTHING;",
        mxid
    )
    .execute(&*POOL)
    .await
    .map(|res| res.rows_affected() > 0)
}

pub async fn create_identity(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO identities (mxid, name) VALUES ($1, $2);",
        mxid,
        name
    )
    .execute(&*POOL)
    .await
    .map(|_| ())
}

pub async fn remove_identity(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM identities WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*POOL)
    .await?;
    sqlx::query!(
        "DELETE FROM activators WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*POOL)
    .await?;
    sqlx::query!(
        "UPDATE users SET current_ident = null WHERE mxid = $1 AND current_ident = $2;",
        mxid,
        name
    )
    .execute(&*POOL)
    .await
    .map(|_| ())
}

pub async fn add_display_name(mxid: &str, name: &str, display_name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET display_name = $3 WHERE mxid = $1 AND name = $2;",
        mxid,
        name,
        display_name
    )
    .execute(&*POOL)
    .await
    .map(|_| ())
}

pub async fn remove_display_name(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET display_name = null WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*POOL)
    .await
    .map(|_| ())
}

pub async fn add_avatar(mxid: &str, name: &str, avatar: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET avatar = $3 WHERE mxid = $1 AND name = $2;",
        mxid,
        name,
        avatar
    )
    .execute(&*POOL)
    .await
    .map(|_| ())
}

pub async fn remove_avatar(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET avatar = null WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*POOL)
    .await?;
    Ok(())
}

pub async fn get_activators(mxid: &str) -> sqlx::Result<Vec<ActivatorInfo>> {
    sqlx::query_as!(
        ActivatorInfo,
        "SELECT name, value FROM activators WHERE mxid = $1;",
        mxid
    )
    .fetch_all(&*POOL)
    .await
}

pub async fn get_name_for_activator(mxid: &str, activator: &str) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar!(
        "SELECT name FROM activators WHERE mxid = $1 AND value = $2",
        mxid,
        activator
    )
    .fetch_optional(&*POOL)
    .await
}

pub async fn add_activator(mxid: &str, name: &str, activator: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO activators (mxid, name, value) VALUES ($1, $2, $3);",
        mxid,
        name,
        activator
    )
    .execute(&*POOL)
    .await?;
    Ok(())
}

pub async fn remove_activator(mxid: &str, name: &str, activator: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM activators WHERE mxid = $1 AND name = $2 AND value = $3;",
        mxid,
        name,
        activator
    )
    .execute(&*POOL)
    .await?;
    Ok(())
}

pub async fn identity_exists(mxid: &str, name: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "SELECT 1 as x FROM identities WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .fetch_optional(&*POOL)
    .await
    .map(|res| res.is_some())
}

pub async fn get_identity(mxid: &str, name: &str) -> sqlx::Result<Identity> {
    sqlx::query_as!(
        Identity,
        r#"SELECT i.mxid, i.name, i.display_name, i.avatar,
             COALESCE(array_agg(a.value) FILTER (WHERE a.value IS NOT NULL), '{}') as "activators!" 
           FROM identities AS i LEFT JOIN activators AS a
           ON i.mxid = a.mxid AND i.name = a.name 
           WHERE i.mxid = $1 AND i.name = $2
           GROUP BY i.mxid, i.name;"#,
        mxid,
        name
    )
    .fetch_one(&*POOL)
    .await
}

pub async fn list_identities(mxid: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!("SELECT name FROM identities WHERE mxid = $1;", mxid)
        .fetch_all(&*POOL)
        .await
}

pub async fn set_current_identity(mxid: &str, name: Option<&str>) -> sqlx::Result<()> {
    sqlx::query!("UPDATE users SET current_ident = $2 WHERE mxid = $1;", mxid, name)
        .execute(&*POOL)
        .await?;
    Ok(())
}

pub async fn get_current_indentity(mxid: &str) -> anyhow::Result<Option<Identity>> {
    match sqlx::query_scalar!("SELECT current_ident FROM users WHERE mxid = $1;", mxid)
        .fetch_optional(&*POOL)
        .await
        .context("Error getting current_ident")?
    {
        Some(Some(name)) => get_identity(mxid, &name)
            .await
            .map(Some)
            .map_err(|e| e.into()),
        _ => Ok(None),
    }
}
