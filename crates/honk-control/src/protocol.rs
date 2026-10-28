#![cfg_attr(not(windows), allow(dead_code))]

use crate::{PresenceStatus, SessionStatus, WaylandStatus};
use honk_engine::{PokeAction, PokeOutcome, ToyKind, MAX_TOYS};
use std::error::Error;
use std::fmt;

const VERSION: &str = "HONK300/1";
const MAX_FRAME_BYTES: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlCommand {
    Stop,
    ForceStop,
    Reload,
    ReloadIf([u8; 32]),
    Status,
    PresenceStatus,
    Session,
    WaylandStatus,
    KwinEnable,
    KwinDisable,
    SwayStatus,
    SwayEnable,
    SwayDisable,
    GnomeStatus,
    GnomeEnable,
    GnomeDisable,
    HyprlandStatus,
    HyprlandEnable,
    HyprlandDisable,
    PointerRequest,
    PointerCancel,
    Do(PokeAction),
    /// Remote-control requests (ADR 0059). They reuse the engine actions but also respect
    /// schedule/presence manners, which local `DO` pokes deliberately do not.
    RemoteDo(PokeAction),
    WalkTo(i32, i32),
    ToyPlace(ToyKind, i32, i32),
    ToyMove(u64, i32, i32),
    Snapshot,
    Toys,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlResponse {
    Ok,
    Err(String),
    Status(RuntimeStatus),
    Presence(PresenceStatus),
    Session(SessionStatus),
    Wayland(WaylandStatus),
    Toy(u64),
    Snapshot(RemoteSnapshot),
    Toys(Vec<ToyPosition>),
}

/// Upper bound on toys reported in one `TOYS` frame; matches the engine toy cap.
pub const MAX_REPORTED_TOYS: usize = MAX_TOYS;

/// Compact live view of the goose for remote status reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteSnapshot {
    pub task: String,
    pub x: i32,
    pub y: i32,
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub quiet: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyPosition {
    pub id: u64,
    pub kind: ToyKind,
    pub x: i32,
    pub y: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformStatus {
    Windows,
    Macos,
    Linux,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundleStatus {
    App,
    Bare,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapabilityStatus {
    Unprobed,
    Supported,
    Unsupported,
    Denied,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeStatus {
    pub running: bool,
    pub platform: PlatformStatus,
    pub bundle: BundleStatus,
    pub overlay: CapabilityStatus,
    pub accessibility: CapabilityStatus,
    pub cursor: CapabilityStatus,
    pub window: CapabilityStatus,
    pub collect: CapabilityStatus,
    pub presence: CapabilityStatus,
    pub audio: CapabilityStatus,
    pub notes: u32,
    pub memes: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    Empty,
    TooLarge,
    WrongVersion,
    UnknownCommand,
    UnknownAction,
    ExtraTokens,
    MissingAction,
    MalformedResponse,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Empty => "empty control frame",
            Self::TooLarge => "control frame is too large",
            Self::WrongVersion => "unsupported control protocol version",
            Self::UnknownCommand => "unknown control command",
            Self::UnknownAction => "unknown poke action",
            Self::ExtraTokens => "unexpected extra tokens",
            Self::MissingAction => "missing poke action",
            Self::MalformedResponse => "malformed control response",
        };
        f.write_str(message)
    }
}

impl Error for ProtocolError {}

impl ControlCommand {
    pub fn encode(self) -> String {
        match self {
            Self::Stop => format!("{VERSION} STOP\n"),
            Self::ForceStop => format!("{VERSION} FORCE_STOP\n"),
            Self::ReloadIf(identity) => {
                let hex = identity
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                format!("{VERSION} RELOAD_IF {hex}\n")
            }
            Self::Reload => format!("{VERSION} RELOAD\n"),
            Self::Status => format!("{VERSION} STATUS\n"),
            Self::PresenceStatus => format!("{VERSION} PRESENCE\n"),
            Self::Session => format!("{VERSION} SESSION\n"),
            Self::WaylandStatus => format!("{VERSION} WAYLAND\n"),
            Self::KwinEnable => format!("{VERSION} KWIN_ENABLE\n"),
            Self::KwinDisable => format!("{VERSION} KWIN_DISABLE\n"),
            Self::SwayStatus => format!("{VERSION} SWAY\n"),
            Self::SwayEnable => format!("{VERSION} SWAY_ENABLE\n"),
            Self::SwayDisable => format!("{VERSION} SWAY_DISABLE\n"),
            Self::GnomeStatus => format!("{VERSION} GNOME\n"),
            Self::GnomeEnable => format!("{VERSION} GNOME_ENABLE\n"),
            Self::GnomeDisable => format!("{VERSION} GNOME_DISABLE\n"),
            Self::HyprlandStatus => format!("{VERSION} HYPRLAND\n"),
            Self::HyprlandEnable => format!("{VERSION} HYPRLAND_ENABLE\n"),
            Self::HyprlandDisable => format!("{VERSION} HYPRLAND_DISABLE\n"),
            Self::PointerRequest => format!("{VERSION} POINTER_REQUEST\n"),
            Self::PointerCancel => format!("{VERSION} POINTER_CANCEL\n"),
            Self::Do(action) => format!("{VERSION} DO {}\n", encode_action(action)),
            Self::RemoteDo(action) => format!("{VERSION} RDO {}\n", encode_action(action)),
            Self::WalkTo(x, y) => format!("{VERSION} WALK_TO {x} {y}\n"),
            Self::ToyPlace(kind, x, y) => format!("{VERSION} TOY_PLACE {} {x} {y}\n", kind.code()),
            Self::ToyMove(id, x, y) => format!("{VERSION} TOY_MOVE {id} {x} {y}\n"),
            Self::Snapshot => format!("{VERSION} SNAPSHOT\n"),
            Self::Toys => format!("{VERSION} TOYS\n"),
        }
    }

    pub fn decode(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.is_empty() {
            return Err(ProtocolError::Empty);
        }
        if frame.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::TooLarge);
        }
        let text = std::str::from_utf8(frame).map_err(|_| ProtocolError::UnknownCommand)?;
        let mut parts = text.trim().split_ascii_whitespace();
        if parts.next() != Some(VERSION) {
            return Err(ProtocolError::WrongVersion);
        }
        let Some(command) = parts.next() else {
            return Err(ProtocolError::UnknownCommand);
        };
        match command {
            "WAYLAND" | "KWIN_ENABLE" | "KWIN_DISABLE" | "SWAY" | "SWAY_ENABLE"
            | "SWAY_DISABLE" | "HYPRLAND" | "HYPRLAND_ENABLE" | "HYPRLAND_DISABLE" | "GNOME"
            | "GNOME_ENABLE" | "GNOME_DISABLE" | "POINTER_REQUEST" | "POINTER_CANCEL" => {
                ensure_end(parts)?;
                Ok(match command {
                    "WAYLAND" => Self::WaylandStatus,
                    "KWIN_ENABLE" => Self::KwinEnable,
                    "KWIN_DISABLE" => Self::KwinDisable,
                    "SWAY" => Self::SwayStatus,
                    "SWAY_ENABLE" => Self::SwayEnable,
                    "SWAY_DISABLE" => Self::SwayDisable,
                    "GNOME" => Self::GnomeStatus,
                    "GNOME_ENABLE" => Self::GnomeEnable,
                    "GNOME_DISABLE" => Self::GnomeDisable,
                    "HYPRLAND" => Self::HyprlandStatus,
                    "HYPRLAND_ENABLE" => Self::HyprlandEnable,
                    "HYPRLAND_DISABLE" => Self::HyprlandDisable,
                    "POINTER_REQUEST" => Self::PointerRequest,
                    _ => Self::PointerCancel,
                })
            }
            "STOP" => {
                ensure_end(parts)?;
                Ok(Self::Stop)
            }
            "FORCE_STOP" => {
                ensure_end(parts)?;
                Ok(Self::ForceStop)
            }
            "RELOAD" => {
                ensure_end(parts)?;
                Ok(Self::Reload)
            }
            "RELOAD_IF" => {
                let hex = parts.next().ok_or(ProtocolError::MissingAction)?;
                ensure_end(parts)?;
                if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(ProtocolError::UnknownCommand);
                }
                let mut identity = [0; 32];
                for (index, byte) in identity.iter_mut().enumerate() {
                    *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16)
                        .map_err(|_| ProtocolError::UnknownCommand)?;
                }
                Ok(Self::ReloadIf(identity))
            }
            "STATUS" => {
                ensure_end(parts)?;
                Ok(Self::Status)
            }
            "PRESENCE" => {
                ensure_end(parts)?;
                Ok(Self::PresenceStatus)
            }
            "SESSION" => {
                ensure_end(parts)?;
                Ok(Self::Session)
            }
            "DO" | "RDO" => {
                let Some(action) = parts.next() else {
                    return Err(ProtocolError::MissingAction);
                };
                ensure_end(parts)?;
                let action = decode_action(action)?;
                Ok(if command == "DO" {
                    Self::Do(action)
                } else {
                    Self::RemoteDo(action)
                })
            }
            "WALK_TO" => {
                let x = parse_number(parts.next())?;
                let y = parse_number(parts.next())?;
                ensure_end(parts)?;
                Ok(Self::WalkTo(x, y))
            }
            "TOY_PLACE" => {
                let kind = parts.next().ok_or(ProtocolError::MissingAction)?;
                let kind = ToyKind::from_code(kind).ok_or(ProtocolError::UnknownAction)?;
                let x = parse_number(parts.next())?;
                let y = parse_number(parts.next())?;
                ensure_end(parts)?;
                Ok(Self::ToyPlace(kind, x, y))
            }
            "TOY_MOVE" => {
                let id = parse_number(parts.next())?;
                let x = parse_number(parts.next())?;
                let y = parse_number(parts.next())?;
                ensure_end(parts)?;
                Ok(Self::ToyMove(id, x, y))
            }
            "SNAPSHOT" | "TOYS" => {
                ensure_end(parts)?;
                Ok(if command == "SNAPSHOT" {
                    Self::Snapshot
                } else {
                    Self::Toys
                })
            }
            _ => Err(ProtocolError::UnknownCommand),
        }
    }
}

