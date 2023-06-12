use axum::{
    extract::{Path, State},
    headers::{authorization::Bearer, Authorization},
    http::{uri::Uri, Request, Response},
    routing::put,
    Router, TypedHeader,
};
use hyper::{client::HttpConnector, Body};
use sqlx::{postgres::PgPoolOptions, Pool, Postgres, Row};
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;

#[derive(Debug, Default, Clone)]
struct ProxyCache {
    user_ids: Arc<RwLock<HashMap<String, String>>>,
}

type Client = hyper::client::Client<HttpConnector, Body>;
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let client = Client::new();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://synapse:beepboop@localhost/synapse")
        .await
        .unwrap();

    let app = Router::new()
        .route(
            "/_matrix/client/r0/rooms/:room_id/send/:event_type/:txn_id",
            put(msg_send_handler).options(passthrough_handler),
        )
        .fallback(passthrough_handler)
        .with_state((client, pool, ProxyCache::default()));

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("reverse proxy listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn msg_send_handler(
    State((client, pool, cache)): State<(Client, Pool<Postgres>, ProxyCache)>,
    Path((room_id, event_type, txn_id)): Path<(String, String, String)>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    mut req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    println!("GOT {room_id} {event_type} {txn_id} {}", auth.token());
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
                .unwrap();
            let mut write_lock = cache.user_ids.write().await;
            write_lock.insert(auth.token().to_owned(), user_id.clone());
            user_id
        }
    };

    let uri = format!(
        "http://127.0.0.1:8008/_matrix/client/v3/rooms/{room_id}/state/m.room.member/{user_id}"
    );
    let body = r#"{ "membership": "join", "displayname": "kittycat"}"#;
    let profile_req = Request::put(uri)
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", auth.token()))
        .body(Body::from(body))
        .unwrap();
    if let Err(e) = client.request(profile_req).await {
        eprintln!("BAD {e}");
    }

    let uri = format!("http://127.0.0.1:8008{}", path_query);
    *req.uri_mut() = Uri::try_from(uri).unwrap();
    client.request(req).await.unwrap()
}

async fn passthrough_handler(
    State((client, _, _)): State<(Client, Pool<Postgres>, ProxyCache)>,
    mut req: Request<Body>,
) -> Response<Body> {
    let path = req.uri().path();
    let path_query = req
        .uri()
        .path_and_query()
        .map(|v| v.as_str())
        .unwrap_or(path);

    println!("BEEEEP");

    let uri = format!("http://127.0.0.1:8008{}", path_query);

    *req.uri_mut() = Uri::try_from(uri).unwrap();

    client.request(req).await.unwrap()
}
