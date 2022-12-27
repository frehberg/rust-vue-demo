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
use tokio_socketcan::{CANSocket, CANFrame, Error};

use rust_embed::RustEmbed;
use crate::State::ClientWsDisconnected;


#[cfg_attr(doc, aquamarine::aquamarine)]
/// WebBased Client Server Communication
///
/// Author: Frank Rehberger
///
/// Repo: https://github.com/frehberg/rust-vue-demo
///
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
mod slide1 {}

#[cfg_attr(doc, aquamarine::aquamarine)]
/// Components
///
/// Repo: https://github.com/frehberg/rust-vue-demo
///
/// MessageMonitor displaying  CANBUS frames asynchronously in WebUI
/// * Axum: Tokio web application framework
/// * VueJS: JavaScript framework for building user interfaces
///
/// ```mermaid
/// graph BT
///   SocketCan --> Rust((Rust Backend App))
///   RustDoc[[RustDoc]] --> Rust
///   RustEmbed[RustEmbed] --> Rust
///   Axum[Axum] --> Rust
///   WebSocket -.- Axum
///   Aquamarine --> RustDoc
///   MermaidJS -.- Aquamarine
///   VueJS[VueJS/UI Framework] --> UI
///   NPM[NPM Build Env] --> UI((Web User Interface))
///   Element-Plus --> VueJS
///   Rust --> MessageMonitor((Message Monitor App))
///   UI --> MessageMonitor
/// ```
mod slide2 {}

/// Project Files
///
/// Vue/Node.JS proect in directory webui/
///
/// npm artifacts will be stored at webui/dist
///
/// ```text
/// ├── build.rs
/// ├── Cargo.toml
/// ├── LICENSE
/// ├── package-lock.json
/// ├── README.md
/// ├── src
/// │ └── main.rs
/// └── webui
///     ├── index.html
///     ├── package.json
///     ├── package-lock.json
///     ├── public
///     │ ├── CNAME
///     │ ├── element-plus-logo-small.svg
///     │ └── favicon.svg
///     ├── README.md
///     ├── src
///     │ ├── App.vue
///     │ ├── assets
///     │ │ └── logo.png
///     │ ├── components
///     │ │ ├── layouts
///     │ │ │ └── BaseSide.vue
///     │ │ └── MessageMonitor.vue
///     │ ├── components.d.ts
///     │ ├── composables
///     │ │ ├── dark.ts
///     │ │ └── index.ts
///     │ ├── env.d.ts
///     │ ├── main.ts
///     │ └── styles
///     │     ├── element
///     │     │ ├── dark.scss
///     │     │ └── index.scss
///     │     └── index.scss
///     ├── tsconfig.json
///     └── vite.config.ts
///```
mod slide3 {}

#[cfg_attr(doc, aquamarine::aquamarine)]
/// Build Process
///
/// Repo: https://github.com/frehberg/rust-vue-demo
///
/// ```mermaid
///  graph
///     s([Rust Source]) --> m[[Rust macro processor]]
///     v([Vue App]) --> b[[build-script invokes npm build]]
///
///     c[[Rust compiler/linker]]--> f([executable])
///     subgraph rustc[Cargo Builder]
///        b -. generate files .-> d([webui/dist])
///        d -. include .-> m
///        m -->  i([Rust intermediate code])
///        i --> c
///     end
/// ```
mod slide4 {}


#[derive(RustEmbed)]
#[folder = "webui/dist/"]
struct Assets;