impl From<PokeOutcome> for ControlResponse {
    /// Map an engine poke result onto the wire response so `do <action>` reports the real
    /// outcome instead of a blanket "received". Rejections become error codes the CLI/TUI show.
    fn from(outcome: PokeOutcome) -> Self {
        match outcome {
            PokeOutcome::Applied => Self::Ok,
            PokeOutcome::Busy => Self::Err("BUSY".into()),
            PokeOutcome::Unsupported => Self::Err("UNSUPPORTED".into()),
            PokeOutcome::Quiet => Self::Err("QUIET".into()),
        }
    }
}

impl ControlResponse {
    pub fn encode(&self) -> String {
        match self {
            Self::Ok => "OK\n".to_string(),
            Self::Err(code) => format!("ERR {code}\n"),
            Self::Status(status) => status.encode(),
            Self::Presence(status) => status.encode(),
            Self::Session(session) => session.encode(),
            Self::Wayland(status) => status.encode(),
            Self::Toy(id) => format!("TOY {id}\n"),
            Self::Snapshot(snapshot) => snapshot.encode(),
            Self::Toys(toys) => {
                let mut frame = String::from("TOYS");
                for toy in toys.iter().take(MAX_REPORTED_TOYS) {
                    frame.push_str(&format!(
                        " {}:{}:{},{}",
                        toy.id,
                        toy.kind.code(),
                        toy.x,
                        toy.y
                    ));
                }
                frame.push('\n');
                frame
            }
        }
    }

