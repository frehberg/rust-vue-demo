use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        TypedHeader,
    },
    body::{boxed, Full},
    http::{header, StatusCode, Uri},
    response::Response,
    response::IntoResponse,
    routing::{get},
    Router,
};
use std::{net::SocketAddr};
use tower_http::{
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};

use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "webui/dist/"]
struct Assets;

// DTO - Data Transfer Object
#[derive(Serialize, Deserialize, Debug)]
struct AppData {
    service_url: String,
    counter: u32,
    body: String,
}

static INDEX_HTML: &str = "index.html";

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    match Assets::get(path) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Response::builder().header(header::CONTENT_TYPE, mime.as_ref()).body(body).unwrap()
        }
        None => {
            if path.contains('.') {
                return not_found().await;
            }

            index_html().await
        }
    }
}

async fn index_html() -> Response {
    match Assets::get(INDEX_HTML) {
        Some(content) => {
            let body = boxed(Full::from(content.data));

            Response::builder().header(header::CONTENT_TYPE, "text/html").body(body).unwrap()
        }
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    Response::builder().status(StatusCode::NOT_FOUND).body(boxed(Full::from("404"))).unwrap()
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_websockets=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // build our application with some routes
    let app = Router::new()
        .fallback(static_handler)

        // routes are matched from bottom to top, so we have to put `nest` at the
        // top since it matches all routes
        .route("/ws", get(ws_handler))
        // logging so we can see whats going on
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );


    // run it with hyper
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
) -> impl IntoResponse {
    if let Some(TypedHeader(user_agent)) = user_agent {
        println!("`{}` connected", user_agent.as_str());
    }

    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    let mut counter = 0;
    loop {
        // Serialize data to a JSON string.
        let my_local_ip = local_ip().unwrap();
        counter += 1;
        let data =  AppData { service_url: format!("http://{}:3000", my_local_ip), counter: counter, body: "Hi".to_string()};

        if let Ok(txt) = serde_json::to_string(&data) {
            if socket
                .send(Message::Text(txt))
                .await
                .is_err()
            {
                println!("client disconnected");
                return;
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
