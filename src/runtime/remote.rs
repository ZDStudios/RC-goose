//! Opt-in remote control through a self-hosted relay (ADR 0059).
//!
//! One background thread keeps a single outbound WebSocket to `[remote].relay_url`, so the
//! controlled computer needs no listening port. Every relay command is translated into the
//! existing local IPC protocol and sent through [`honk_control::send_command`], exactly like the
//! CLI: the runtime loop, engine gates, manners and terminal protection stay authoritative. The
//! relay can only choose from the finite command list below; it never reaches the engine directly.

use honk_config::RemoteConfig;
use honk_control::{ControlCommand, ControlResponse, RemoteIndicator};
use honk_engine::PokeAction;
use serde::Deserialize;
use serde_json::{json, Value};
use std::io;
use std::net::{TcpStream, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};
use tungstenite::client::IntoClientRequest;
use tungstenite::protocol::WebSocketConfig;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

const PROTOCOL_VERSION: u64 = 1;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(200);
const STATUS_INTERVAL: Duration = Duration::from_millis(500);
const STATUS_HEARTBEAT: Duration = Duration::from_secs(15);
const MIN_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
const UNAUTHORIZED_BACKOFF: Duration = Duration::from_secs(300);
const MAX_MESSAGE_BYTES: usize = 16 * 1024;

const STATE_CONNECTING: u8 = 1;
const STATE_CONNECTED: u8 = 2;

type Socket = WebSocket<MaybeTlsStream<TcpStream>>;

/// Runtime-owned remote-control lifecycle: starts from config, follows reloads, and honours the
/// tray's Disconnect for the rest of this session (until the `[remote]` settings change).
#[derive(Default)]
pub(crate) struct RemoteSlot {
    client: Option<RemoteClient>,
    settings: Option<RemoteConfig>,
}

impl RemoteSlot {
    pub(crate) fn sync(&mut self, config: &RemoteConfig) {
        if self.settings.as_ref() == Some(config) {
            return;
        }
        self.settings = Some(config.clone());
        self.stop();
        if !config.enabled {
            return;
        }
        let problems = config.problems();
        if !problems.is_empty() {
            eprintln!(
                "honk300: remote control stays off; fix [remote] in config.toml: {}",
                problems.join("; ")
            );
            return;
        }
        self.client = Some(RemoteClient::start(config.clone()));
    }

    pub(crate) fn disconnect(&mut self) {
        if self.client.is_some() {
            println!("honk300: remote control disconnected from the tray for this session.");
        }
        self.stop();
    }

    pub(crate) fn indicator(&self) -> RemoteIndicator {
        self.client
            .as_ref()
            .map_or(RemoteIndicator::Off, RemoteClient::indicator)
    }

    /// Relay host for the tray label; never includes the path, query or credentials.
    pub(crate) fn relay_host(&self) -> &str {
        self.settings
            .as_ref()
            .filter(|_| self.client.is_some())
            .map_or("", |settings| relay_host(&settings.relay_url))
    }

    fn stop(&mut self) {
        if let Some(client) = self.client.take() {
            client.stop();
        }
    }
}

struct Shared {
    stop: AtomicBool,
    state: AtomicU8,
}

struct RemoteClient {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl RemoteClient {
    fn start(settings: RemoteConfig) -> Self {
        let shared = Arc::new(Shared {
            stop: AtomicBool::new(false),
            state: AtomicU8::new(STATE_CONNECTING),
        });
        let thread_shared = Arc::clone(&shared);
        let thread = std::thread::Builder::new()
            .name("honk300-remote".into())
            .spawn(move || run(&settings, &thread_shared))
            .map_err(|error| eprintln!("honk300: remote control thread could not start ({error})"))
            .ok();
        Self { shared, thread }
    }

    fn indicator(&self) -> RemoteIndicator {
        if self.thread.is_none() {
            return RemoteIndicator::Off;
        }
        match self.shared.state.load(Ordering::Acquire) {
            STATE_CONNECTED => RemoteIndicator::Connected,
            _ => RemoteIndicator::Connecting,
        }
    }

