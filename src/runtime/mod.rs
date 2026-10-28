#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
mod windows_profile;

#[cfg(windows)]
pub(crate) mod remote;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
mod macos_accessibility;

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(any(target_os = "linux", test))]
mod owned_props;

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
mod core;

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
pub(crate) mod control_surface;

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
use honk_config::BackendCapability;
#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
use honk_config::{CliOverrides, Config};

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
#[derive(Debug, Clone)]
pub struct RuntimeOptions {
    pub config_path: std::path::PathBuf,
    pub config: Config,
    pub cli_overrides: CliOverrides,
}

#[cfg(any(windows, target_os = "macos", target_os = "linux"))]
pub(crate) fn audio_probe_capability(available: bool) -> BackendCapability {
    if available {
        BackendCapability::Supported
    } else {
        BackendCapability::Failed
    }
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
pub(crate) fn backend_capability(status: honk_control::CapabilityStatus) -> BackendCapability {
    use honk_control::CapabilityStatus;
    match status {
        CapabilityStatus::Supported => BackendCapability::Supported,
        CapabilityStatus::Denied => BackendCapability::Denied,
        CapabilityStatus::Failed => BackendCapability::Failed,
        CapabilityStatus::Unprobed | CapabilityStatus::Unsupported => {
            BackendCapability::Unsupported
        }
    }
}

#[cfg(test)]
mod tests {
    use super::audio_probe_capability;
    use honk_config::BackendCapability;

    #[test]
    fn successful_audio_reprobe_recovers_failed_capability() {
        assert_eq!(audio_probe_capability(true), BackendCapability::Supported);
        assert_eq!(audio_probe_capability(false), BackendCapability::Failed);
    }
}
