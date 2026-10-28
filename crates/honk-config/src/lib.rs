//! Versioned TOML configuration for honk300.
//!
//! This crate is intentionally above `honk-engine`: it may know about paths,
//! TOML, and user-facing validation, then converts validated settings into the
//! platform-free option structs consumed by the engine.

mod editing;
pub use editing::{config_path_identity, ConfigRevision, ConfigSnapshot};

use honk_engine::{
    AppearanceOptions, CollectWindowCapabilities, CollectWindowOptions, FootMarkTiming,
    ForeignWindowOptions, HourlyHonkOptions, InteractionOptions, LocalMinute, MoodIntensity,
    MoodOptions, MouseStealOptions, ParametersTable, RenderPalette, ScheduleOptions, TimingOptions,
    WorldOptions,
};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use toml_edit::{value, DocumentMut, Item, Table};

pub const CONFIG_VERSION: u32 = 2;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub goose_config_version: u32,
    pub behavior: BehaviorConfig,
    pub colors: ColorConfig,
    pub speeds: SpeedConfig,
    pub mud: MudConfig,
    pub mouse: MouseConfig,
    pub behaviors: FutureBehaviorConfig,
    pub moods: MoodConfig,
    pub mischief: MischiefConfig,
    pub interaction: InteractionConfig,
    pub schedule: ScheduleConfig,
    pub appearance: AppearanceConfig,
    pub audio: AudioConfig,
    pub safety: SafetyConfig,
    pub platform: PlatformConfig,
    pub lifecycle: LifecycleConfig,
    pub remote: RemoteConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            goose_config_version: CONFIG_VERSION,
            behavior: BehaviorConfig::default(),
            colors: ColorConfig::default(),
            speeds: SpeedConfig::default(),
            mud: MudConfig::default(),
            mouse: MouseConfig::default(),
            behaviors: FutureBehaviorConfig::default(),
            moods: MoodConfig::default(),
            mischief: MischiefConfig::default(),
            interaction: InteractionConfig::default(),
            schedule: ScheduleConfig::default(),
            appearance: AppearanceConfig::default(),
            audio: AudioConfig::default(),
            safety: SafetyConfig::default(),
            platform: PlatformConfig::default(),
            lifecycle: LifecycleConfig::default(),
            remote: RemoteConfig::default(),
        }
    }
}

/// Opt-in remote control through a self-hosted relay (ADR 0059). Off by default; changes apply
/// on reload. The goose only ever opens an outbound WebSocket to `relay_url`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct RemoteConfig {
    pub enabled: bool,
    /// `wss://` relay endpoint, e.g. `wss://goose.example.com/ws`. Plain `ws://` is accepted only
    /// for `localhost`/loopback testing.
    pub relay_url: String,
    /// Shared secret matching the relay's `RELAY_TOKEN`.
    pub token: String,
    /// Name shown in the control panel. Empty uses the computer's name.
    pub device_name: String,
}

impl RemoteConfig {
    /// Validation problems for an enabled remote section (an empty list means usable).
    pub fn problems(&self) -> Vec<String> {
        let mut errors = Vec::new();
        if !self.enabled {
            return errors;
        }
        if !relay_url_allowed(&self.relay_url) {
            errors.push(
                "remote.relay_url must be a wss:// URL (ws:// is allowed only for localhost)"
                    .into(),
            );
        }
        if self.token.trim().len() < 16 {
            errors.push("remote.token must be at least 16 characters".into());
        }
        if !self.device_name.is_empty() && !device_name_allowed(&self.device_name) {
            errors.push(
                "remote.device_name must be 1-40 letters, digits, spaces, dots, dashes or underscores"
                    .into(),
            );
        }
        errors
    }
}

fn relay_url_allowed(url: &str) -> bool {
    let (secure, rest) = if let Some(rest) = url.strip_prefix("wss://") {
        (true, rest)
    } else if let Some(rest) = url.strip_prefix("ws://") {
        (false, rest)
    } else {
        return false;
    };
    let authority = rest.split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || url.chars().any(char::is_whitespace) {
        return false;
    }
    let host = if let Some(bracketed) = authority.strip_prefix('[') {
        bracketed.split(']').next().unwrap_or_default()
    } else {
        authority.split(':').next().unwrap_or_default()
    };
    secure || matches!(host, "localhost" | "127.0.0.1" | "::1")
}

/// The relay accepts the same name alphabet (`relay/server.js`).
pub fn device_name_allowed(name: &str) -> bool {
    let mut bytes = name.bytes();
    name.len() <= 40
        && bytes.next().is_some_and(|b| b.is_ascii_alphanumeric())
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'.' | b'_' | b'-'))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LifecycleConfig {
    /// Reconcile the install family's one owned login-start mechanism. This setting never creates
    /// an additional scheduler/startup-folder owner alongside the existing platform mechanism.
    pub autostart_on_login: bool,
}

impl ColorConfig {
    fn derived_palette(&self) -> RenderPalette {
        let d = RenderPalette::default();
        RenderPalette::from_legacy(
            parse_hex_rgb(&self.goose_white).unwrap_or(d.goose_white),
            parse_hex_rgb(&self.goose_orange).unwrap_or(d.goose_orange),
            parse_hex_rgb(&self.goose_outline).unwrap_or(d.goose_outline),
        )
    }

    /// Effective V2 tones as hex strings: the explicit key when set, else the tone
    /// derived from the legacy three (what the renderer will actually use).
    pub fn shade_hex(&self) -> String {
        self.goose_shade
            .clone()
            .unwrap_or_else(|| hex_string(self.derived_palette().goose_shade))
    }

    pub fn wing_hex(&self) -> String {
        self.goose_wing
            .clone()
            .unwrap_or_else(|| hex_string(self.derived_palette().goose_wing))
    }

    pub fn orange_dark_hex(&self) -> String {
        self.goose_orange_dark
            .clone()
            .unwrap_or_else(|| hex_string(self.derived_palette().goose_orange_dark))
    }
}

fn hex_string(rgb: (u8, u8, u8)) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb.0, rgb.1, rgb.2)
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BehaviorConfig {
    pub can_attack_mouse: bool,
    pub attack_randomly: bool,
    pub use_custom_colors: bool,
    pub first_wander_time_seconds: f32,
    pub min_wandering_time_seconds: f32,
    pub max_wandering_time_seconds: f32,
}