// DTO - Data Transfer Object
#[derive(Serialize, Deserialize, Debug)]
struct AppData {
    service_url: Option<String>,
    data: Option<String>,
    notice: Option<String>,
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

enum State {
    Continue,
    ClientWsDisconnected,
    InternalError,
    CanFailed,
}

fn json_message(data: Option<&str>, notice: Option<&str>) -> Result<String, ()> {
    // Serialize data to a JSON string.
    let my_local_ip = local_ip().unwrap();
    let data = AppData {
        service_url: Some(format!("http://{}:3000", my_local_ip)),
        data: data.map(|x| x.to_string()).or(None),
        notice: notice.map(|x| x.to_string()).or(None),
    };

    serde_json::to_string(&data).map(|x| x).or(Err(()))
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

async fn send_ws_message(socket: &mut WebSocket, data: Option<&str>, notice: Option<&str>) -> State {
    if let Ok(txt) = json_message(data, notice) {
        if socket
            .send(Message::Text(txt))
            .await
            .is_err() {
            return State::ClientWsDisconnected;
        }
        return State::Continue;
    } else {
        return State::InternalError;
    }
}

async fn write_frame(can_tx: Option<&CANSocket>, frame: CANFrame) -> State {
    match can_tx {
        Some(tx) => {
            if let Ok(_) = tx.write_frame(frame).unwrap().await {
                println!("write frame succeeded");
                return State::Continue;
            } else {
                return State::CanFailed;
            }
        }
        _ => {
            return State::CanFailed;
        }
    }
}

async fn handle_message(_socket: &mut WebSocket, can_tx: Option<&CANSocket>, msg: Message) -> State {
    match msg {
        Message::Text(t) => {
            println!("client sent: {:?}", t);
            if let Ok(frame) = parse_frame(t) {
                return write_frame(can_tx, frame).await;
            } else {
                return State::InternalError;
            }
        }
        Message::Binary(_) => {
            println!("client sent binary data");
            return State::Continue;
        }
        Message::Ping(_) => {
            println!("socket ping");
            return State::Continue;
        }
        Message::Pong(_) => {
            println!("socket pong");
            return State::Continue;
        }
        Message::Close(_) => {
            println!("client disconnected");
            return State::Continue;
        }
    }
}

async fn handle_time_trigger(socket: &mut WebSocket) -> State {
    println!("time trigger - updating service url");
    send_ws_message(socket, None, None).await
}

async fn handle_can_frame(socket: &mut WebSocket, frame: CANFrame) -> State {
    let fmt = format!("{:X}#{}", frame.id(), hex::encode(frame.data()));
    println!("received can frame {}", fmt);
    return send_ws_message(socket, Some(&fmt), None).await;
}

async fn handle_event_ws_or_can(socket: &mut WebSocket, can_rx: &mut CANSocket, can_tx: &CANSocket) -> State {
    tokio::select! {
        Some(msg)  = socket.recv() => {
             if let Ok(msg) = msg {
                return handle_message(socket, Some(&can_tx), msg).await;
             } else {
                 return State::ClientWsDisconnected;
             }
        }
        Some(Ok(frame)) = can_rx.next() => {
            return handle_can_frame(socket, frame).await ;
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
             return handle_time_trigger(socket, ).await;
        }
    }
}

async fn handle_event_ws(socket: &mut WebSocket) -> State {
    tokio::select! {
        Some(msg)  = socket.recv() => {
             if let Ok(msg) = msg {
                return handle_message(socket, None, msg).await;
             } else {
                 return State::ClientWsDisconnected;
             }
        }
        _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
             return  handle_time_trigger(socket).await;
        }
    }
}


async fn handle_socket_can(socket: &mut WebSocket,
                           can_rx: &mut Result<CANSocket, Error>,
                           can_tx: &Result<CANSocket, Error>) -> State {
    match (can_rx, can_tx) {
        (Ok(rx), Ok(tx)) => {
            return handle_event_ws_or_can(socket,
                                          rx, tx).await;
        }
        _ => {
            return handle_event_ws(socket).await;
        }
    }
}

async fn handle_socket(mut socket: WebSocket) {
    // open canbus and loop
    let can = candev();
    let mut can_rx = CANSocket::open(&can);
    let mut can_tx = CANSocket::open(&can);
    let msg_can_failed = Some("missing CAN device");
    let msg_can_connected = Some("connected to CAN device");

    let notice = if let Ok(_) = can_rx { None } else { msg_can_failed };

    match send_ws_message(&mut socket, None, notice).await {
        ClientWsDisconnected => {
            println!("client disconnected");
            return;
        }
        _ => ()
    }

    loop {
        match handle_socket_can(&mut socket, &mut can_rx, &can_tx).await {
            State::ClientWsDisconnected => {
                println!("client disconnected");
                return;
            }
            State::InternalError => {
                println!("internal server error");
                return;
            }
            State::CanFailed => {
                // signal to UI and try re-open
                match send_ws_message(&mut socket, None, msg_can_failed).await {
                    ClientWsDisconnected => {
                        println!("client disconnected");
                        return;
                    }
                    _ => ()
                }
                can_rx = CANSocket::open(&can);
                can_tx = CANSocket::open(&can);
                if can_rx.is_ok() && can_tx.is_ok() {
                    match send_ws_message(&mut socket, None, msg_can_connected).await {
                        ClientWsDisconnected => {
                            println!("client disconnected");
                            return;
                        }
                        _ => ()
                    }
                }
            }
            State::Continue => {
                if can_rx.is_err() {
                    can_rx = CANSocket::open(&can);
                    can_tx = CANSocket::open(&can);
                    if can_rx.is_ok() && can_tx.is_ok() {
                        match send_ws_message(&mut socket, None, msg_can_connected).await {
                            ClientWsDisconnected => {
                                println!("client disconnected");
                                return;
                            }
                            _ => ()
                        }
                    }
                }
            }
        }
    }
}