    pub fn decode(frame: &[u8]) -> Result<Self, ProtocolError> {
        if frame.is_empty() {
            return Err(ProtocolError::Empty);
        }
        if frame.len() > MAX_FRAME_BYTES {
            return Err(ProtocolError::TooLarge);
        }
        let text = std::str::from_utf8(frame).map_err(|_| ProtocolError::MalformedResponse)?;
        let mut parts = text.trim().split_ascii_whitespace();
        match parts.next() {
            Some("OK") => {
                ensure_end(parts)?;
                Ok(Self::Ok)
            }
            Some("ERR") => {
                let Some(code) = parts.next() else {
                    return Err(ProtocolError::MalformedResponse);
                };
                ensure_end(parts)?;
                Ok(Self::Err(code.to_string()))
            }
            Some("STATUS") => RuntimeStatus::decode(parts).map(Self::Status),
            Some("PRESENCE") => PresenceStatus::decode(parts).map(Self::Presence),
            Some("SESSION") => SessionStatus::decode(parts).map(Self::Session),
            Some("WAYLAND") => WaylandStatus::decode(parts).map(Self::Wayland),
            Some("TOY") => {
                let id =
                    parse_number(parts.next()).map_err(|_| ProtocolError::MalformedResponse)?;
                ensure_end(parts)?;
                Ok(Self::Toy(id))
            }
            Some("SNAP") => RemoteSnapshot::decode(parts).map(Self::Snapshot),
            Some("TOYS") => {
                let mut toys = Vec::new();
                for token in parts {
                    if toys.len() == MAX_REPORTED_TOYS {
                        return Err(ProtocolError::MalformedResponse);
                    }
                    toys.push(ToyPosition::decode(token)?);
                }
                Ok(Self::Toys(toys))
            }
            _ => Err(ProtocolError::MalformedResponse),
        }
    }
}

