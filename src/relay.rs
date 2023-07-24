use anyhow::{bail, Context};
use axum::{
    extract::{Path, State},
    headers::{authorization::Bearer, Authorization},
    http::{uri::Uri, Request, Response},
    routing::put,
    Router, TypedHeader,
};
use hyper::{client::HttpConnector, Body, StatusCode};
use matrix_sdk::ruma::events::room::member::OriginalRoomMemberEvent;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::{collections::HashMap, str::from_utf8, sync::Arc};
use tokio::sync::RwLock;

use crate::{config::CONFIG, db::queries};

#[derive(Debug, Default, Clone)]
struct ProxyCache {
    user_ids: Arc<RwLock<HashMap<String, String>>>,
}

type Client = hyper::client::Client<HttpConnector, Body>;

pub async fn init() -> anyhow::Result<()> {
    let client = Client::new();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&CONFIG.synapse.db.db_uri())
        .await?;

    let app = Router::new()
        .route(
            "/_matrix/client/r0/rooms/:room_id/send/:event_type/:txn_id",
            put(msg_send_handler).options(passthrough_handler),
        )
        .fallback(passthrough_handler)
        .with_state((client, pool, ProxyCache::default()));

    println!("reverse proxy listening on {}", CONFIG.listen);
    axum::Server::bind(&CONFIG.listen)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn update_indentity(
    client: Client,
    pool: Pool<Postgres>,
    cache: ProxyCache,
    room_id: String,
    auth: Authorization<Bearer>,
) -> anyhow::Result<()> {
    let read_lock = cache.user_ids.read().await;
    let user_id = match read_lock.get(auth.token()) {
        Some(user_id) => user_id.clone(),
        None => {
            drop(read_lock);
            let user_id = sqlx::query("SELECT user_id FROM access_tokens WHERE token = $1")
                .bind(auth.token())
                .map(|row| row.get::<String, usize>(0))
                .fetch_one(&pool)
                .await
                .context("Error getting user from auth token")?;
            let mut write_lock = cache.user_ids.write().await;
            write_lock.insert(auth.token().to_owned(), user_id.clone());
            user_id
        }
    };

    let identity = queries::get_current_indentity(&user_id)
        .await
        .context("Error getting user's current identity")?;

    if let Some(identity) = identity {
        let event_api_url = format!(
            "{}/_matrix/client/v3/rooms/{room_id}/state/m.room.member/{user_id}",
            CONFIG.synapse.host
        );
        let get_join_event = Request::get(&event_api_url)
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", auth.token()))
            .body(Body::empty())
            .unwrap();
        let (info, body) = client
            .request(get_join_event)
            .await
            .context("Error in request to get event api")?
            .into_parts();
        let body = hyper::body::to_bytes(body)
            .await
            .context("Error getting event req body")?;
        if !info.status.is_success() {
            let body = from_utf8(&body).unwrap_or("[body not UTF8]");
            bail!("Error getting user's join event:\n\n{body}");
        }
        let mut join_event = serde_json::from_slice::<OriginalRoomMemberEvent>(&body)
            .context("Get event response not valid")?
            .content;

        let mut changed = false;

        match (identity.display_name, &join_event.displayname) {
            (Some(ident_name), Some(curr_name)) if ident_name != *curr_name => {
                join_event.displayname = Some(ident_name);
                changed = true;
            }
            _ => {}
        }
        match (identity.avatar, &join_event.avatar_url) {
            (Some(ident_avatar), Some(curr_avatar)) if ident_avatar != *curr_avatar => {
                join_event.avatar_url = Some(ident_avatar.into());
                changed = true;
            }
            _ => {}
        }

        if changed {
            let body = serde_json::to_vec(&join_event)?;
            let set_join_event_req = Request::put(event_api_url)
                .header("Accept", "application/json")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {}", auth.token()))
                .body(Body::from(body))?;
            client.request(set_join_event_req).await?;
        }
    }

    Ok(())
}

async fn passthrough(client: Client, mut req: Request<Body>) -> anyhow::Result<Response<Body>> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    tracing::debug!("Pass through request to {path}");
    let uri = format!("{}{}", CONFIG.synapse.host, path_query);
    *req.uri_mut() = Uri::try_from(uri)?;
    let resp = client.request(req).await?;
    Ok(resp)
}

async fn msg_send_handler(
    State((client, pool, cache)): State<(Client, Pool<Postgres>, ProxyCache)>,
    Path((room_id, event_type, txn_id)): Path<(String, String, String)>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    req: Request<Body>,
) -> Response<Body> {
    tracing::info!(
        "Message event handler got {room_id} {event_type} {txn_id} {}",
        auth.token()
    );
    if let Err(e) = update_indentity(client.clone(), pool, cache, room_id, auth).await {
        tracing::error!("Error handling message event: {e:#}");
    }
    match passthrough(client, req).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Error doing pass through to matrix server");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("{e:#}").into())
                .unwrap()
        }
    }
}

async fn passthrough_handler(
    State((client, _, _)): State<(Client, Pool<Postgres>, ProxyCache)>,
    req: Request<Body>,
) -> Response<Body> {
    match passthrough(client, req).await {
        Ok(resp) => resp,
        Err(e) => {
            tracing::error!("Error doing pass through to matrix server");
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(format!("{e:#}").into())
                .unwrap()
        }
    }
}