impl Default for BehaviorConfig {
    fn default() -> Self {
        Self {
            can_attack_mouse: true,
            attack_randomly: false,
            use_custom_colors: false,
            first_wander_time_seconds: 20.0,
            min_wandering_time_seconds: 20.0,
            max_wandering_time_seconds: 40.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    pub goose_white: String,
    pub goose_orange: String,
    pub goose_outline: String,
    /// Renderer-V2 tones (ADR 0014). Absent in pre-V2 config files: `None` derives a
    /// coherent tone from the legacy three so old files keep working.
    pub goose_shade: Option<String>,
    pub goose_wing: Option<String>,
    pub goose_orange_dark: Option<String>,
}

impl Default for ColorConfig {
    fn default() -> Self {
        let palette = RenderPalette::default();
        Self {
            goose_white: hex_string(palette.goose_white),
            goose_orange: hex_string(palette.goose_orange),
            goose_outline: hex_string(palette.goose_outline),
            goose_shade: Some(hex_string(palette.goose_shade)),
            goose_wing: Some(hex_string(palette.goose_wing)),
            goose_orange_dark: Some(hex_string(palette.goose_orange_dark)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SpeedConfig {
    pub walk_speed: f32,
    pub run_speed: f32,
    pub charge_speed: f32,
    pub acceleration_normal: f32,
    pub acceleration_charged: f32,
    pub step_time_normal: f32,
    pub step_time_charged: f32,
}

impl Default for SpeedConfig {
    fn default() -> Self {
        Self {
            walk_speed: 80.0,
            run_speed: 200.0,
            charge_speed: 400.0,
            acceleration_normal: 1300.0,
            acceleration_charged: 2300.0,
            step_time_normal: 0.2,
            step_time_charged: 0.1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MudConfig {
    pub duration_to_track_seconds: f32,
    pub footmark_lifetime_seconds: f32,
    pub footmark_shrink_seconds: f32,
}

impl Default for MudConfig {
    fn default() -> Self {
        Self {
            duration_to_track_seconds: 10.0,
            footmark_lifetime_seconds: 8.5,
            footmark_shrink_seconds: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MouseConfig {
    pub grab_distance: f32,
    pub drop_distance: f32,
    pub succ_time: f32,
}

impl Default for MouseConfig {
    fn default() -> Self {
        Self {
            grab_distance: 60.0,
            drop_distance: 200.0,
            succ_time: 2.5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct FutureBehaviorConfig {
    pub on_hour_double_honk: bool,
    pub multi_monitor_chase: bool,
}

impl Default for FutureBehaviorConfig {
    fn default() -> Self {
        Self {
            on_hour_double_honk: true,
            multi_monitor_chase: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MoodConfig {
    pub dynamic_moods: bool,
    pub mood_intensity: String,
}

impl Default for MoodConfig {
    fn default() -> Self {
        Self {
            dynamic_moods: true,
            mood_intensity: "normal".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct MischiefConfig {
    pub perch_and_ride: bool,
    pub collect_windows: bool,
    pub collect_notes: bool,
    pub collect_memes: bool,
}

impl Default for MischiefConfig {
    fn default() -> Self {
        Self {
            perch_and_ride: true,
            collect_windows: true,
            collect_notes: true,
            collect_memes: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct InteractionConfig {
    pub pat_streak: bool,
}

impl Default for InteractionConfig {
    fn default() -> Self {
        Self { pat_streak: true }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ScheduleConfig {
    pub quiet_hours_enabled: bool,
    pub quiet_start: String,
    pub quiet_end: String,
    pub dnd_respect: bool,
    pub seasonal: bool,
    pub autumn: bool,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            quiet_hours_enabled: true,
            quiet_start: "22:00".into(),
            quiet_end: "08:00".into(),
            dnd_respect: true,
            seasonal: false,
            autumn: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppearanceConfig {
    pub calm_goose: bool,
    pub expressions: bool,
    pub reduced_motion: bool,
}
impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            calm_goose: false,
            expressions: true,
            reduced_motion: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub enabled: bool,
    pub honk: bool,
    pub bite: bool,
    pub mud: bool,
    pub pat: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            honk: true,
            bite: true,
            mud: true,
            pat: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SafetyConfig {
    pub pause_on_fullscreen: bool,
    pub no_mouse_steal: bool,
    pub no_window_ride: bool,
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            pause_on_fullscreen: true,
            no_mouse_steal: false,
            no_window_ride: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct PlatformConfig {
    pub wayland: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CliOverrides {
    pub no_sound: bool,
    pub no_mouse_steal: bool,
    pub no_window_ride: bool,
    pub wayland: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendState {
    pub cursor_warp: BackendCapability,
    pub window_watch: BackendCapability,
    pub collect_window: BackendCapability,
    pub presence: BackendCapability,
    pub audio: BackendCapability,
    pub note_count: u32,
    pub meme_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendCapability {
    Supported,
    Unsupported,
    Denied,
    Failed,
}

impl BackendCapability {
    pub fn active(self) -> bool {
        matches!(self, Self::Supported)
    }
}

impl From<bool> for BackendCapability {
    fn from(supported: bool) -> Self {
        if supported {
            Self::Supported
        } else {
            Self::Unsupported
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EffectiveOptions {
    pub audio: AudioConfig,
    pub no_sound: bool,
    pub no_mouse_steal: bool,
    pub no_window_ride: bool,
    pub wayland: bool,
    pub world: WorldOptions,
}

#[derive(Debug, Clone)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: Config,
    pub warning: Option<String>,
    pub migrated_from: Option<u32>,
}

#[derive(Debug, Clone)]
pub enum ConfigLoadState {
    Missing { path: PathBuf },
    Loaded(Box<LoadedConfig>),
    Malformed { path: PathBuf, error: String },
    UnsupportedVersion { path: PathBuf, found: u32 },
}

#[derive(Debug)]
pub enum ConfigError {
    NoDefaultPath,
    Io(io::Error),
    Parse(toml::de::Error),
    MalformedDocument(String),
    WrongVersion(u32),
    Validation(Vec<String>),
    InvalidTarget(String),
    Conflict,
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Conflict => {
                f.write_str("settings changed in another editor; reload them before saving")
            }
            Self::NoDefaultPath => f.write_str("could not determine a honk300 config path"),
            Self::Io(err) => write!(f, "config I/O error: {err}"),
            Self::Parse(err) => write!(f, "malformed config.toml: {err}"),
            Self::MalformedDocument(err) => write!(f, "malformed config.toml: {err}"),
            Self::WrongVersion(version) => {
                write!(f, "unsupported goose_config_version {version}")
            }
            Self::Validation(errors) => write!(f, "invalid config: {}", errors.join("; ")),
            Self::InvalidTarget(message) => write!(f, "invalid config target: {message}"),
        }
    }
}

impl std::error::Error for ConfigError {}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> Self {
        Self::Io(err)
    }
}

impl From<toml::de::Error> for ConfigError {
    fn from(err: toml::de::Error) -> Self {
        Self::Parse(err)
    }
}

impl Config {
    pub fn load(path: Option<PathBuf>) -> Result<ConfigLoadState, ConfigError> {
        let path = resolve_path(path)?;
        let path_is_symlink = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata.file_type().is_symlink(),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(ConfigLoadState::Missing { path });
            }
            Err(err) => return Err(ConfigError::Io(err)),
        };
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound && path_is_symlink => {
                return Ok(ConfigLoadState::Malformed {
                    path,
                    error: "config path is a dangling symlink".into(),
                });
            }
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(ConfigLoadState::Missing { path });
            }
            Err(err) => return Err(ConfigError::Io(err)),
        };
        let mut doc = match text.parse::<DocumentMut>() {
            Ok(doc) => doc,
            Err(err) => {
                return Ok(ConfigLoadState::Malformed {
                    path,
                    error: err.to_string(),
                });
            }
        };

        let found_version = match document_version(&doc) {
            Ok(version) => version.unwrap_or(CONFIG_VERSION),
            Err(error) => return Ok(ConfigLoadState::Malformed { path, error }),
        };
        if found_version != 1 && found_version != CONFIG_VERSION {
            return Ok(ConfigLoadState::UnsupportedVersion {
                path,
                found: found_version,
            });
        }

        let migrated_from = if found_version == 1 {
            if let Err(error) = migrate_v1_document(&mut doc) {
                return Ok(ConfigLoadState::Malformed { path, error });
            }
            Some(1)
        } else {
            None
        };
        let normalized = doc.to_string();
        let warning = unknown_key_warning(&normalized);
        let config: Self = match toml::from_str(&normalized) {
            Ok(config) => config,
            Err(err) => {
                return Ok(ConfigLoadState::Malformed {
                    path,
                    error: err.to_string(),
                });
            }
        };
        if let Err(err) = config.validate() {
            return Ok(ConfigLoadState::Malformed {
                path,
                error: err.to_string(),
            });
        }
        Ok(ConfigLoadState::Loaded(Box::new(LoadedConfig {
            path,
            config,
            warning,
            migrated_from,
        })))
    }

    pub fn load_or_default(path: Option<PathBuf>) -> Result<LoadedConfig, ConfigError> {
        match Self::load(path)? {
            ConfigLoadState::Missing { path } => Ok(LoadedConfig {
                path,
                config: Self::default(),
                warning: None,
                migrated_from: None,
            }),
            ConfigLoadState::Loaded(loaded) => Ok(*loaded),
            ConfigLoadState::Malformed { error, .. } => Err(ConfigError::MalformedDocument(error)),
            ConfigLoadState::UnsupportedVersion { found, .. } => {
                Err(ConfigError::WrongVersion(found))
            }
        }
    }

    pub fn load_existing(path: &Path) -> Result<Self, ConfigError> {
        Self::load_existing_with_warning(path).map(|loaded| loaded.config)
    }

    pub fn load_existing_with_warning(path: &Path) -> Result<LoadedConfig, ConfigError> {
        match Self::load(Some(path.to_path_buf()))? {
            ConfigLoadState::Loaded(loaded) => Ok(*loaded),
            ConfigLoadState::Missing { .. } => Err(ConfigError::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "config file does not exist",
            ))),
            ConfigLoadState::Malformed { error, .. } => Err(ConfigError::MalformedDocument(error)),
            ConfigLoadState::UnsupportedVersion { found, .. } => {
                Err(ConfigError::WrongVersion(found))
            }
        }
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        let mut errors = Vec::new();
        if self.goose_config_version != CONFIG_VERSION {
            return Err(ConfigError::WrongVersion(self.goose_config_version));
        }
        positive(
            "behavior.first_wander_time_seconds",
            self.behavior.first_wander_time_seconds,
            &mut errors,
        );
        positive(
            "behavior.min_wandering_time_seconds",
            self.behavior.min_wandering_time_seconds,
            &mut errors,
        );
        positive(
            "behavior.max_wandering_time_seconds",
            self.behavior.max_wandering_time_seconds,
            &mut errors,
        );
        if self.behavior.max_wandering_time_seconds < self.behavior.min_wandering_time_seconds {
            errors.push(
                "behavior.max_wandering_time_seconds must be >= min_wandering_time_seconds".into(),
            );
        }
        positive("mouse.grab_distance", self.mouse.grab_distance, &mut errors);
        positive("mouse.drop_distance", self.mouse.drop_distance, &mut errors);
        positive("mouse.succ_time", self.mouse.succ_time, &mut errors);
        positive("speeds.walk_speed", self.speeds.walk_speed, &mut errors);
        positive("speeds.run_speed", self.speeds.run_speed, &mut errors);
        positive("speeds.charge_speed", self.speeds.charge_speed, &mut errors);
        positive(
            "speeds.acceleration_normal",
            self.speeds.acceleration_normal,
            &mut errors,
        );
        positive(
            "speeds.acceleration_charged",
            self.speeds.acceleration_charged,
            &mut errors,
        );
        positive(
            "speeds.step_time_normal",
            self.speeds.step_time_normal,
            &mut errors,
        );
        positive(
            "speeds.step_time_charged",
            self.speeds.step_time_charged,
            &mut errors,
        );
        positive(
            "mud.duration_to_track_seconds",
            self.mud.duration_to_track_seconds,
            &mut errors,
        );
        positive(
            "mud.footmark_lifetime_seconds",
            self.mud.footmark_lifetime_seconds,
            &mut errors,
        );
        positive(
            "mud.footmark_shrink_seconds",
            self.mud.footmark_shrink_seconds,
            &mut errors,
        );
        if self.mud.footmark_shrink_seconds > self.mud.footmark_lifetime_seconds {
            errors.push("mud.footmark_shrink_seconds must be <= footmark_lifetime_seconds".into());
        }
        validate_hex_color("colors.goose_white", &self.colors.goose_white, &mut errors);
        validate_hex_color(
            "colors.goose_orange",
            &self.colors.goose_orange,
            &mut errors,
        );
        if let Some(v) = &self.colors.goose_shade {
            validate_hex_color("colors.goose_shade", v, &mut errors);
        }
        if let Some(v) = &self.colors.goose_wing {
            validate_hex_color("colors.goose_wing", v, &mut errors);
        }
        if let Some(v) = &self.colors.goose_orange_dark {
            validate_hex_color("colors.goose_orange_dark", v, &mut errors);
        }
        validate_hex_color(
            "colors.goose_outline",
            &self.colors.goose_outline,
            &mut errors,
        );
        validate_time(
            "schedule.quiet_start",
            &self.schedule.quiet_start,
            &mut errors,
        );
        validate_time("schedule.quiet_end", &self.schedule.quiet_end, &mut errors);
        if !matches!(
            self.moods.mood_intensity.as_str(),
            "calm" | "normal" | "spicy"
        ) {
            errors.push("moods.mood_intensity must be calm, normal, or spicy".into());
        }
        if errors.is_empty() {
            Ok(())
        } else {
            Err(ConfigError::Validation(errors))
        }
    }

    pub fn save_atomic(&self, path: &Path) -> Result<(), ConfigError> {
        editing::save(self, path, None).map(|_| ())
    }

    pub fn save_if_revision(
        &self,
        path: &Path,
        expected: &ConfigRevision,
    ) -> Result<ConfigRevision, ConfigError> {
        editing::save(self, path, Some(expected))
    }

    pub fn restart_required_changes(&self, next: &Self) -> Vec<&'static str> {
        let mut changes = Vec::new();
        if self.behaviors.multi_monitor_chase != next.behaviors.multi_monitor_chase {
            changes.push("behaviors.multi_monitor_chase");
        }
        if self.platform.wayland != next.platform.wayland {
            changes.push("platform.wayland");
        }
        changes
    }

    pub fn effective_options(&self, backend: BackendState, cli: CliOverrides) -> EffectiveOptions {
        let no_sound = cli.no_sound || !self.audio.enabled;
        let no_mouse_steal = cli.no_mouse_steal || self.safety.no_mouse_steal;
        let no_window_ride = cli.no_window_ride || self.safety.no_window_ride;
        let default_schedule = ScheduleOptions::default();

        let mut foreign_window = ForeignWindowOptions::with_backend_support(
            backend.window_watch.active(),
            !no_window_ride,
        );
        foreign_window.enabled = self.mischief.perch_and_ride && !no_window_ride;

        // Backend capability gates every collect operation: if the runtime reported it can no
        // longer drive collect windows, none of these are available regardless of config. This
        // keeps a runtime capability loss durable across reloads instead of being reset by one.
        let collect_supported = backend.collect_window.active();
        let collect_capabilities = CollectWindowCapabilities {
            spawn_note: collect_supported,
            spawn_image: collect_supported,
            move_window: collect_supported && self.mischief.collect_windows,
            set_passthrough: collect_supported,
            synthesize_text: collect_supported,
        };
        let mut collect_window = CollectWindowOptions::with_backend_support(
            collect_capabilities,
            backend.note_count,
            backend.meme_count,
        );
        collect_window.enabled = self.mischief.collect_windows;
        collect_window.notes_enabled = self.mischief.collect_notes;
        collect_window.memes_enabled = self.mischief.collect_memes;

        let world = WorldOptions {
            multi_monitor_chase: self.behaviors.multi_monitor_chase,
            mouse_steal: MouseStealOptions {
                enabled: self.behavior.can_attack_mouse && !no_mouse_steal,
                warp_supported: backend.cursor_warp.active(),
                grab_distance: self.mouse.grab_distance,
                drop_distance: self.mouse.drop_distance,
                succ_time: self.mouse.succ_time,
                attack_randomly: self.behavior.attack_randomly,
            },
            foreign_window,
            collect_window,
            interaction: InteractionOptions {
                pat_streak: self.interaction.pat_streak,
            },
            appearance: AppearanceOptions {
                calm_goose: self.appearance.calm_goose,
                expressions: self.appearance.expressions,
                reduced_motion: self.appearance.reduced_motion,
            },
            timing: TimingOptions {
                first_wander_time: self.behavior.first_wander_time_seconds,
                min_wandering_time: self.behavior.min_wandering_time_seconds,
                max_wandering_time: self.behavior.max_wandering_time_seconds,
                // Excursion/puddle cadences ride the engine defaults (ADR 0016); not
                // yet config-exposed.
                ..TimingOptions::default()
            },
            parameters: ParametersTable {
                walk_speed: self.speeds.walk_speed,
                run_speed: self.speeds.run_speed,
                charge_speed: self.speeds.charge_speed,
                acceleration_normal: self.speeds.acceleration_normal,
                acceleration_charged: self.speeds.acceleration_charged,
                stop_radius: ParametersTable::default().stop_radius,
                step_time_normal: self.speeds.step_time_normal,
                step_time_charged: self.speeds.step_time_charged,
                duration_to_track_mud: self.mud.duration_to_track_seconds,
            },
            footmarks: FootMarkTiming {
                lifetime: self.mud.footmark_lifetime_seconds,
                shrink_time: self.mud.footmark_shrink_seconds,
            },
            palette: if self.behavior.use_custom_colors {
                let d = RenderPalette::default();
                let white = parse_hex_rgb(&self.colors.goose_white).unwrap_or(d.goose_white);
                let orange = parse_hex_rgb(&self.colors.goose_orange).unwrap_or(d.goose_orange);
                let outline = parse_hex_rgb(&self.colors.goose_outline).unwrap_or(d.goose_outline);
                // Derive the V2 tones from the legacy three, then let explicit keys win.
                let mut palette = RenderPalette::from_legacy(white, orange, outline);
                let explicit = |v: &Option<String>| v.as_deref().and_then(parse_hex_rgb);
                if let Some(c) = explicit(&self.colors.goose_shade) {
                    palette.goose_shade = c;
                }
                if let Some(c) = explicit(&self.colors.goose_wing) {
                    palette.goose_wing = c;
                }
                if let Some(c) = explicit(&self.colors.goose_orange_dark) {
                    palette.goose_orange_dark = c;
                }
                palette
            } else {
                RenderPalette::default()
            },
            mood: MoodOptions {
                dynamic_moods: self.moods.dynamic_moods,
                intensity: parse_mood_intensity(&self.moods.mood_intensity),
            },
            hourly_honk: HourlyHonkOptions {
                on_hour_double_honk: self.behaviors.on_hour_double_honk,
            },
            schedule: ScheduleOptions {
                quiet_hours_enabled: self.schedule.quiet_hours_enabled,
                quiet_start: parse_local_minute(&self.schedule.quiet_start)
                    .unwrap_or(default_schedule.quiet_start),
                quiet_end: parse_local_minute(&self.schedule.quiet_end)
                    .unwrap_or(default_schedule.quiet_end),
                dnd_respect: self.schedule.dnd_respect,
                pause_on_fullscreen: self.safety.pause_on_fullscreen,
                seasonal: self.schedule.seasonal,
                autumn: self.schedule.autumn,
            },
        };

        EffectiveOptions {
            audio: self.audio,
            no_sound,
            no_mouse_steal,
            no_window_ride,
            wayland: cli.wayland || self.platform.wayland,
            world,
        }
    }

    fn write_to_document(&self, doc: &mut DocumentMut) {
        doc["goose_config_version"] = value(self.goose_config_version as i64);
        let behavior = table_mut(doc, "behavior");
        behavior.remove("silence_sounds");
        set_bool(behavior, "can_attack_mouse", self.behavior.can_attack_mouse);
        set_bool(behavior, "attack_randomly", self.behavior.attack_randomly);
        set_bool(
            behavior,
            "use_custom_colors",
            self.behavior.use_custom_colors,
        );
        set_float(
            behavior,
            "first_wander_time_seconds",
            self.behavior.first_wander_time_seconds,
        );
        set_float(
            behavior,
            "min_wandering_time_seconds",
            self.behavior.min_wandering_time_seconds,
        );
        set_float(
            behavior,
            "max_wandering_time_seconds",
            self.behavior.max_wandering_time_seconds,
        );

        let colors = table_mut(doc, "colors");
        set_str(colors, "goose_white", &self.colors.goose_white);
        set_str(colors, "goose_orange", &self.colors.goose_orange);
        set_str(colors, "goose_outline", &self.colors.goose_outline);
        if let Some(v) = &self.colors.goose_shade {
            set_str(colors, "goose_shade", v);
        }
        if let Some(v) = &self.colors.goose_wing {
            set_str(colors, "goose_wing", v);
        }
        if let Some(v) = &self.colors.goose_orange_dark {
            set_str(colors, "goose_orange_dark", v);
        }

        let speeds = table_mut(doc, "speeds");
        set_float(speeds, "walk_speed", self.speeds.walk_speed);
        set_float(speeds, "run_speed", self.speeds.run_speed);
        set_float(speeds, "charge_speed", self.speeds.charge_speed);
        set_float(
            speeds,
            "acceleration_normal",
            self.speeds.acceleration_normal,
        );
        set_float(
            speeds,
            "acceleration_charged",
            self.speeds.acceleration_charged,
        );
        set_float(speeds, "step_time_normal", self.speeds.step_time_normal);
        set_float(speeds, "step_time_charged", self.speeds.step_time_charged);
        speeds.remove("stop_radius");

        let mud = table_mut(doc, "mud");
        set_float(
            mud,
            "duration_to_track_seconds",
            self.mud.duration_to_track_seconds,
        );
        set_float(
            mud,
            "footmark_lifetime_seconds",
            self.mud.footmark_lifetime_seconds,
        );
        set_float(
            mud,
            "footmark_shrink_seconds",
            self.mud.footmark_shrink_seconds,
        );

        let mouse = table_mut(doc, "mouse");
        set_float(mouse, "grab_distance", self.mouse.grab_distance);
        set_float(mouse, "drop_distance", self.mouse.drop_distance);
        set_float(mouse, "succ_time", self.mouse.succ_time);

        let behaviors = table_mut(doc, "behaviors");
        set_bool(
            behaviors,
            "on_hour_double_honk",
            self.behaviors.on_hour_double_honk,
        );
        set_bool(
            behaviors,
            "multi_monitor_chase",
            self.behaviors.multi_monitor_chase,
        );

        let moods = table_mut(doc, "moods");
        set_bool(moods, "dynamic_moods", self.moods.dynamic_moods);
        set_str(moods, "mood_intensity", &self.moods.mood_intensity);

        let mischief = table_mut(doc, "mischief");
        set_bool(mischief, "perch_and_ride", self.mischief.perch_and_ride);
        set_bool(mischief, "collect_windows", self.mischief.collect_windows);
        set_bool(mischief, "collect_notes", self.mischief.collect_notes);
        set_bool(mischief, "collect_memes", self.mischief.collect_memes);

        let interaction = table_mut(doc, "interaction");
        set_bool(interaction, "pat_streak", self.interaction.pat_streak);

        let schedule = table_mut(doc, "schedule");
        set_bool(
            schedule,
            "quiet_hours_enabled",
            self.schedule.quiet_hours_enabled,
        );
        set_str(schedule, "quiet_start", &self.schedule.quiet_start);
        set_str(schedule, "quiet_end", &self.schedule.quiet_end);
        set_bool(schedule, "dnd_respect", self.schedule.dnd_respect);
        set_bool(schedule, "seasonal", self.schedule.seasonal);
        set_bool(schedule, "autumn", self.schedule.autumn);

        let appearance = table_mut(doc, "appearance");
        set_bool(appearance, "calm_goose", self.appearance.calm_goose);
        set_bool(appearance, "expressions", self.appearance.expressions);
        set_bool(appearance, "reduced_motion", self.appearance.reduced_motion);

        let audio = table_mut(doc, "audio");
        set_bool(audio, "enabled", self.audio.enabled);
        set_bool(audio, "honk", self.audio.honk);
        set_bool(audio, "bite", self.audio.bite);
        set_bool(audio, "mud", self.audio.mud);
        set_bool(audio, "pat", self.audio.pat);

        let safety = table_mut(doc, "safety");
        set_bool(
            safety,
            "pause_on_fullscreen",
            self.safety.pause_on_fullscreen,
        );
        set_bool(safety, "no_mouse_steal", self.safety.no_mouse_steal);
        set_bool(safety, "no_window_ride", self.safety.no_window_ride);

        let platform = table_mut(doc, "platform");
        set_bool(platform, "wayland", self.platform.wayland);

        let lifecycle = table_mut(doc, "lifecycle");
        set_bool(
            lifecycle,
            "autostart_on_login",
            self.lifecycle.autostart_on_login,
        );

        // Only materialize [remote] once someone has configured it, so ordinary saves do not
        // add an empty remote-control section to every config file.
        if self.remote != RemoteConfig::default() || doc.as_table().contains_key("remote") {
            let remote = table_mut(doc, "remote");
            set_bool(remote, "enabled", self.remote.enabled);
            remote["relay_url"] = value(self.remote.relay_url.as_str());
            remote["token"] = value(self.remote.token.as_str());
            remote["device_name"] = value(self.remote.device_name.as_str());
        }
    }
}

pub fn default_config_path() -> Option<PathBuf> {
    if cfg!(windows) {
        return Some(
            PathBuf::from(std::env::var_os("LOCALAPPDATA")?)
                .join("honk300")
                .join("config.toml"),
        );
    }
    if cfg!(target_os = "macos") {
        return Some(
            PathBuf::from(std::env::var_os("HOME")?)
                .join("Library")
                .join("Application Support")
                .join("honk300")
                .join("config.toml"),
        );
    }
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share"))
        })?;
    Some(base.join("honk300").join("config.toml"))
}

pub fn resolve_path(path: Option<PathBuf>) -> Result<PathBuf, ConfigError> {
    match path {
        Some(path) => Ok(path),
        None => default_config_path().ok_or(ConfigError::NoDefaultPath),
    }
}

/// Explicitly replace a config with schema-current defaults. Existing bytes are copied to a
/// timestamped sibling first, including malformed and future-version configs.
pub fn reset_to_defaults(path: &Path) -> Result<Option<PathBuf>, ConfigError> {
    let existed = match fs::symlink_metadata(path) {
        Ok(_) => true,
        Err(err) if err.kind() == io::ErrorKind::NotFound => false,
        Err(err) => return Err(ConfigError::Io(err)),
    };
    let target = save_target(path)?;
    let backup = if existed {
        let backup = next_backup_path(path)?;
        fs::copy(path, &backup)?;
        Some(backup)
    } else {
        None
    };

    let config = Config::default();
    config.validate()?;
    let mut doc = DocumentMut::new();
    config.write_to_document(&mut doc);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    persist_document(&target, &doc)?;
    Ok(backup)
}

fn next_backup_path(path: &Path) -> Result<PathBuf, ConfigError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|err| ConfigError::InvalidTarget(format!("system clock is before epoch: {err}")))?
        .as_millis();
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("config.toml");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    for suffix in 0u16..=u16::MAX {
        let suffix = if suffix == 0 {
            String::new()
        } else {
            format!("-{suffix}")
        };
        let candidate = parent.join(format!("{file_name}.backup-{timestamp}{suffix}"));
        match fs::symlink_metadata(&candidate) {
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(candidate),
            Ok(_) => {}
            Err(err) => return Err(ConfigError::Io(err)),
        }
    }
    Err(ConfigError::InvalidTarget(
        "could not choose an unused config backup name".into(),
    ))
}

fn persist_document(target: &Path, doc: &DocumentMut) -> Result<(), ConfigError> {
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = NamedTempFile::new_in(parent)?;
    tmp.write_all(doc.to_string().as_bytes())?;
    tmp.flush()?;
    tmp.as_file().sync_all()?;
    tmp.persist(target)
        .map_err(|err| ConfigError::Io(err.error))?;
    Ok(())
}

fn document_version(doc: &DocumentMut) -> Result<Option<u32>, String> {
    let Some(item) = doc.as_table().get("goose_config_version") else {
        return Ok(None);
    };
    let Some(version) = item.as_integer() else {
        return Err("goose_config_version must be a non-negative integer".into());
    };
    u32::try_from(version)
        .map(Some)
        .map_err(|_| "goose_config_version is outside the supported integer range".into())
}

fn migrate_v1_document(doc: &mut DocumentMut) -> Result<(), String> {
    let silence_sounds = doc
        .get("behavior")
        .and_then(Item::as_table)
        .and_then(|table| table.get("silence_sounds"))
        .map(|item| {
            item.as_bool()
                .ok_or_else(|| "behavior.silence_sounds must be a boolean".to_string())
        })
        .transpose()?
        .unwrap_or(false);
    let audio_enabled = doc
        .get("audio")
        .and_then(Item::as_table)
        .and_then(|table| table.get("enabled"))
        .map(|item| {
            item.as_bool()
                .ok_or_else(|| "audio.enabled must be a boolean".to_string())
        })
        .transpose()?
        .unwrap_or(true);

    doc["goose_config_version"] = value(CONFIG_VERSION as i64);
    table_mut(doc, "behavior").remove("silence_sounds");
    table_mut(doc, "speeds").remove("stop_radius");
    table_mut(doc, "audio")["enabled"] = value(audio_enabled && !silence_sounds);
    Ok(())
}

fn save_target(path: &Path) -> Result<PathBuf, ConfigError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            let linked = fs::read_link(path)?;
            let target = if linked.is_absolute() {
                linked
            } else {
                path.parent().unwrap_or_else(|| Path::new(".")).join(linked)
            };
            let metadata = fs::symlink_metadata(&target).map_err(|err| {
                ConfigError::InvalidTarget(format!(
                    "{} points to an unavailable target ({err})",
                    path.display()
                ))
            })?;
            if !metadata.file_type().is_file() {
                return Err(ConfigError::InvalidTarget(format!(
                    "{} must point directly to a regular file",
                    path.display()
                )));
            }
            Ok(target)
        }
        Ok(metadata) if metadata.file_type().is_file() => Ok(path.to_path_buf()),
        Ok(_) => Err(ConfigError::InvalidTarget(format!(
            "{} is not a regular file",
            path.display()
        ))),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(path.to_path_buf()),
        Err(err) => Err(ConfigError::Io(err)),
    }
}

fn positive(name: &str, value: f32, errors: &mut Vec<String>) {
    if !value.is_finite() || value <= 0.0 {
        errors.push(format!("{name} must be a positive finite number"));
    }
}

fn validate_hex_color(name: &str, value: &str, errors: &mut Vec<String>) {
    if parse_hex_rgb(value).is_none() {
        errors.push(format!("{name} must be a #rrggbb hex color"));
    }
}

fn parse_hex_rgb(value: &str) -> Option<(u8, u8, u8)> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

fn parse_mood_intensity(value: &str) -> MoodIntensity {
    match value {
        "calm" => MoodIntensity::Calm,
        "spicy" => MoodIntensity::Spicy,
        _ => MoodIntensity::Normal,
    }
}

fn parse_local_minute(value: &str) -> Option<LocalMinute> {
    let (hour, minute) = value.split_once(':')?;
    let hour = hour.parse::<u8>().ok()?;
    let minute = minute.parse::<u8>().ok()?;
    LocalMinute::new(hour, minute)
}

fn validate_time(name: &str, value: &str, errors: &mut Vec<String>) {
    let Some((hour, minute)) = value.split_once(':') else {
        errors.push(format!("{name} must use HH:MM"));
        return;
    };
    let Ok(hour) = hour.parse::<u8>() else {
        errors.push(format!("{name} hour is invalid"));
        return;
    };
    let Ok(minute) = minute.parse::<u8>() else {
        errors.push(format!("{name} minute is invalid"));
        return;
    };
    if hour > 23 || minute > 59 {
        errors.push(format!("{name} must be within 00:00 through 23:59"));
    }
}

fn unknown_key_warning(text: &str) -> Option<String> {
    let doc = text.parse::<DocumentMut>().ok()?;
    let mut unknown = Vec::new();
    let root_allowed = [
        "goose_config_version",
        "behavior",
        "colors",
        "speeds",
        "mud",
        "mouse",
        "behaviors",
        "moods",
        "mischief",
        "interaction",
        "schedule",
        "appearance",
        "audio",
        "safety",
        "platform",
        "lifecycle",
        "remote",
    ];
    for (key, item) in doc.as_table().iter() {
        if !root_allowed.contains(&key) {
            unknown.push(key.to_string());
            continue;
        }
        let Some(table) = item.as_table() else {
            continue;
        };
        let allowed = known_section_keys(key);
        for (child, _) in table.iter() {
            if !allowed.contains(&child) {
                unknown.push(format!("{key}.{child}"));
            }
        }
    }
    if unknown.is_empty() {
        None
    } else {
        unknown.sort();
        unknown.dedup();
        Some(format!(
            "ignored unknown config key{}: {}",
            if unknown.len() == 1 { "" } else { "s" },
            unknown.join(", ")
        ))
    }
}

fn known_section_keys(section: &str) -> &'static [&'static str] {
    match section {
        "behavior" => &[
            "can_attack_mouse",
            "attack_randomly",
            "use_custom_colors",
            "first_wander_time_seconds",
            "min_wandering_time_seconds",
            "max_wandering_time_seconds",
        ],
        "colors" => &[
            "goose_white",
            "goose_orange",
            "goose_outline",
            "goose_shade",
            "goose_wing",
            "goose_orange_dark",
        ],
        "speeds" => &[
            "walk_speed",
            "run_speed",
            "charge_speed",
            "acceleration_normal",
            "acceleration_charged",
            "step_time_normal",
            "step_time_charged",
        ],
        "mud" => &[
            "duration_to_track_seconds",
            "footmark_lifetime_seconds",
            "footmark_shrink_seconds",
        ],
        "mouse" => &["grab_distance", "drop_distance", "succ_time"],
        "behaviors" => &["on_hour_double_honk", "multi_monitor_chase"],
        "moods" => &["dynamic_moods", "mood_intensity"],
        "mischief" => &[
            "perch_and_ride",
            "collect_windows",
            "collect_notes",
            "collect_memes",
        ],
        "interaction" => &["pat_streak"],
        "schedule" => &[
            "quiet_hours_enabled",
            "quiet_start",
            "quiet_end",
            "dnd_respect",
            "seasonal",
            "autumn",
        ],
        "appearance" => &["calm_goose", "expressions", "reduced_motion"],
        "audio" => &["enabled", "honk", "bite", "mud", "pat"],
        "safety" => &["pause_on_fullscreen", "no_mouse_steal", "no_window_ride"],
        "platform" => &["wayland"],
        "lifecycle" => &["autostart_on_login"],
        "remote" => &["enabled", "relay_url", "token", "device_name"],
        _ => &[],
    }
}

fn table_mut<'a>(doc: &'a mut DocumentMut, name: &str) -> &'a mut Table {
    if !doc.as_table().contains_key(name) || !doc[name].is_table() {
        doc[name] = Item::Table(Table::new());
    }
    doc[name].as_table_mut().expect("table was just installed")
}

fn set_bool(table: &mut Table, key: &str, v: bool) {
    table[key] = value(v);
}

fn set_float(table: &mut Table, key: &str, v: f32) {
    // Parsing the shortest round-trippable f32 string back to f64 avoids exposing the
    // binary-tail digits produced by a direct `f32 as f64` conversion.
    let stable = v
        .to_string()
        .parse::<f64>()
        .expect("a finite f32 always has a finite decimal representation");
    table[key] = value(stable);
}

fn set_str(table: &mut Table, key: &str, v: &str) {
    table[key] = value(v);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn backend() -> BackendState {
        BackendState {
            cursor_warp: BackendCapability::Supported,
            window_watch: BackendCapability::Supported,
            collect_window: BackendCapability::Supported,
            presence: BackendCapability::Supported,
            audio: BackendCapability::Supported,
            note_count: 1,
            meme_count: 1,
        }
    }

    #[test]
    fn backend_collect_loss_disables_collect_window() {
        // When the backend reports it can no longer spawn/move collect windows, that capability
        // loss must survive into the effective options (and therefore across a reload), even
        // though the user's config still enables collect behavior.
        let mut c = Config::default();
        c.mischief.collect_windows = true;
        c.mischief.collect_notes = true;
        c.mischief.collect_memes = true;

        let mut backend = backend();
        backend.collect_window = BackendCapability::Failed;

        let effective = c.effective_options(backend, CliOverrides::default());
        assert!(
            !effective.world.collect_window.active(),
            "a backend collect-window capability loss must disable collect behavior"
        );
    }

    #[test]
    fn partial_toml_keeps_defaults_for_missing_fields() {
        let c: Config = toml::from_str("[audio]\nenabled = false\n").unwrap();
        assert_eq!(c.goose_config_version, CONFIG_VERSION);
        assert!(!c.audio.enabled);
        assert!(c.audio.honk);
        assert_eq!(c.mouse.grab_distance, 60.0);
        assert_eq!(c.mud.duration_to_track_seconds, 10.0);
        assert!(!c.lifecycle.autostart_on_login);
        assert!(!c.schedule.seasonal);
    }

    #[test]
    fn seasonal_effects_default_off_and_preserve_an_existing_choice() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        for saved in [
            "[schedule]\nautumn = true\n",
            "[schedule]\nseasonal = true\n",
        ] {
            fs::write(&path, saved).unwrap();
            let config = Config::load_existing(&path).unwrap();
            let enabled = saved.contains("seasonal = true");
            assert_eq!(config.schedule.seasonal, enabled);
            assert_eq!(
                config
                    .effective_options(backend(), CliOverrides::default())
                    .world
                    .schedule
                    .seasonal,
                enabled
            );
            config.save_atomic(&path).unwrap();
            assert_eq!(
                Config::load_existing(&path).unwrap().schedule.seasonal,
                enabled
            );
        }
    }

    #[test]
    fn explicit_existing_mud_duration_survives_round_trip_and_reaches_world_options() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            format!(
                "goose_config_version = {CONFIG_VERSION}\n\
                 [mud]\n\
                 duration_to_track_seconds = 15.0\n"
            ),
        )
        .unwrap();

        let config = Config::load_existing(&path).unwrap();
        assert_eq!(config.mud.duration_to_track_seconds, 15.0);
        assert_eq!(
            config
                .effective_options(backend(), CliOverrides::default())
                .world
                .parameters
                .duration_to_track_mud,
            15.0
        );

        config.save_atomic(&path).unwrap();
        let reloaded = Config::load_existing(&path).unwrap();
        assert_eq!(reloaded.mud.duration_to_track_seconds, 15.0);
    }

    #[test]
    fn login_autostart_preference_round_trips_in_the_lifecycle_section() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = Config::default();
        config.lifecycle.autostart_on_login = true;

        config.save_atomic(&path).unwrap();

        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("[lifecycle]"));
        assert!(saved.contains("autostart_on_login = true"));
        assert!(
            Config::load_existing(&path)
                .unwrap()
                .lifecycle
                .autostart_on_login
        );
    }

    #[test]
    fn existing_appearance_defaults_expressions_on_without_rewriting_saved_options() {
        let config: Config = toml::from_str("[appearance]\ncalm_goose = true\n").unwrap();
        assert!(config.appearance.calm_goose && config.appearance.expressions);
        assert!(!config.appearance.reduced_motion);
    }

    #[test]
    fn expression_options_persist_with_unknown_fields_and_comments() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(
            &path,
            "# personal settings\n[appearance]\nfuture = 3 # retained\n",
        )
        .unwrap();
        let mut config = Config::load_existing(&path).unwrap();
        config.appearance.expressions = false;
        config.appearance.reduced_motion = true;
        config.save_atomic(&path).unwrap();
        let saved = fs::read_to_string(&path).unwrap();
        assert!(saved.contains("# personal settings") && saved.contains("future = 3 # retained"));
        let loaded = Config::load_existing(&path).unwrap();
        assert!(!loaded.appearance.expressions && loaded.appearance.reduced_motion);
    }

    #[test]
    fn remote_control_defaults_off_and_validates_only_when_enabled() {
        let mut config = Config::default();
        assert!(!config.remote.enabled);
        assert!(config.validate().is_ok());
        // A broken [remote] section disables remote control only; it never blocks the goose.
        config.remote.enabled = true;
        assert!(config.validate().is_ok());
        assert!(!config.remote.problems().is_empty());

        let mut remote = RemoteConfig {
            enabled: true,
            relay_url: "wss://goose.example.com/ws".into(),
            token: "0123456789abcdef".into(),
            device_name: "Office PC".into(),
        };
        assert!(remote.problems().is_empty());
        for url in [
            "ws://localhost:8080/ws",
            "ws://127.0.0.1:8080/ws",
            "ws://[::1]:8080/ws",
        ] {
            remote.relay_url = url.into();
            assert!(remote.problems().is_empty(), "{url}");
        }
        for url in [
            "ws://goose.example.com/ws",
            "https://goose.example.com",
            "wss://",
            "wss://user@goose.example.com",
            "",
        ] {
            remote.relay_url = url.into();
            assert_eq!(remote.problems().len(), 1, "{url}");
        }
        remote.relay_url = "wss://goose.example.com/ws".into();
        remote.token = "short".into();
        remote.device_name = "-bad/name".into();
        assert_eq!(remote.problems().len(), 2);
        remote.enabled = false;
        assert!(remote.problems().is_empty());
    }

    #[test]
    fn remote_section_round_trips_and_is_not_added_to_untouched_configs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        Config::default().save_atomic(&path).unwrap();
        assert!(!fs::read_to_string(&path).unwrap().contains("[remote]"));

        let config = Config {
            remote: RemoteConfig {
                enabled: true,
                relay_url: "wss://goose.example.com/ws".into(),
                token: "0123456789abcdef".into(),
                device_name: String::new(),
            },
            ..Config::default()
        };
        config.save_atomic(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("[remote]"));
        assert!(unknown_key_warning(&text).is_none());
        let loaded = Config::load_existing(&path).unwrap();
        assert_eq!(loaded.remote, config.remote);
    }

    #[test]
    fn unknown_keys_are_ignored_on_load() {
        let c: Config =
            toml::from_str("future_root = true\n[audio]\nfuture = 1\nenabled = false\n").unwrap();
        assert!(!c.audio.enabled);
    }

    #[test]
    fn unknown_keys_warn_on_load() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "future_root = true\n[audio]\nfuture = 1\n").unwrap();
        let loaded = Config::load_existing_with_warning(&path).unwrap();
        let warning = loaded.warning.expect("unknown keys should warn");
        assert!(warning.contains("future_root"));
        assert!(warning.contains("audio.future"));
    }

    #[test]
    fn wrong_version_is_rejected() {
        let c: Config = toml::from_str("goose_config_version = 99\n").unwrap();
        assert!(matches!(c.validate(), Err(ConfigError::WrongVersion(99))));
    }

    #[test]
    fn validation_catches_bad_ranges() {
        let c: Config = toml::from_str(&format!(
            "goose_config_version = {CONFIG_VERSION}\n[mouse]\ngrab_distance = -1.0\n"
        ))
        .unwrap();
        assert!(matches!(c.validate(), Err(ConfigError::Validation(_))));
    }

    #[test]
    fn validation_catches_bad_speed_mud_and_color_values() {
        let mut c = Config::default();
        c.speeds.walk_speed = 0.0;
        c.mud.footmark_lifetime_seconds = 1.0;
        c.mud.footmark_shrink_seconds = 2.0;
        c.colors.goose_white = "white".into();
        let Err(ConfigError::Validation(errors)) = c.validate() else {
            panic!("expected validation errors");
        };
        assert!(errors.iter().any(|e| e.contains("speeds.walk_speed")));
        assert!(errors.iter().any(|e| e.contains("footmark_shrink_seconds")));
        assert!(errors.iter().any(|e| e.contains("colors.goose_white")));
    }

    #[test]
    fn effective_options_merge_cli_and_config() {
        let mut c = Config::default();
        c.safety.no_mouse_steal = true;
        c.mischief.collect_memes = false;
        c.mouse.succ_time = 3.5;
        let effective = c.effective_options(backend(), CliOverrides::default());
        assert!(!effective.world.mouse_steal.enabled);
        assert_eq!(effective.world.mouse_steal.succ_time, 3.5);
        assert_eq!(effective.world.parameters.walk_speed, 80.0);
        assert_eq!(effective.world.footmarks.lifetime, 8.5);
        assert_eq!(effective.world.mood.intensity, MoodIntensity::Normal);
        assert!(effective.world.multi_monitor_chase);
        assert!(!effective.world.appearance.calm_goose);
        assert!(effective.world.hourly_honk.on_hour_double_honk);
        assert_eq!(effective.world.schedule.quiet_start.minutes(), 22 * 60);
        assert_eq!(effective.world.schedule.quiet_end.minutes(), 8 * 60);
        assert!(effective.world.schedule.dnd_respect);
        assert!(effective.world.schedule.pause_on_fullscreen);
        assert!(!effective.world.schedule.seasonal);
        assert!(effective.world.schedule.autumn);
        assert!(!effective
            .world
            .collect_window
            .kind_active(honk_engine::CollectWindowKind::Meme));
    }

    #[test]
    fn effective_options_maps_speed_mud_palette_mood_and_hourly_honk() {
        let mut c = Config::default();
        c.speeds.walk_speed = 91.0;
        c.mud.duration_to_track_seconds = 12.0;
        c.mud.footmark_lifetime_seconds = 6.0;
        c.mud.footmark_shrink_seconds = 2.0;
        c.behavior.use_custom_colors = true;
        c.colors.goose_white = "#ddeeff".into();
        c.colors.goose_orange = "#112233".into();
        c.colors.goose_outline = "#445566".into();
        c.moods.mood_intensity = "spicy".into();
        c.behaviors.on_hour_double_honk = false;
        c.behaviors.multi_monitor_chase = false;
        c.appearance.calm_goose = true;
        c.schedule.quiet_hours_enabled = false;
        c.schedule.quiet_start = "21:15".into();
        c.schedule.quiet_end = "06:45".into();
        c.schedule.dnd_respect = false;
        c.schedule.seasonal = false;
        c.schedule.autumn = false;
        c.safety.pause_on_fullscreen = false;

        let effective = c.effective_options(backend(), CliOverrides::default());
        assert_eq!(effective.world.parameters.walk_speed, 91.0);
        assert_eq!(effective.world.parameters.duration_to_track_mud, 12.0);
        assert_eq!(effective.world.footmarks.lifetime, 6.0);
        assert_eq!(effective.world.footmarks.shrink_time, 2.0);
        assert_eq!(effective.world.palette.goose_white, (0xdd, 0xee, 0xff));
        assert_eq!(effective.world.palette.goose_orange, (0x11, 0x22, 0x33));
        assert_eq!(effective.world.palette.goose_outline, (0x44, 0x55, 0x66));
        assert_eq!(effective.world.mood.intensity, MoodIntensity::Spicy);
        assert!(!effective.world.multi_monitor_chase);
        assert!(effective.world.appearance.calm_goose);
        assert!(!effective.world.hourly_honk.on_hour_double_honk);
        assert!(!effective.world.schedule.quiet_hours_enabled);
        assert_eq!(effective.world.schedule.quiet_start.minutes(), 21 * 60 + 15);
        assert_eq!(effective.world.schedule.quiet_end.minutes(), 6 * 60 + 45);
        assert!(!effective.world.schedule.dnd_respect);
        assert!(!effective.world.schedule.pause_on_fullscreen);
        assert!(!effective.world.schedule.seasonal);
        assert!(!effective.world.schedule.autumn);
    }

    #[test]
    fn parses_quiet_time_to_local_minutes() {
        assert_eq!(parse_local_minute("00:00").unwrap().minutes(), 0);
        assert_eq!(parse_local_minute("23:59").unwrap().minutes(), 23 * 60 + 59);
        assert!(parse_local_minute("24:00").is_none());
        assert!(parse_local_minute("12:60").is_none());
        assert!(parse_local_minute("nope").is_none());
    }

    #[test]
    fn save_preserves_unknown_keys_when_possible() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        fs::write(&path, "custom = 7\n[audio]\nunknown = true\n").unwrap();
        let mut c = Config::default();
        c.audio.enabled = false;
        c.save_atomic(&path).unwrap();
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("custom = 7"));
        assert!(text.contains("unknown = true"));
        assert!(text.contains("enabled = false"));
    }

    #[test]
    fn typed_load_states_distinguish_missing_malformed_and_newer_configs() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("missing.toml");
        assert!(matches!(
            Config::load(Some(missing.clone())).unwrap(),
            ConfigLoadState::Missing { path } if path == missing
        ));

        let malformed = dir.path().join("malformed.toml");
        fs::write(&malformed, "[audio\nenabled = true\n").unwrap();
        assert!(matches!(
            Config::load(Some(malformed.clone())).unwrap(),
            ConfigLoadState::Malformed { path, .. } if path == malformed
        ));

        let newer = dir.path().join("newer.toml");
        fs::write(&newer, "goose_config_version = 99\n").unwrap();
        assert!(matches!(
            Config::load(Some(newer.clone())).unwrap(),
            ConfigLoadState::UnsupportedVersion {
                path,
                found: 99,
            } if path == newer
        ));
    }

    #[test]
    fn v1_migration_combines_legacy_mute_controls_and_removes_retired_keys() {
        for (silence_sounds, audio_enabled) in [(true, true), (false, false)] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("config.toml");
            fs::write(
                &path,
                format!(
                    "goose_config_version = 1\nfuture_root = 'keep'\n\
                     [behavior]\nsilence_sounds = {silence_sounds}\n\
                     [audio]\nenabled = {audio_enabled}\nfuture_audio = true\n\
                     [speeds]\nstop_radius = 42.0\n"
                ),
            )
            .unwrap();

            let ConfigLoadState::Loaded(loaded) = Config::load(Some(path.clone())).unwrap() else {
                panic!("v1 config should migrate in memory");
            };
            assert_eq!(loaded.config.goose_config_version, CONFIG_VERSION);
            assert!(!loaded.config.audio.enabled);
            assert_eq!(loaded.migrated_from, Some(1));

            loaded.config.save_atomic(&path).unwrap();
            let saved = fs::read_to_string(&path).unwrap();
            assert!(!saved.contains("silence_sounds"));
            assert!(!saved.contains("stop_radius"));
            assert!(saved.contains("future_root = 'keep'"));
            assert!(saved.contains("future_audio = true"));
        }
    }

