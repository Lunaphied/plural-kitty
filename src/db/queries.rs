use anyhow::Context;
use sqlx::Row;

use super::models::*;
use super::{PK_POOL, SYNAPSE_POOL};

pub async fn get_synapse_user(access_token: &str) -> anyhow::Result<String> {
    sqlx::query("SELECT user_id FROM access_tokens WHERE token = $1")
        .bind(access_token)
        .map(|row| row.get::<String, usize>(0))
        .fetch_one(&*SYNAPSE_POOL)
        .await
        .context("Error getting user from auth token")
}

pub async fn get_synapse_profile(mxid: &str) -> anyhow::Result<ProfileInfo> {
    sqlx::query_as(
        r#"SELECT 
        COALESCE(displayname, '') AS displayname, COALESCE(avatar_url, '') AS avatar_url
        FROM profiles WHERE full_user_id = $1"#,
    )
    .bind(mxid)
    .fetch_one(&*SYNAPSE_POOL)
    .await
    .with_context(|| format!("Error getting display name and avatar for {mxid}"))
}

pub async fn read_msgs(room_id: &str, event_id: &str) -> sqlx::Result<bool> {
    let read = sqlx::query!(
        "SELECT true FROM read_msgs WHERE room_id = $1 AND event_id = $2",
        room_id,
        event_id
    )
    .fetch_optional(&*PK_POOL)
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
        .execute(&*PK_POOL)
        .await?;
    }
    Ok(read)
}

pub async fn create_user(mxid: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "INSERT INTO users (mxid) VALUES ($1) ON CONFLICT DO NOTHING;",
        mxid
    )
    .execute(&*PK_POOL)
    .await
    .map(|res| res.rows_affected() > 0)
}

pub async fn get_users() -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!("SELECT mxid FROM users")
        .fetch_all(&*PK_POOL)
        .await
}

pub async fn create_identity(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO identities (mxid, name) VALUES ($1, $2);",
        mxid,
        name
    )
    .execute(&*PK_POOL)
    .await
    .map(|_| ())
}

pub async fn remove_identity(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM identities WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*PK_POOL)
    .await?;
    sqlx::query!(
        "UPDATE users SET current_ident = null WHERE mxid = $1 AND current_ident = $2;",
        mxid,
        name
    )
    .execute(&*PK_POOL)
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
    .execute(&*PK_POOL)
    .await
    .map(|_| ())
}

pub async fn remove_display_name(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET display_name = null WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*PK_POOL)
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
    .execute(&*PK_POOL)
    .await
    .map(|_| ())
}

pub async fn remove_avatar(mxid: &str, name: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE identities SET avatar = null WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .execute(&*PK_POOL)
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
    .execute(&*PK_POOL)
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
    .execute(&*PK_POOL)
    .await?;
    Ok(())
}

pub async fn identity_exists(mxid: &str, name: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "SELECT 1 as x FROM identities WHERE mxid = $1 AND name = $2;",
        mxid,
        name
    )
    .fetch_optional(&*PK_POOL)
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
    .fetch_one(&*PK_POOL)
    .await
}

pub async fn list_identities(mxid: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!("SELECT name FROM identities WHERE mxid = $1;", mxid)
        .fetch_all(&*PK_POOL)
        .await
}

pub async fn set_current_identity(mxid: &str, name: Option<&str>) -> sqlx::Result<()> {
    sqlx::query!(
        "UPDATE users SET current_ident = $2 WHERE mxid = $1;",
        mxid,
        name
    )
    .execute(&*PK_POOL)
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
    .fetch_optional(&*PK_POOL)
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
    .fetch_optional(&*PK_POOL)
    .await
}

pub async fn update_tracking_ident(mxid: &str, profile: &ProfileInfo) -> sqlx::Result<()> {
    sqlx::query!(
        r#"
        UPDATE identities
        SET 
            display_name = $2,
            avatar = $3
        WHERE mxid = $1
        AND track_account = TRUE
    "#,
        mxid,
        profile.display_name,
        profile.avatar
    )
    .execute(&*PK_POOL)
    .await?;
    Ok(())
}

pub async fn toggle_tracking(mxid: &str, name: &str) -> sqlx::Result<bool> {
    sqlx::query_scalar!(
        "UPDATE identities SET track_account = NOT track_account
        WHERE mxid = $1 AND name = $2 RETURNING track_account",
        mxid,
        name
    )
    .fetch_one(&*PK_POOL)
    .await
}

pub async fn is_room_ignored(mxid: &str, room_id: &str) -> sqlx::Result<bool> {
    sqlx::query!(
        "SELECT NULL AS x FROM ignored_rooms WHERE mxid = $1 AND room_id = $2",
        mxid,
        room_id
    )
    .fetch_optional(&*PK_POOL)
    .await
    .map(|res| res.is_some())
}

pub async fn ignore_room(mxid: &str, room_id: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "INSERT INTO ignored_rooms (mxid, room_id) VALUES ($1, $2)",
        mxid,
        room_id
    )
    .execute(&*PK_POOL)
    .await?;
    Ok(())
}

pub async fn unignore_room(mxid: &str, room_id: &str) -> sqlx::Result<()> {
    sqlx::query!(
        "DELETE FROM ignored_rooms WHERE mxid = $1 AND room_id = $2",
        mxid,
        room_id
    )
    .execute(&*PK_POOL)
    .await?;
    Ok(())
}

pub async fn list_ignored(mxid: &str) -> sqlx::Result<Vec<String>> {
    sqlx::query_scalar!("SELECT room_id FROM ignored_rooms WHERE mxid = $1", mxid)
        .fetch_all(&*PK_POOL)
        .await
}

pub async fn room_alias(room_id: &str) -> sqlx::Result<String> {
    sqlx::query_scalar("SELECT room_alias FROM room_aliases WHERE room_id = $1")
        .bind(room_id)
        .fetch_one(&*SYNAPSE_POOL)
        .await
}
