use axum::{
    extract::{Path, State},
    headers::{authorization::Bearer, Authorization},
    http::{uri::Uri, Request, Response},
    routing::put,
    Router, TypedHeader,
};
use hyper::{client::HttpConnector, Body};
use std::net::SocketAddr;

type Client = hyper::client::Client<HttpConnector, Body>;
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let client = Client::new();

    let app = Router::new()
        .route(
            "/_matrix/client/r0/rooms/:room_id/send/:event_type/:txn_id",
            put(msg_send_handler).options(passthrough_handler),
        )
        .fallback(passthrough_handler)
        .with_state(client);

    let addr = SocketAddr::from(([127, 0, 0, 1], 4000));
    println!("reverse proxy listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn msg_send_handler(
    State(client): State<Client>,
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

    println!("GOT {room_id} {event_type} {txn_id} {auth:?}");
    let uri = format!("http://127.0.0.1:8008/_matrix/client/v3/rooms/{room_id}/state/m.room.member/@test:test.local");
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
    State(client): State<Client>,
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