impl RuntimeStatus {
    pub fn not_running() -> Self {
        Self {
            running: false,
            platform: PlatformStatus::current(),
            bundle: BundleStatus::Unknown,
            overlay: CapabilityStatus::Unprobed,
            accessibility: CapabilityStatus::Unprobed,
            cursor: CapabilityStatus::Unprobed,
            window: CapabilityStatus::Unprobed,
            collect: CapabilityStatus::Unprobed,
            presence: CapabilityStatus::Unprobed,
            audio: CapabilityStatus::Unprobed,
            notes: 0,
            memes: 0,
        }
    }

    fn encode(&self) -> String {
        format!(
            "STATUS G={} P={} B={} V={} A={} C={} W={} K={} R={} O={} N={} M={}\n",
            u8::from(self.running),
            self.platform.encode(),
            self.bundle.encode(),
            self.overlay.encode(),
            self.accessibility.encode(),
            self.cursor.encode(),
            self.window.encode(),
            self.collect.encode(),
            self.presence.encode(),
            self.audio.encode(),
            self.notes,
            self.memes
        )
    }

    fn decode(parts: std::str::SplitAsciiWhitespace<'_>) -> Result<Self, ProtocolError> {
        let mut status = RuntimeStatus::not_running();
        let mut seen = 0u16;
        for token in parts {
            let Some((key, value)) = token.split_once('=') else {
                return Err(ProtocolError::MalformedResponse);
            };
            match key {
                "G" => {
                    status.running = match value {
                        "0" => false,
                        "1" => true,
                        _ => return Err(ProtocolError::MalformedResponse),
                    };
                    seen |= 1 << 0;
                }
                "P" => {
                    status.platform = PlatformStatus::decode(value)?;
                    seen |= 1 << 1;
                }
                "B" => {
                    status.bundle = BundleStatus::decode(value)?;
                    seen |= 1 << 2;
                }
                "V" => {
                    status.overlay = CapabilityStatus::decode(value)?;
                    seen |= 1 << 11;
                }
                "A" => {
                    status.accessibility = CapabilityStatus::decode(value)?;
                    seen |= 1 << 3;
                }
                "C" => {
                    status.cursor = CapabilityStatus::decode(value)?;
                    seen |= 1 << 4;
                }
                "W" => {
                    status.window = CapabilityStatus::decode(value)?;
                    seen |= 1 << 5;
                }
                "K" => {
                    status.collect = CapabilityStatus::decode(value)?;
                    seen |= 1 << 6;
                }
                "R" => {
                    status.presence = CapabilityStatus::decode(value)?;
                    seen |= 1 << 7;
                }
                "O" => {
                    status.audio = CapabilityStatus::decode(value)?;
                    seen |= 1 << 8;
                }
                "N" => {
                    status.notes = value
                        .parse()
                        .map_err(|_| ProtocolError::MalformedResponse)?;
                    seen |= 1 << 9;
                }
                "M" => {
                    status.memes = value
                        .parse()
                        .map_err(|_| ProtocolError::MalformedResponse)?;
                    seen |= 1 << 10;
                }
                _ => return Err(ProtocolError::MalformedResponse),
            }
        }
        if seen == 0b1111_1111_1111 {
            Ok(status)
        } else {
            Err(ProtocolError::MalformedResponse)
        }
    }
}

