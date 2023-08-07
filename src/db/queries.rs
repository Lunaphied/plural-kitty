use anyhow::Context;

use super::models::*;
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

pub async fn add_activator(mxid: &str, name: &str, activator: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET activators = array_append(activators, $3) WHERE mxid = $1 AND name = $2",
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
        "UPDATE identities SET activators = array_remove(activators, $3) WHERE mxid = $1 AND name = $2",
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
        "SELECT * FROM identities WHERE mxid = $1 AND name = $2",
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
    sqlx::query!(
        "UPDATE users SET current_ident = $2 WHERE mxid = $1;",
        mxid,
        name
    )
    .execute(&*POOL)
    .await?;
    Ok(())
}

pub async fn get_current_indentity(mxid: &str) -> anyhow::Result<Option<Identity>> {
    sqlx::query_as!(
        Identity,
        r#"
        SELECT
            i.mxid AS mxid,
            i.name AS name, 
            i.display_name AS display_name,
            i.avatar AS avatar,
            i.activators AS activators,
            i.track_account AS track_account
        FROM users AS u 
            JOIN identities AS i ON u.mxid = i.mxid AND u.current_ident = i.name
        WHERE u.mxid = $1
        "#,
        mxid
    )
    .fetch_optional(&*POOL)
    .await
    .context("Error getting current_ident")
}

pub async fn set_identity_from_activator(
    mxid: &str,
    activator: &str,
) -> sqlx::Result<Option<String>> {
    sqlx::query_scalar!(
        r#"
        UPDATE users
        SET current_ident = sub.name
        FROM (
            SELECT name
            FROM identities
            WHERE mxid = $1 AND $2 = ANY(activators)
        ) AS sub
        WHERE mxid = $1
        RETURNING current_ident AS "name!"
    "#,
        mxid,
        activator
    )
    .fetch_optional(&*POOL)
    .await
}
