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
    id: u32,
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

fn json_message(id: &u32, data: Option<&str>, notice: Option<&str>) -> Result<String, ()> {
    // Serialize data to a JSON string.
    let my_local_ip = local_ip().unwrap();
    let data = AppData {
        id: id.clone(),
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

async fn write_frame(can_tx: Option<&CANSocket>, frame: CANFrame) -> bool {
    match can_tx {
        Some(tx) => {
            if let Ok(_) = tx.write_frame(frame).unwrap().await {
                println!("write frame succeeded");
                return true;
            } else {
                println!("write frame failed");
                return false;
            }
        }
        _ => {
            println!("could not write frame");
            return false;
        }
    }
}

async fn handle_message(socket: &mut WebSocket, counter: &u32, can_tx: Option<&CANSocket>, msg: Message) -> bool {
    match msg {
        Message::Text(t) => {
            println!("client sent: {:?}", t);
            if let Ok(frame) = parse_frame(t) {
                if !write_frame(can_tx, frame).await {
                    let notice = Some("failed to write can frame");
                    let data = None;
                    if let Ok(txt) = json_message(counter, data, notice) {
                        if socket
                            .send(Message::Text(txt))
                            .await
                            .is_err() {
                            println!("client disconnected");
                            return false;
                        }
                    }
                    return false;
                }
                return true;
            } else {
                println!("invalid CAN message");
                return false;
            }
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

async fn send_status_update(socket: &mut WebSocket, counter: &u32, notice: Option<&str>) -> bool {
    if let Ok(txt) = json_message(counter, None, notice) {
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

async fn handle_time_trigger(socket: &mut WebSocket, counter: &u32) -> bool {
    println!("time trigger - updating service url");
    send_status_update(socket, counter, None).await
}

async fn handle_can_frame(socket: &mut WebSocket, counter: &u32, frame: CANFrame) -> bool {
    let fmt = format!("{:X}#{}", frame.id(), hex::encode(frame.data()));
    println!("received can frame {}", fmt);
    let notice = None;
    let data = Some(fmt.as_str());
    if let Ok(txt) = json_message(counter, data, notice) {
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

async fn handle_event_ws_or_can(socket: &mut WebSocket, counter: &u32, can_rx: &mut CANSocket, can_tx: &CANSocket) -> bool {
    tokio::select! {
            Some(msg)  = socket.recv() => {
                 if let Ok(msg) = msg {
                    if ! handle_message(socket, counter, Some(&can_tx), msg).await {
                        return false;
                    }
                 } else {
                     println!("client disconnected");
                     return false;
                 }
            }
            Some(Ok(frame)) = can_rx.next() => {
                 if ! handle_can_frame(socket, &counter, frame).await {
                     return false;
                }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                 if ! handle_time_trigger(socket, &counter).await {
                    return false;
                 }
            }
        }
    return true;
}

async fn handle_event_ws(socket: &mut WebSocket, counter: &u32) -> bool {
    tokio::select! {
            Some(msg)  = socket.recv() => {
                 if let Ok(msg) = msg {
                    if ! handle_message(socket, counter, None, msg).await {
                        return false;
                    }
                 } else {
                     println!("client disconnected");
                     return false;
                 }
            }
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {
                 if ! handle_time_trigger(socket, &counter).await {
                    return false;
                 }
            }
        }
    return true;
}

async fn handle_socket(mut socket: WebSocket) {
    let mut counter: u32 = 0;

    // open canbus and loop
    let can = candev();
    let mut can_rx = CANSocket::open(&can);
    let mut can_tx = CANSocket::open(&can);
    let notice = if let Ok(_) = can_rx { None } else { Some("missing canbus device") };

    if !send_status_update(&mut socket, &counter, notice).await {
        return;
    }

    counter += 1;

    loop {
        match (&mut can_rx, &can_tx) {
            (Ok(rx), Ok(tx)) => {
                if !handle_event_ws_or_can(&mut socket, &mut counter,
                                           rx, tx).await {
                    return;
                }
            }
            _ => {
                println!("canbus device not found {}", &can);
                if !handle_event_ws(&mut socket, &mut counter).await {
                    return;
                }
                can_rx = CANSocket::open(&can);
                can_tx = CANSocket::open(&can);
            }
        }
        counter += 1;
    }
}