impl RemoteSnapshot {
    fn encode(&self) -> String {
        format!(
            "SNAP T={} X={} Y={} L={} U={} W={} H={} Q={}\n",
            self.task,
            self.x,
            self.y,
            self.left,
            self.top,
            self.width,
            self.height,
            u8::from(self.quiet)
        )
    }

    fn decode(parts: std::str::SplitAsciiWhitespace<'_>) -> Result<Self, ProtocolError> {
        let mut snapshot = RemoteSnapshot {
            task: String::new(),
            x: 0,
            y: 0,
            left: 0,
            top: 0,
            width: 0,
            height: 0,
            quiet: false,
        };
        let mut seen = 0u8;
        for token in parts {
            let (key, value) = token
                .split_once('=')
                .ok_or(ProtocolError::MalformedResponse)?;
            let malformed = |_| ProtocolError::MalformedResponse;
            let bit = match key {
                "T" => {
                    if value.is_empty()
                        || value.len() > 32
                        || !value.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                    {
                        return Err(ProtocolError::MalformedResponse);
                    }
                    snapshot.task = value.to_string();
                    0
                }
                "X" => {
                    snapshot.x = parse_number(Some(value)).map_err(malformed)?;
                    1
                }
                "Y" => {
                    snapshot.y = parse_number(Some(value)).map_err(malformed)?;
                    2
                }
                "L" => {
                    snapshot.left = parse_number(Some(value)).map_err(malformed)?;
                    3
                }
                "U" => {
                    snapshot.top = parse_number(Some(value)).map_err(malformed)?;
                    4
                }
                "W" => {
                    snapshot.width = parse_number(Some(value)).map_err(malformed)?;
                    5
                }
                "H" => {
                    snapshot.height = parse_number(Some(value)).map_err(malformed)?;
                    6
                }
                "Q" => {
                    snapshot.quiet = match value {
                        "0" => false,
                        "1" => true,
                        _ => return Err(ProtocolError::MalformedResponse),
                    };
                    7
                }
                _ => return Err(ProtocolError::MalformedResponse),
            };
            seen |= 1 << bit;
        }
        if seen == u8::MAX {
            Ok(snapshot)
        } else {
            Err(ProtocolError::MalformedResponse)
        }
    }
}