    fn stop(mut self) {
        self.shared.stop.store(true, Ordering::Release);
        // The worker notices within one poll interval; a connect in flight is bounded by the
        // connect timeout, so detach instead of blocking the overlay's event loop on it.
        drop(self.thread.take());
    }
}

impl Drop for RemoteClient {
    fn drop(&mut self) {
        self.shared.stop.store(true, Ordering::Release);
    }
}

fn run(settings: &RemoteConfig, shared: &Shared) {
    let name = device_name(&settings.device_name);
    let host = relay_host(&settings.relay_url).to_string();
    let mut backoff = MIN_BACKOFF;
    let mut last_error = String::new();
    while !shared.stop.load(Ordering::Acquire) {
        shared.state.store(STATE_CONNECTING, Ordering::Release);
        let outcome = session(settings, &name, shared);
        if shared.state.swap(STATE_CONNECTING, Ordering::AcqRel) == STATE_CONNECTED {
            // A session that was accepted and later dropped retries promptly.
            backoff = MIN_BACKOFF;
            last_error.clear();
        }
        match outcome {
            Ok(()) => backoff = MIN_BACKOFF,
            Err(SessionError::Unauthorized) => {
                eprintln!(
                    "honk300: remote relay {host} rejected the token; retrying in 5 minutes."
                );
                backoff = UNAUTHORIZED_BACKOFF;
            }
            Err(SessionError::Io(error)) => {
                let message = error.to_string();
                if message != last_error {
                    eprintln!("honk300: remote relay {host} unavailable ({message}); retrying.");
                    last_error = message;
                }
            }
        }
        let resume = Instant::now() + backoff;
        while Instant::now() < resume && !shared.stop.load(Ordering::Acquire) {
            std::thread::sleep(POLL_INTERVAL);
        }
        backoff = (backoff * 2).min(MAX_BACKOFF).max(backoff);
    }
}

enum SessionError {
    Unauthorized,
    Io(io::Error),
}

impl From<io::Error> for SessionError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

fn ws_error(error: tungstenite::Error) -> io::Error {
    match error {
        tungstenite::Error::Io(error) => error,
        other => io::Error::other(other.to_string()),
    }
}

fn would_block(error: &tungstenite::Error) -> bool {
    matches!(error, tungstenite::Error::Io(e)
        if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut))
}

fn connect(url: &str) -> io::Result<Socket> {
    let request = url
        .into_client_request()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error.to_string()))?;
    let uri = request.uri();
    let host = uri
        .host()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "relay URL has no host"))?
        .trim_start_matches('[')
        .trim_end_matches(']')
        .to_string();
    let port = uri
        .port_u16()
        .unwrap_or(if uri.scheme_str() == Some("wss") {
            443
        } else {
            80
        });
    let mut last = io::Error::new(io::ErrorKind::NotFound, "relay host did not resolve");
    let stream = (host.as_str(), port)
        .to_socket_addrs()?
        .find_map(
            |address| match TcpStream::connect_timeout(&address, CONNECT_TIMEOUT) {
                Ok(stream) => Some(stream),
                Err(error) => {
                    last = error;
                    None
                }
            },
        )
        .ok_or(last)?;
    stream.set_read_timeout(Some(CONNECT_TIMEOUT))?;
    stream.set_write_timeout(Some(CONNECT_TIMEOUT))?;
    stream.set_nodelay(true)?;
    let config = WebSocketConfig::default()
        .max_message_size(Some(MAX_MESSAGE_BYTES))
        .max_frame_size(Some(MAX_MESSAGE_BYTES));
    let (socket, _) = tungstenite::client_tls_with_config(request, stream, Some(config), None)
        .map_err(|error| io::Error::other(format!("handshake failed: {error}")))?;
    let tcp = match socket.get_ref() {
        MaybeTlsStream::Plain(stream) => stream,
        MaybeTlsStream::Rustls(stream) => stream.get_ref(),
        _ => return Err(io::Error::other("unsupported relay transport")),
    };
    tcp.set_read_timeout(Some(POLL_INTERVAL))?;
    Ok(socket)
}

fn send_json(socket: &mut Socket, value: &Value) -> io::Result<()> {
    match socket.send(Message::text(value.to_string())) {
        Ok(()) => Ok(()),
        Err(error) if would_block(&error) => Ok(()),
        Err(error) => Err(ws_error(error)),
    }
}