    #[test]
    fn save_refuses_to_replace_malformed_or_newer_existing_config() {
        let dir = tempfile::tempdir().unwrap();
        for (name, original) in [
            ("malformed.toml", "[audio\nenabled = true\n"),
            ("newer.toml", "goose_config_version = 99\nfuture = true\n"),
        ] {
            let path = dir.path().join(name);
            fs::write(&path, original).unwrap();
            assert!(Config::default().save_atomic(&path).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
    }

    #[test]
    fn explicit_reset_backs_up_existing_bytes_before_writing_defaults() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let original = "[broken\nvalue = true\n";
        fs::write(&path, original).unwrap();

        let backup = reset_to_defaults(&path)
            .unwrap()
            .expect("an existing config must be backed up");
        assert_ne!(backup, path);
        assert!(backup.parent() == path.parent());
        assert_eq!(fs::read_to_string(backup).unwrap(), original);
        assert_eq!(Config::load_existing(&path).unwrap(), Config::default());
    }

    #[test]
    fn explicit_reset_of_missing_path_materializes_valid_defaults_without_backup() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        assert_eq!(reset_to_defaults(&path).unwrap(), None);
        assert_eq!(Config::load_existing(&path).unwrap(), Config::default());
    }

    #[test]
    fn save_uses_stable_human_readable_float_values() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let mut config = Config::default();
        config.speeds.step_time_normal = 0.2;
        config.save_atomic(&path).unwrap();
        let saved = fs::read_to_string(path).unwrap();
        assert!(saved.contains("step_time_normal = 0.2\n"), "{saved}");
        assert!(!saved.contains("0.200000002"), "{saved}");
        assert!(!saved.contains("stop_radius"), "{saved}");
    }

    #[test]
    fn restart_required_changes_include_display_topology_and_backend_selection() {
        let current = Config::default();
        let mut next = current.clone();
        assert!(current.restart_required_changes(&next).is_empty());

        next.behaviors.multi_monitor_chase = !next.behaviors.multi_monitor_chase;
        next.platform.wayland = !next.platform.wayland;
        assert_eq!(
            current.restart_required_changes(&next),
            vec!["behaviors.multi_monitor_chase", "platform.wayland"]
        );
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn save_preserves_config_symlink_and_replaces_regular_file_target() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("target.toml");
        let link = dir.path().join("config.toml");
        fs::write(
            &target,
            "goose_config_version = 2\n[audio]\nenabled = true\n",
        )
        .unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&target, &link).unwrap();
        #[cfg(windows)]
        if !try_symlink_file(&target, &link) {
            return;
        }

        let mut config = Config::default();
        config.audio.enabled = false;
        config.save_atomic(&link).unwrap();

        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert!(!Config::load_existing(&target).unwrap().audio.enabled);
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn save_rejects_dangling_and_non_file_symlink_targets() {
        let dir = tempfile::tempdir().unwrap();
        let dangling = dir.path().join("dangling.toml");
        let missing_target = dir.path().join("missing.toml");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&missing_target, &dangling).unwrap();
        #[cfg(windows)]
        if !try_symlink_file(&missing_target, &dangling) {
            return;
        }
        assert!(Config::default().save_atomic(&dangling).is_err());
        assert!(fs::symlink_metadata(&dangling)
            .unwrap()
            .file_type()
            .is_symlink());

        let directory_target = dir.path().join("directory-target");
        fs::create_dir(&directory_target).unwrap();
        let directory_link = dir.path().join("directory.toml");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&directory_target, &directory_link).unwrap();
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&directory_target, &directory_link).unwrap();
        assert!(Config::default().save_atomic(&directory_link).is_err());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn dangling_symlink_is_not_reported_as_a_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let dangling = dir.path().join("config.toml");
        let missing_target = dir.path().join("missing.toml");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&missing_target, &dangling).unwrap();
        #[cfg(windows)]
        if !try_symlink_file(&missing_target, &dangling) {
            return;
        }

        assert!(matches!(
            Config::load(Some(dangling)).unwrap(),
            ConfigLoadState::Malformed { .. }
        ));
    }

    #[cfg(windows)]
    fn try_symlink_file(target: &Path, link: &Path) -> bool {
        match std::os::windows::fs::symlink_file(target, link) {
            Ok(()) => true,
            Err(err) if err.raw_os_error() == Some(1314) => {
                eprintln!("skipping symlink assertion: Windows symlink privilege unavailable");
                false
            }
            Err(err) => panic!("could not create test symlink: {err}"),
        }
    }
}