impl ToyPosition {
    fn decode(token: &str) -> Result<Self, ProtocolError> {
        let malformed = |_| ProtocolError::MalformedResponse;
        let mut fields = token.splitn(3, ':');
        let (Some(id), Some(kind), Some(point)) = (fields.next(), fields.next(), fields.next())
        else {
            return Err(ProtocolError::MalformedResponse);
        };
        let kind = ToyKind::from_code(kind).ok_or(ProtocolError::MalformedResponse)?;
        let (x, y) = point
            .split_once(',')
            .ok_or(ProtocolError::MalformedResponse)?;
        Ok(Self {
            id: parse_number(Some(id)).map_err(malformed)?,
            kind,
            x: parse_number(Some(x)).map_err(malformed)?,
            y: parse_number(Some(y)).map_err(malformed)?,
        })
    }
}

impl PlatformStatus {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Macos
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Other
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::Macos => "macOS",
            Self::Linux => "Linux",
            Self::Other => "other",
        }
    }

    fn encode(self) -> &'static str {
        match self {
            Self::Windows => "WIN",
            Self::Macos => "MAC",
            Self::Linux => "LIN",
            Self::Other => "OTH",
        }
    }

    fn decode(value: &str) -> Result<Self, ProtocolError> {
        match value {
            "WIN" => Ok(Self::Windows),
            "MAC" => Ok(Self::Macos),
            "LIN" => Ok(Self::Linux),
            "OTH" => Ok(Self::Other),
            _ => Err(ProtocolError::MalformedResponse),
        }
    }
}

impl BundleStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::App => ".app",
            Self::Bare => "bare binary",
            Self::Unknown => "unknown",
        }
    }

    fn encode(self) -> &'static str {
        match self {
            Self::App => "APP",
            Self::Bare => "BARE",
            Self::Unknown => "UNK",
        }
    }

    fn decode(value: &str) -> Result<Self, ProtocolError> {
        match value {
            "APP" => Ok(Self::App),
            "BARE" => Ok(Self::Bare),
            "UNK" => Ok(Self::Unknown),
            _ => Err(ProtocolError::MalformedResponse),
        }
    }
}

impl CapabilityStatus {
    pub fn active(self) -> bool {
        matches!(self, Self::Supported)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Unprobed => "unprobed",
            Self::Supported => "supported",
            Self::Unsupported => "unsupported",
            Self::Denied => "denied",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn encode(self) -> &'static str {
        match self {
            Self::Unprobed => "N",
            Self::Supported => "S",
            Self::Unsupported => "U",
            Self::Denied => "D",
            Self::Failed => "F",
        }
    }

    pub(crate) fn decode(value: &str) -> Result<Self, ProtocolError> {
        match value {
            "N" => Ok(Self::Unprobed),
            "S" => Ok(Self::Supported),
            "U" => Ok(Self::Unsupported),
            "D" => Ok(Self::Denied),
            "F" => Ok(Self::Failed),
            _ => Err(ProtocolError::MalformedResponse),
        }
    }
}

fn ensure_end(mut parts: std::str::SplitAsciiWhitespace<'_>) -> Result<(), ProtocolError> {
    if parts.next().is_some() {
        Err(ProtocolError::ExtraTokens)
    } else {
        Ok(())
    }
}

fn parse_number<T: std::str::FromStr>(token: Option<&str>) -> Result<T, ProtocolError> {
    token
        .ok_or(ProtocolError::MissingAction)?
        .parse()
        .map_err(|_| ProtocolError::UnknownCommand)
}

fn encode_action(action: PokeAction) -> &'static str {
    match action {
        PokeAction::Honk => "HONK",
        PokeAction::Wander => "WANDER",
        PokeAction::Mud => "MUD",
        PokeAction::Meme => "MEME",
        PokeAction::Note => "NOTE",
        PokeAction::Nab => "NAB",
    }
}