fn session(settings: &RemoteConfig, name: &str, shared: &Shared) -> Result<(), SessionError> {
    let mut socket = connect(&settings.relay_url)?;
    send_json(
        &mut socket,
        &json!({
            "type": "hello",
            "role": "goose",
            "v": PROTOCOL_VERSION,
            "token": settings.token,
            "name": name,
            "platform": std::env::consts::OS,
            "version": env!("CARGO_PKG_VERSION"),
        }),
    )?;

    let mut welcomed = false;
    let hello_deadline = Instant::now() + CONNECT_TIMEOUT;
    let mut last_status: Option<Value> = None;
    let mut last_poll = Instant::now() - STATUS_INTERVAL;
    let mut last_sent = Instant::now();
    loop {
        if shared.stop.load(Ordering::Acquire) {
            let _ = socket.close(None);
            let _ = socket.flush();
            return Ok(());
        }
        if !welcomed && Instant::now() > hello_deadline {
            return Err(
                io::Error::new(io::ErrorKind::TimedOut, "relay did not answer hello").into(),
            );
        }

        match socket.read() {
            Ok(Message::Text(text)) => match serde_json::from_str::<Incoming>(text.as_str()) {
                Ok(message) if message.kind == "welcome" => {
                    welcomed = true;
                    shared.state.store(STATE_CONNECTED, Ordering::Release);
                    println!("honk300: remote control connected as \"{name}\".");
                }
                Ok(message) if message.kind == "error" => {
                    if message.error.as_deref() == Some("unauthorized") {
                        return Err(SessionError::Unauthorized);
                    }
                    return Err(io::Error::other(format!(
                        "relay refused the goose: {}",
                        message.error.as_deref().unwrap_or("unknown")
                    ))
                    .into());
                }
                Ok(message) if message.kind == "command" && welcomed => {
                    let reply = execute(&message);
                    send_json(&mut socket, &reply)?;
                    last_poll = Instant::now() - STATUS_INTERVAL;
                }
                _ => {}
            },
            Ok(Message::Close(_)) => {
                return Err(io::Error::other("relay closed the connection").into())
            }
            Ok(_) => {}
            Err(error) if would_block(&error) => {}
            Err(tungstenite::Error::ConnectionClosed) => {
                return Err(io::Error::other("relay closed the connection").into())
            }
            Err(error) => return Err(ws_error(error).into()),
        }
        match socket.flush() {
            Ok(()) => {}
            Err(error) if would_block(&error) => {}
            Err(error) => return Err(ws_error(error).into()),
        }

        if welcomed && last_poll.elapsed() >= STATUS_INTERVAL {
            last_poll = Instant::now();
            let status = status_report();
            if last_status.as_ref() != Some(&status) || last_sent.elapsed() >= STATUS_HEARTBEAT {
                send_json(&mut socket, &status)?;
                last_status = Some(status);
                last_sent = Instant::now();
            }
        }
    }
}

#[derive(Debug, Deserialize)]
struct Incoming {
    #[serde(rename = "type")]
    kind: String,
    id: Option<String>,
    cmd: Option<String>,
    x: Option<i64>,
    y: Option<i64>,
    leaf: Option<u64>,
    error: Option<String>,
}

/// Translate one relay command into the finite local IPC vocabulary.
fn command_for(message: &Incoming) -> Result<ControlCommand, &'static str> {
    let point = || -> Result<(i32, i32), &'static str> {
        let x = message.x.and_then(|x| i32::try_from(x).ok());
        let y = message.y.and_then(|y| i32::try_from(y).ok());
        x.zip(y).ok_or("BAD_POINT")
    };
    Ok(match message.cmd.as_deref().ok_or("UNKNOWN_COMMAND")? {
        "honk" => ControlCommand::RemoteDo(PokeAction::Honk),
        "mud" => ControlCommand::RemoteDo(PokeAction::Mud),
        "meme" => ControlCommand::RemoteDo(PokeAction::Meme),
        "note" => ControlCommand::RemoteDo(PokeAction::Note),
        "chase_cursor" => ControlCommand::RemoteDo(PokeAction::Nab),
        "wander" => ControlCommand::RemoteDo(PokeAction::Wander),
        "walk_to" => {
            let (x, y) = point()?;
            ControlCommand::WalkTo(x, y)
        }
        "leaf_place" => {
            let (x, y) = point()?;
            ControlCommand::LeafPlace(x, y)
        }
        "leaf_move" => {
            let (x, y) = point()?;
            ControlCommand::LeafMove(message.leaf.ok_or("BAD_LEAF")?, x, y)
        }
        // The existing graceful walk-off; the force variant is deliberately not reachable.
        "walk_off" => ControlCommand::Stop,
        _ => return Err("UNKNOWN_COMMAND"),
    })
}

fn execute(message: &Incoming) -> Value {
    let id = message.id.clone().unwrap_or_default();
    let command = match command_for(message) {
        Ok(command) => command,
        Err(error) => return json!({ "type": "result", "id": id, "ok": false, "error": error }),
    };
    match honk_control::send_command(command) {
        Ok(ControlResponse::Ok) => json!({ "type": "result", "id": id, "ok": true }),
        Ok(ControlResponse::Leaf(leaf)) => {
            json!({ "type": "result", "id": id, "ok": true, "leaf": leaf })
        }
        Ok(ControlResponse::Err(code)) => {
            json!({ "type": "result", "id": id, "ok": false, "error": code })
        }
        Ok(_) => json!({ "type": "result", "id": id, "ok": false, "error": "UNEXPECTED" }),
        Err(_) => json!({ "type": "result", "id": id, "ok": false, "error": "NOT_RUNNING" }),
    }
}

