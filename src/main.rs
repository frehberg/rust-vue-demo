use std::env;
use sscanf::sscanf;
use axum::{
    body::{boxed, Full},
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        TypedHeader,
    },
    http::{header, StatusCode, Uri},
    response::IntoResponse,
    response::Response,
    routing::get,
    Router,
};
use local_ip_address::local_ip;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tower_http::trace::{DefaultMakeSpan, TraceLayer};

use futures_util::stream::StreamExt;
use tokio_socketcan::{CANSocket, CANFrame};

use rust_embed::RustEmbed;

/// Diagrams
///
/// Build Process
#[cfg_attr(doc, aquamarine::aquamarine)]
/// ```mermaid
/// graph
///    s([Rust Source]) --> m[[Rust macro processor]]
///    v([Vue App]) --> b[[build-script invokes npm build]]
///    i([Rust intermediate code]) --> f([executable])
///    subgraph rustc[Rust Compiler]
///       b -. generate files .-> d([webui/dist])
///       d -. include .-> m
///       m --> i
///    end
/// ```
/// Client Server Communication
/// ```mermaid
/// graph LR
///      u[[WebUI]] --> s[[HTTP Service]]
///      s <-- read-write --> c[[CAN Device]]
///      u <-. websocket .-> s
///
///      subgraph browser[Browser]
///         u
///      end
///
///      subgraph rustc[Web Service]
///      s -. read .-> db([Embedded Assets webui/dist])
///      end
/// ```
mod diagrams {}

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
static CANDEV_KEY: &str = "CANDEV";
static CANDEV_DEFAULT: &str = "vcan0";

fn candev() -> String {
    return match env::var(CANDEV_KEY) {
        Ok(val) => val.to_string(),
        Err(_) => CANDEV_DEFAULT.to_string()
    };
}

async fn static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    match Assets::get(path) {
        Some(content) => {
            let body = boxed(Full::from(content.data));
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            Response::builder()
                .header(header::CONTENT_TYPE, mime.as_ref())
                .body(body)
                .unwrap()
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

            Response::builder()
                .header(header::CONTENT_TYPE, "text/html")
                .body(body)
                .unwrap()
        }
        None => not_found().await,
    }
}

async fn not_found() -> Response {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(boxed(Full::from("404")))
        .unwrap()
}

#[tokio::main]
async fn main() {
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

    const ANY_IP4: [u8; 4] = [0, 0, 0, 0];
    const LISTEN_PORT: u16 = 3000;
    let addr = SocketAddr::from((ANY_IP4, LISTEN_PORT));

    let primary_ip = local_ip().unwrap();
    println!("Reading/Writing can device {}", candev());
    println!("listening on http://{}:{}", primary_ip, LISTEN_PORT);
    println!("listening on http://127.0.0.1:{}", LISTEN_PORT);

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

fn create_txt(counter: &u32, msg: &str) -> Result<String, ()> {
    // Serialize data to a JSON string.
    let my_local_ip = local_ip().unwrap();
    let data = AppData {
        service_url: format!("http://{}:3000", my_local_ip),
        counter: counter.clone(),
        body: msg.to_string(),
    };

    if let Ok(txt) = serde_json::to_string(&data) {
        return Ok(txt);
    }
    return Err(());
}

fn parse_frame(t: String) -> Result<CANFrame, ()> {
    if let Ok(parsed) = sscanf!(&t, "{u32:x}#{str}") {
        let (id, hexdata) = parsed;
        if let Ok(data) = hex::decode(hexdata.as_bytes()) {
            if let Ok(frame) = CANFrame::new(id, &data, false, false) {
                return Ok(frame);
            }
        }
    }

    return Err(());
}

async fn handle_message(can_tx: &CANSocket, msg: Message) -> bool {
    match msg {
        Message::Text(t) => {
            println!("client sent: {:?}", t);
            if let Ok(frame) = parse_frame(t) {
                if let Ok(_) = can_tx.write_frame(frame).unwrap().await {
                    println!("write frame succeeded");
                    return true;
                }
            }
            println!("internal error");
            return false;
        }
        Message::Binary(_) => {
            println!("client sent binary data");
            return true;
        }
        Message::Ping(_) => {
            println!("socket ping");
            return true;
        }
        Message::Pong(_) => {
            println!("socket pong");
            return true;
        }
        Message::Close(_) => {
            println!("client disconnected");
            return false;
        }
    }
}

async fn handle_time_trigger(socket: &mut WebSocket, counter: &u32) -> bool {
    println!("timer trigger");
    if let Ok(txt) = create_txt(counter, "-") {
        if socket
            .send(Message::Text(txt))
            .await
            .is_err() {
            println!("client disconnected");
            return false;
        }
        return true;
    } else {
        println!("internal error");
        return false;
    }
}

async fn handle_can_frame(socket: &mut WebSocket, counter: &u32, frame: CANFrame) -> bool {
    let fmt = format!("{:x}#{:X?}", frame.id(), frame.data());
    println!("received can frame {}", fmt);
    if let Ok(txt) = create_txt(counter, &fmt) {
        if socket
            .send(Message::Text(txt))
            .await
            .is_err() {
            println!("client disconnected");
            return false;
        }
        return true;
    } else {
        println!("internal error");
        return false;
    }
}

async fn handle_socket_loop(mut socket: WebSocket, mut can_rx: CANSocket, can_tx: CANSocket) {
    let mut counter = 0;

    loop {
        tokio::select! {
            Some(msg)  = socket.recv() => {
                 if let Ok(msg) = msg {
                    if ! handle_message(&can_tx, msg).await {
                        return;
                    }
                 } else {
                     println!("client disconnected");
                     return;
                 }
            }
            Some(Ok(frame)) = can_rx.next() => {
                 if ! handle_can_frame(&mut socket, &counter, frame).await {
                     return;
                }
                counter += 1;
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                 if ! handle_time_trigger(&mut socket, &counter).await {
                    return;
                 }
                 counter += 1;
            }
        }
    }
}

async fn handle_socket(socket: WebSocket) {
    // open canbus and loop
    let can = candev();
    if let Ok(can_rx) = CANSocket::open(&can) {
        if let Ok(can_tx) = CANSocket::open(&can) {
            handle_socket_loop(socket, can_rx, can_tx).await;
        } else {
            println!("canbus device not found {}", &can);
        }
    } else {
        println!("canbus device not found {}", &can);
    }
}
