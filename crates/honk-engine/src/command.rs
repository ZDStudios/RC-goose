//! Platform-free commands sent from the CLI/TUI control plane into the running world.
//!
//! M10 owns the local IPC transport outside this crate. The engine only sees a closed
//! enum of goose actions and returns a deterministic outcome that callers can report.

/// A live action that can be requested through IPC.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PokeAction {
    Honk,
    Wander,
    Mud,
    Meme,
    Note,
    Nab,
}

/// Result of applying a live action to the current world state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PokeOutcome {
    Applied,
    Unsupported,
    Busy,
    /// Refused because schedule/presence manners (quiet hours, DND, fullscreen, calm goose) are
    /// active. Only remote-control requests report this; local pokes keep their existing gates.
    Quiet,
}