fn status_report() -> Value {
    let Ok(ControlResponse::Snapshot(snapshot)) =
        honk_control::send_command(ControlCommand::Snapshot)
    else {
        return json!({ "type": "status", "online": false });
    };
    let leaves = match honk_control::send_command(ControlCommand::Leaves) {
        Ok(ControlResponse::Leaves(leaves)) => leaves
            .iter()
            .map(|leaf| json!({ "id": leaf.id, "x": leaf.x, "y": leaf.y }))
            .collect(),
        _ => Vec::new(),
    };
    json!({
        "type": "status",
        "online": true,
        "action": snapshot.task,
        "x": snapshot.x,
        "y": snapshot.y,
        "quiet": snapshot.quiet,
        "screen": { "x": snapshot.left, "y": snapshot.top, "w": snapshot.width, "h": snapshot.height },
        "leaves": leaves,
    })
}

fn relay_host(url: &str) -> &str {
    let rest = url
        .strip_prefix("wss://")
        .or_else(|| url.strip_prefix("ws://"))
        .unwrap_or(url);
    rest.split(['/', '?', '#']).next().unwrap_or_default()
}

/// Configured name, or this computer's name reduced to the relay's name alphabet.
fn device_name(configured: &str) -> String {
    if !configured.is_empty() {
        return configured.to_string();
    }
    let raw = std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default();
    let cleaned: String = raw
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '-'
            }
        })
        .take(40)
        .collect();
    if honk_config::device_name_allowed(&cleaned) {
        cleaned
    } else {
        "goose".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn incoming(cmd: &str, x: Option<i64>, y: Option<i64>, leaf: Option<u64>) -> Incoming {
        Incoming {
            kind: "command".into(),
            id: Some("r1".into()),
            cmd: Some(cmd.into()),
            x,
            y,
            leaf,
            error: None,
        }
    }

    #[test]
    fn relay_commands_map_onto_the_finite_ipc_vocabulary() {
        let cases = [
            ("honk", ControlCommand::RemoteDo(PokeAction::Honk)),
            ("mud", ControlCommand::RemoteDo(PokeAction::Mud)),
            ("meme", ControlCommand::RemoteDo(PokeAction::Meme)),
            ("note", ControlCommand::RemoteDo(PokeAction::Note)),
            ("chase_cursor", ControlCommand::RemoteDo(PokeAction::Nab)),
            ("wander", ControlCommand::RemoteDo(PokeAction::Wander)),
            ("walk_off", ControlCommand::Stop),
        ];
        for (cmd, expected) in cases {
            assert_eq!(command_for(&incoming(cmd, None, None, None)), Ok(expected));
        }
        assert_eq!(
            command_for(&incoming("walk_to", Some(-10), Some(20), None)),
            Ok(ControlCommand::WalkTo(-10, 20))
        );
        assert_eq!(
            command_for(&incoming("leaf_move", Some(1), Some(2), Some(7))),
            Ok(ControlCommand::LeafMove(7, 1, 2))
        );
    }

    #[test]
    fn malformed_or_unknown_relay_commands_are_refused_locally() {
        assert_eq!(
            command_for(&incoming("force_stop", None, None, None)),
            Err("UNKNOWN_COMMAND")
        );
        assert_eq!(
            command_for(&incoming("walk_to", Some(1), None, None)),
            Err("BAD_POINT")
        );
        assert_eq!(
            command_for(&incoming("walk_to", Some(i64::MAX), Some(0), None)),
            Err("BAD_POINT")
        );
        assert_eq!(
            command_for(&incoming("leaf_move", Some(1), Some(2), None)),
            Err("BAD_LEAF")
        );
    }

    #[test]
    fn tray_label_shows_only_the_relay_host() {
        assert_eq!(
            relay_host("wss://goose.example.com/ws?x=1"),
            "goose.example.com"
        );
        assert_eq!(relay_host("ws://127.0.0.1:8090/ws"), "127.0.0.1:8090");
    }

    #[test]
    fn device_names_fall_back_to_a_relay_safe_value() {
        assert_eq!(device_name("Office PC"), "Office PC");
        let derived = device_name("");
        assert!(honk_config::device_name_allowed(&derived), "{derived}");
    }

    #[test]
    fn disconnect_keeps_remote_off_until_settings_change() {
        let mut slot = RemoteSlot::default();
        let mut config = RemoteConfig {
            enabled: true,
            relay_url: "ws://127.0.0.1:9/ws".into(),
            token: "0123456789abcdef".into(),
            device_name: "test".into(),
        };
        slot.sync(&config);
        assert_ne!(slot.indicator(), RemoteIndicator::Off);
        slot.disconnect();
        assert_eq!(slot.indicator(), RemoteIndicator::Off);
        slot.sync(&config);
        assert_eq!(
            slot.indicator(),
            RemoteIndicator::Off,
            "same settings stay disconnected"
        );
        config.device_name = "renamed".into();
        slot.sync(&config);
        assert_ne!(slot.indicator(), RemoteIndicator::Off);
        config.enabled = false;
        slot.sync(&config);
        assert_eq!(slot.indicator(), RemoteIndicator::Off);
    }
}
