use anyhow::Context;
use axum::{
    extract::{Path, State},
    headers::{authorization::Bearer, Authorization},
    http::{uri::Uri, Request, Response},
    routing::put,
    Router, TypedHeader,
};
use hyper::{client::HttpConnector, Body, StatusCode};
use matrix_sdk::ruma::{
    api::client::state::{get_state_events_for_key, send_state_event},
    events::{room::member::RoomMemberEventContent, AnyStateEventContent, StateEventType},
    OwnedRoomId,
};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};
use tokio::sync::{Mutex, RwLock};

use crate::{config::CONFIG, db::queries};

#[derive(Debug, Clone)]
struct AppState {
    client: HttpClient,
    user_ids: Arc<RwLock<HashMap<String, String>>>,
    update_locks: Arc<RwLock<HashMap<String, Arc<Mutex<()>>>>>,
}

type HttpClient = hyper::client::Client<HttpConnector, Body>;

pub static STARTED: AtomicBool = AtomicBool::new(false);

#[tokio::main]
pub async fn init() -> anyhow::Result<()> {
    let client = HttpClient::new();
    let state = AppState {
        client,
        user_ids: Default::default(),
        update_locks: Default::default(),
    };

    let app = Router::new()
        .route(
            "/_matrix/client/:version/rooms/:room_id/send/:event_type/:txn_id",
            put(msg_send_handler).options(passthrough_handler),
        )
        .fallback(passthrough_handler)
        .with_state(state);

    println!("reverse proxy listening on {}", CONFIG.listen);
    STARTED.store(true, std::sync::atomic::Ordering::SeqCst);
    axum::Server::bind(&CONFIG.listen)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn update_indentity(
    AppState {
        client,
        user_ids,
        update_locks,
    }: &AppState,
    room_id: String,
    auth: Authorization<Bearer>,
) -> anyhow::Result<()> {
    let read_lock = user_ids.read().await;
    let user_id = match read_lock.get(auth.token()) {
        Some(user_id) => user_id.clone(),
        None => {
            drop(read_lock);
            let user_id = queries::get_synapse_user(auth.token()).await?;
            let mut write_lock = user_ids.write().await;
            write_lock.insert(auth.token().to_owned(), user_id.clone());
            user_id
        }
    };

    if let Some(member) = queries::get_current_fronter(&user_id)
        .await
        .context("Error getting user's current member")?
    {
        if queries::is_room_ignored(&user_id, &room_id).await? {
            tracing::debug!("Message in ignored room");
            return Ok(());
        }
        // ** This ensures multiple join evens aren't sent if the users sends a second message
        // before the join event is posted.
        let update_lock = {
            let read_lock = update_locks.read().await;
            match read_lock.get(&user_id) {
                Some(lock) => lock.to_owned(),
                None => {
                    drop(read_lock);
                    let mut write_lock = update_locks.write().await;
                    let update_lock = Arc::new(Mutex::new(()));
                    write_lock.insert(user_id.clone(), update_lock.clone());
                    update_lock
                }
            }
        };
        let _lock = update_lock.lock().await;
        // **
        let client = matrix_sdk::ruma::Client::builder()
            .homeserver_url(CONFIG.synapse.host.to_owned())
            .access_token(Some(auth.token().to_owned()))
            .http_client(client.to_owned())
            .await
            .context("Error building proxy matrix client")?;

        let room_id: OwnedRoomId = room_id
            .parse()
            .with_context(|| format!("room id {room_id} not valid room id"))?;
        let mut join_event: RoomMemberEventContent = client
            .send_request(get_state_events_for_key::v3::Request::new(
                room_id.clone(),
                StateEventType::RoomMember,
                user_id.to_owned(),
            ))
            .await
            .with_context(|| format!("Error getting join event for user {user_id}"))?
            .content
            .deserialize_as()
            .with_context(|| format!("Error deserializing join event for {user_id}"))?;

        let mut changed = false;

        match (member.display_name, &join_event.displayname) {
            (Some(ident_name), Some(curr_name)) if ident_name != *curr_name => {
                join_event.displayname = Some(ident_name);
                changed = true;
            }
            (Some(ident_name), None) => {
                join_event.displayname = Some(ident_name);
                changed = true;
            }
            _ => {}
        }
        match (member.avatar, &join_event.avatar_url) {
            (Some(ident_avatar), Some(curr_avatar)) if ident_avatar != *curr_avatar => {
                join_event.avatar_url = Some(ident_avatar.into());
                changed = true;
            }
            (Some(ident_avatar), None) => {
                join_event.avatar_url = Some(ident_avatar.into());
                changed = true;
            }
            _ => {}
        }

        if changed {
            client
                .send_request(
                    send_state_event::v3::Request::new(
                        room_id,
                        &user_id,
                        &AnyStateEventContent::from(join_event),
                    )
                    .with_context(|| format!("Error serializing join event for {user_id}"))?,
                )
                .await
                .with_context(|| format!("Error sending new join event for {user_id}"))?;
        }
    }

    Ok(())
}

async fn passthrough(
    client: &HttpClient,
    mut req: Request<Body>,
) -> anyhow::Result<Response<Body>> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);
    tracing::debug!("Pass through request to {} {path}", req.method());
    let uri = format!("{}{}", CONFIG.synapse.host, path_query);
    *req.uri_mut() = Uri::try_from(uri)?;
    let resp = client.request(req).await?;
    Ok(resp)
}

async fn msg_send_handler(
    State(state): State<AppState>,
    Path((_version, room_id, event_type, txn_id)): Path<(String, String, String, String)>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    req: Request<Body>,
) -> Response<Body> {
    tracing::info!(
        "Message event handler got {room_id} {event_type} {txn_id} {}",
        auth.token()
    );
    if let Err(e) = update_indentity(&state, room_id, auth).await {
        tracing::error!("Error handling message event: {e:#}");
    }
    match passthrough(&state.client, req).await {
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

async fn passthrough_handler(State(state): State<AppState>, req: Request<Body>) -> Response<Body> {
    match passthrough(&state.client, req).await {
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
