//! Shared local control channel for honk300.
//!
//! The CLI and config TUI both speak this finite protocol to the one running
//! goose instance. The engine stays below this layer and only receives closed,
//! platform-neutral command data.

pub mod icon;
mod platform;
mod presence;
mod protocol;
mod session;
mod wayland;

pub use presence::{combine_capabilities, PresenceStatus};
pub use session::{DesktopBackend, DesktopEnvironment, SessionStatus};
pub use wayland::WaylandStatus;

pub use platform::CommandServer;
pub use platform::{
    send_command, wait_for_shutdown, wait_for_shutdown_lease, LifecycleLease, Singleton,
    SingletonStatus, UpdateGuard,
};
pub use protocol::{
    BundleStatus, CapabilityStatus, ControlCommand, ControlResponse, PlatformStatus, ProtocolError,
    RemoteSnapshot, RuntimeStatus, ToyPosition, MAX_REPORTED_TOYS,
};

/// A user action emitted by an operating-system control surface.
///
/// This is intentionally smaller than [`ControlCommand`]. Native trays and the macOS menu bar
/// may open the existing configuration TUI, launch the verified updater helper, or request the
/// existing graceful shutdown, but they do not gain a second configuration model or a new IPC
/// command namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlSurfaceCommand {
    Configure,
    Update,
    Quit,
    /// End the opt-in remote-control connection for this session (ADR 0059). The runtime owns
    /// the connection, so it handles this before the shared configure/update/quit dispatch.
    DisconnectRemote,
}

/// Remote-control connection state that native control surfaces must show (ADR 0059).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RemoteIndicator {
    #[default]
    Off,
    Connecting,
    Connected,
}