fn decode_action(action: &str) -> Result<PokeAction, ProtocolError> {
    match action {
        "HONK" => Ok(PokeAction::Honk),
        "WANDER" => Ok(PokeAction::Wander),
        "MUD" => Ok(PokeAction::Mud),
        "MEME" => Ok(PokeAction::Meme),
        "NOTE" => Ok(PokeAction::Note),
        "NAB" => Ok(PokeAction::Nab),
        _ => Err(ProtocolError::UnknownAction),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_commands() {
        for command in [
            ControlCommand::Stop,
            ControlCommand::ForceStop,
            ControlCommand::Reload,
            ControlCommand::ReloadIf([0xa5; 32]),
            ControlCommand::Status,
            ControlCommand::Do(PokeAction::Honk),
            ControlCommand::Do(PokeAction::Wander),
            ControlCommand::Do(PokeAction::Mud),
            ControlCommand::Do(PokeAction::Meme),
            ControlCommand::Do(PokeAction::Note),
            ControlCommand::Do(PokeAction::Nab),
            ControlCommand::RemoteDo(PokeAction::Meme),
            ControlCommand::WalkTo(-2_560, 1_439),
            ControlCommand::ToyPlace(ToyKind::Ball, 0, 0),
            ControlCommand::ToyPlace(ToyKind::Stick, -1, 2),
            ControlCommand::ToyPlace(ToyKind::Crumbs, 3, -4),
            ControlCommand::ToyMove(u64::MAX, i32::MIN, i32::MAX),
            ControlCommand::Snapshot,
            ControlCommand::Toys,
        ] {
            assert_eq!(
                ControlCommand::decode(command.encode().as_bytes()).unwrap(),
                command
            );
        }
    }

    #[test]
    fn rejects_malformed_commands() {
        assert_eq!(ControlCommand::decode(b""), Err(ProtocolError::Empty));
        assert_eq!(
            ControlCommand::decode(b"HONK299/1 STOP\n"),
            Err(ProtocolError::WrongVersion)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 DO\n"),
            Err(ProtocolError::MissingAction)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 DO YELL\n"),
            Err(ProtocolError::UnknownAction)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 STOP NOW\n"),
            Err(ProtocolError::ExtraTokens)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 FORCE_STOP NOW\n"),
            Err(ProtocolError::ExtraTokens)
        );
        assert_eq!(
            ControlCommand::decode(&[b'X'; MAX_FRAME_BYTES + 1]),
            Err(ProtocolError::TooLarge)
        );
    }

    #[test]
    fn rejects_malformed_remote_commands() {
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 WALK_TO 10\n"),
            Err(ProtocolError::MissingAction)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 WALK_TO 1.5 2\n"),
            Err(ProtocolError::UnknownCommand)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 TOY_MOVE -1 2 3\n"),
            Err(ProtocolError::UnknownCommand)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 TOY_PLACE X 2 3\n"),
            Err(ProtocolError::UnknownAction)
        );
        assert_eq!(
            ControlCommand::decode(b"HONK300/1 SNAPSHOT NOW\n"),
            Err(ProtocolError::ExtraTokens)
        );
    }

    #[test]
    fn remote_responses_round_trip_under_the_frame_limit() {
        let snapshot = ControlResponse::Snapshot(RemoteSnapshot {
            task: "collect_window_annoyed".into(),
            x: -1_234_567,
            y: 1_234_567,
            left: -7_680,
            top: -2_160,
            width: 15_360,
            height: 4_320,
            quiet: true,
        });
        let toys = ControlResponse::Toys(
            (0..MAX_REPORTED_TOYS as u64)
                .map(|id| ToyPosition {
                    id: 1_000 + id,
                    kind: ToyKind::Crumbs,
                    x: -10_000,
                    y: 10_000,
                })
                .collect(),
        );
        for response in [
            snapshot,
            toys,
            ControlResponse::Toy(42),
            ControlResponse::Toys(vec![]),
        ] {
            let frame = response.encode();
            assert!(frame.len() <= MAX_FRAME_BYTES, "{frame}");
            assert_eq!(ControlResponse::decode(frame.as_bytes()).unwrap(), response);
        }
        assert_eq!(
            ControlResponse::decode(b"SNAP T=Wander X=1 Y=1 L=0 U=0 W=1 H=1 Q=0\n"),
            Err(ProtocolError::MalformedResponse)
        );
        assert_eq!(
            ControlResponse::decode(b"SNAP T=wander X=1 Y=1 L=0 U=0 W=1 H=1\n"),
            Err(ProtocolError::MalformedResponse)
        );
    }

    #[test]
    fn maps_poke_outcomes_to_responses() {
        assert_eq!(
            ControlResponse::from(PokeOutcome::Applied),
            ControlResponse::Ok
        );
        assert_eq!(
            ControlResponse::from(PokeOutcome::Busy),
            ControlResponse::Err("BUSY".into())
        );
        assert_eq!(
            ControlResponse::from(PokeOutcome::Unsupported),
            ControlResponse::Err("UNSUPPORTED".into())
        );
        assert_eq!(
            ControlResponse::from(PokeOutcome::Quiet),
            ControlResponse::Err("QUIET".into())
        );
    }

    #[test]
    fn round_trips_responses() {
        assert_eq!(
            ControlResponse::decode(ControlResponse::Ok.encode().as_bytes()).unwrap(),
            ControlResponse::Ok
        );
        assert_eq!(
            ControlResponse::decode(ControlResponse::Err("BUSY".into()).encode().as_bytes())
                .unwrap(),
            ControlResponse::Err("BUSY".into())
        );
    }

    #[test]
    fn round_trips_status_response_under_frame_limit() {
        let status = RuntimeStatus {
            running: true,
            platform: PlatformStatus::Macos,
            bundle: BundleStatus::App,
            overlay: CapabilityStatus::Supported,
            accessibility: CapabilityStatus::Denied,
            cursor: CapabilityStatus::Denied,
            window: CapabilityStatus::Supported,
            collect: CapabilityStatus::Failed,
            presence: CapabilityStatus::Unsupported,
            audio: CapabilityStatus::Supported,
            notes: 12,
            memes: 34,
        };
        let frame = ControlResponse::Status(status).encode();
        assert!(frame.len() <= MAX_FRAME_BYTES, "{frame}");
        assert_eq!(
            ControlResponse::decode(frame.as_bytes()).unwrap(),
            ControlResponse::Status(status)
        );
    }

    #[test]
    fn rejects_malformed_status_response() {
        assert_eq!(
            ControlResponse::decode(b"STATUS G=1 P=MAC\n"),
            Err(ProtocolError::MalformedResponse)
        );
        assert_eq!(
            ControlResponse::decode(
                b"STATUS G=1 P=MAC B=APP V=S A=X C=S W=S K=S R=S O=S N=1 M=2\n"
            ),
            Err(ProtocolError::MalformedResponse)
        );
        // The overlay field (V=) is part of the required set, mirroring how every other status
        // key is mandatory: an otherwise-complete frame that omits it is malformed.
        assert_eq!(
            ControlResponse::decode(b"STATUS G=1 P=MAC B=APP A=S C=S W=S K=S R=S O=S N=1 M=2\n"),
            Err(ProtocolError::MalformedResponse)
        );
    }

    #[test]
    fn stopped_status_reports_capabilities_as_unprobed() {
        let status = RuntimeStatus::not_running();
        for capability in [
            status.overlay,
            status.accessibility,
            status.cursor,
            status.window,
            status.collect,
            status.presence,
            status.audio,
        ] {
            assert_eq!(capability, CapabilityStatus::Unprobed);
            assert_eq!(capability.label(), "unprobed");
        }
        let frame = ControlResponse::Status(status).encode();
        assert_eq!(
            ControlResponse::decode(frame.as_bytes()).unwrap(),
            ControlResponse::Status(status)
        );
    }
}
