//! Bounded, versioned stdio bridge. Rust owns settings and lifecycle decisions;
//! the native window owns only the draft and its presentation (ADR 0041).
mod launch;
mod schema;
use honk_config::{Config, ConfigError, ConfigRevision, ConfigSnapshot};
use honk_control::{send_command, ControlCommand, ControlResponse, RuntimeStatus};
pub(crate) use launch::launch;
use serde::Deserialize;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

const PROTOCOL: u32 = 1;
const MAX_REQUEST: u64 = 64 * 1024;
const MAX_RESPONSE: usize = 128 * 1024;
type Error = Box<dyn std::error::Error>;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    protocol: u32,
    request_id: u64,
    command: Operation,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Operation {
    Read {},
    Validate {
        revision: ConfigRevision,
        patch: BTreeMap<String, Value>,
    },
    Save {
        revision: ConfigRevision,
        patch: BTreeMap<String, Value>,
    },
    Status {},
    Start {},
    Stop {},
    CheckUpdates {},
    Update {},
    KdeSetup {},
    KdeRemove {},
    SwaySetup {},
    SwayRemove {},
    HyprlandSetup {},
    HyprlandRemove {},
    GnomeSetup {},
    GnomeRemove {},
    PointerRequest {},
    PointerCancel {},
}

pub(crate) fn run(path: Option<PathBuf>) -> Result<(), Error> {
    let path = honk_config::resolve_path(path)?;
    serve(io::stdin().lock(), io::stdout().lock(), &path)
}

fn serve(reader: impl Read, mut writer: impl Write, path: &Path) -> Result<(), Error> {
    let mut input = Vec::new();
    reader.take(MAX_REQUEST + 1).read_to_end(&mut input)?;
    let parsed = if input.len() as u64 > MAX_REQUEST {
        Err("settings request exceeds 64 KiB".to_owned())
    } else {
        serde_json::from_slice::<Request>(&input).map_err(|error| error.to_string())
    };
    let (id, result) = match parsed {
        Ok(request) if request.protocol == PROTOCOL => {
            (request.request_id, execute(request.command, path))
        }
        Ok(request) => (
            request.request_id,
            Err("unsupported settings protocol version".into()),
        ),
        Err(error) => (0, Err(error.into())),
    };
    let response = match result {
        Ok(data) => json!({"protocol": PROTOCOL, "request_id": id, "ok": true, "data": data}),
        Err(error) => {
            let code = match error.downcast_ref::<ConfigError>() {
                Some(ConfigError::Conflict) => "conflict",
                Some(ConfigError::Validation(_)) => "validation",
                _ => "failed",
            };
            json!({"protocol": PROTOCOL, "request_id": id, "ok": false, "error": {"code": code, "message": friendly_error(error.as_ref())}})
        }
    };
    let bytes = serde_json::to_vec(&response)?;
    if bytes.len() > MAX_RESPONSE {
        return Err("settings response exceeds 128 KiB".into());
    }
    writer.write_all(&bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}

fn friendly_error(error: &(dyn std::error::Error + 'static)) -> String {
    if let Some(ConfigError::Validation(errors)) = error.downcast_ref::<ConfigError>() {
        let mut message = errors.join("; ");
        for field in schema::FIELDS {
            message = message.replace(field.key, field.label);
        }
        format!("Invalid settings: {message}")
    } else {
        error.to_string()
    }
}

fn execute(operation: Operation, path: &Path) -> Result<Value, Error> {
    match operation {
        Operation::Read {} => read_settings(
            path,
            &crate::install::prepare_config_autostart,
            &runtime_json,
        ),
        Operation::Validate { revision, patch } => {
            let (before, config) = edited(path, &revision, patch)?;
            Ok(
                json!({"message": "Settings are valid.", "restart_required": restart_fields(&before.config, &config)}),
            )
        }
        Operation::Save { revision, patch } => save(path, revision, patch, &|config| {
            crate::install::reconcile_config_autostart(config.lifecycle.autostart_on_login)
                .map_err(|error| error.to_string())
        }),
        Operation::Status {} => Ok(
            json!({"runtime": runtime_json(), "integrations": crate::integrations::status(), "sway": crate::integrations::sway_status(), "hyprland": crate::integrations::hyprland_status(), "gnome": crate::integrations::gnome_status()}),
        ),
        Operation::KdeSetup {} => crate::integrations::setup(),
        Operation::KdeRemove {} => crate::integrations::remove(),
        Operation::SwaySetup {} => crate::integrations::sway_setup(),
        Operation::SwayRemove {} => crate::integrations::sway_remove(),
        Operation::HyprlandSetup {} => crate::integrations::hyprland_setup(),
        Operation::HyprlandRemove {} => crate::integrations::hyprland_remove(),
        Operation::GnomeSetup {} => crate::integrations::gnome_setup(),
        Operation::GnomeRemove {} => crate::integrations::gnome_remove(),
        Operation::PointerRequest {} => crate::integrations::pointer(true),
        Operation::PointerCancel {} => crate::integrations::pointer(false),
        Operation::Start {} => {
            let message = honk_config_tui::start_from_config(path)?;
            Ok(json!({"message": message, "runtime": runtime_json()}))
        }
        Operation::Stop {} => {
            match send_command(ControlCommand::Stop)? {
                ControlResponse::Ok => honk_control::wait_for_shutdown()?,
                response => return Err(format!("stop rejected: {response:?}").into()),
            }
            Ok(json!({"message": "Goose stopped.", "runtime": runtime_json()}))
        }
        Operation::CheckUpdates {} => Ok(json!({"updates": crate::update::check()?})),
        Operation::Update {} => {
            // The visible, independent helper owns the retained completion screen, lifecycle
            // lease, recovery and relaunch. Closing settings cannot kill its transaction.
            crate::runtime::control_surface::open_update_helper()?;
            Ok(json!({"message": "Updater opened. Its result will remain in the update window."}))
        }
    }
}

fn read_settings(
    path: &Path,
    prepare: &dyn Fn(&Path, &mut Config) -> Result<(), Error>,
    runtime: &dyn Fn() -> Value,
) -> Result<Value, Error> {
    let mut before = ConfigSnapshot::load(path)?;
    prepare(path, &mut before.config)?;
    // Installer intent may have updated the file. Return its resulting revision,
    // so an unrelated edit cannot turn an obsolete login choice into new intent.
    let snapshot = ConfigSnapshot::load(path)?;
    let mut value = snapshot_json(&snapshot);
    value["runtime"] = runtime();
    value["integrations"] = crate::integrations::status();
    value["sway"] = crate::integrations::sway_status();
    value["hyprland"] = crate::integrations::hyprland_status();
    value["gnome"] = crate::integrations::gnome_status();
    Ok(value)
}

fn snapshot_json(snapshot: &ConfigSnapshot) -> Value {
    json!({"version": env!("CARGO_PKG_VERSION"), "revision": snapshot.revision,
        "fields": schema::fields(&snapshot.config), "warning": snapshot.warning})
}

fn edited(
    path: &Path,
    revision: &ConfigRevision,
    patch: BTreeMap<String, Value>,
) -> Result<(ConfigSnapshot, Config), Error> {
    let before = ConfigSnapshot::load(path)?;
    if before.revision != *revision {
        return Err(ConfigError::Conflict.into());
    }
    let config = apply_patch(&before.config, patch)?;
    Ok((before, config))
}

fn apply_patch(config: &Config, patch: BTreeMap<String, Value>) -> Result<Config, Error> {
    if patch.len() > schema::FIELDS.len() {
        return Err("too many settings in patch".into());
    }
    let mut document = serde_json::to_value(config)?;
    for (key, value) in patch {
        if !schema::FIELDS.iter().any(|field| field.key == key) {
            return Err(format!("unknown setting: {key}").into());
        }
        let (section, name) = key.split_once('.').ok_or("invalid setting key")?;
        document[section][name] = value;
    }
    let config: Config = serde_json::from_value(document)?;
    config.validate()?;
    Ok(config)
}

fn restart_fields(before: &Config, after: &Config) -> Vec<&'static str> {
    if before.platform.wayland != after.platform.wayland {
        vec!["Use native Wayland"]
    } else {
        vec![]
    }
}

fn save(
    path: &Path,
    revision: ConfigRevision,
    patch: BTreeMap<String, Value>,
    reconcile: &dyn Fn(&Config) -> Result<(), String>,
) -> Result<Value, Error> {
    let (before, config) = edited(path, &revision, patch)?;
    let restart = restart_fields(&before.config, &config);
    let revision = config.save_if_revision(path, &revision)?;
    let mut warning = None;
    if config.lifecycle != before.config.lifecycle {
        warning = revision
            .with_guard(path, || {
                reconcile(&config).map_err(honk_config::ConfigError::InvalidTarget)
            })
            .err()
            .map(|error| error.to_string());
    }
    let identity = revision.reload_token(path)?;
    let (apply_state, message) = match send_command(ControlCommand::ReloadIf(identity)) {
        Ok(ControlResponse::Ok) => ("applied", "Saved and applied to the running goose.".into()),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            (
                "saved",
                "Saved. The goose will use these settings when it starts.".into(),
            )
        }
        result => (
            "saved",
            format!("Saved; runtime reload was not confirmed: {result:?}"),
        ),
    };
    let snapshot = ConfigSnapshot {
        config,
        revision,
        warning,
    };
    let mut data = snapshot_json(&snapshot);
    data["apply_state"] = json!(apply_state);
    data["message"] = json!(message);
    data["restart_required"] = json!(restart);
    Ok(data)
}

fn runtime_json() -> Value {
    runtime_status_json(send_command(ControlCommand::Status))
}

fn runtime_status_json(result: io::Result<ControlResponse>) -> Value {
    let status = match result {
        Ok(ControlResponse::Status(status)) => status,
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            RuntimeStatus::not_running()
        }
        result => {
            return json!({"available": false, "running": null,
            "error": format!("Runtime status could not be confirmed: {result:?}")})
        }
    };
    let presence = if status.running {
        match send_command(ControlCommand::PresenceStatus) {
            Ok(ControlResponse::Presence(value)) => value,
            _ => honk_control::PresenceStatus::unprobed(),
        }
    } else {
        honk_control::PresenceStatus::unprobed()
    };
    let session = if status.running && status.platform == honk_control::PlatformStatus::Linux {
        match send_command(ControlCommand::Session) {
            Ok(ControlResponse::Session(session)) => Some(json!({
                "backend": session.backend.label(), "desktop_hint": session.desktop.label(),
                "prop_positioning": session.prop_positioning.label(),
            })),
            _ => None,
        }
    } else {
        None
    };
    json!({"available": true, "running": status.running, "platform": status.platform.label(),
        "overlay": status.overlay.label(), "accessibility": status.accessibility.label(),
        "cursor": status.cursor.label(), "windows": status.window.label(),
        "notes_and_memes": status.collect.label(), "manners": status.presence.label(),
        "fullscreen": presence.fullscreen.label(), "dnd": presence.dnd.label(),
        "audio": status.audio.label(), "notes": status.notes, "memes": status.memes,
        "session": session})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gui_save_holds_the_revision_guard_during_login_reconciliation() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        Config::default().save_atomic(&path).unwrap();
        let before = ConfigSnapshot::load(&path).unwrap();
        let result = save(
            &path,
            before.revision,
            BTreeMap::from([("lifecycle.autostart_on_login".into(), json!(true))]),
            &|config| {
                assert!(config.lifecycle.autostart_on_login);
                let saved = ConfigSnapshot::load(&path).unwrap();
                let mut rival = saved.config;
                rival.lifecycle.autostart_on_login = false;
                assert!(rival.save_if_revision(&path, &saved.revision).is_err());
                Ok(())
            },
        )
        .unwrap();
        assert!(result["warning"].is_null());
        assert!(
            ConfigSnapshot::load(&path)
                .unwrap()
                .config
                .lifecycle
                .autostart_on_login
        );
    }

    #[test]
    fn read_reloads_installer_intent_before_exposing_the_edit_revision() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(
            &path,
            "# preserve me\n[lifecycle]\nautostart_on_login = false\n",
        )
        .unwrap();
        let before = ConfigSnapshot::load(&path).unwrap();
        let data = read_settings(
            &path,
            &|path, config| {
                config.lifecycle.autostart_on_login = true;
                config.save_if_revision(path, &before.revision)?;
                Ok(())
            },
            &|| runtime_status_json(Err(io::ErrorKind::TimedOut.into())),
        )
        .unwrap();
        let revision: ConfigRevision = serde_json::from_value(data["revision"].clone()).unwrap();
        assert_ne!(revision, before.revision);
        assert_eq!(revision, ConfigRevision::read(&path).unwrap());
        let (_, changed) = edited(
            &path,
            &revision,
            BTreeMap::from([("audio.enabled".into(), json!(false))]),
        )
        .unwrap();
        assert!(changed.lifecycle.autostart_on_login);
        assert!(!changed.audio.enabled);
        assert!(std::fs::read_to_string(&path)
            .unwrap()
            .contains("# preserve me"));
        assert_eq!(
            data["fields"].as_array().unwrap().len(),
            schema::FIELDS.len()
        );
        assert_eq!(data["runtime"]["available"], false);
        assert!(data["runtime"]["running"].is_null());
    }

    #[test]
    fn unavailable_runtime_status_is_distinct_from_a_stopped_goose() {
        for result in [
            Err(io::ErrorKind::PermissionDenied.into()),
            Err(io::ErrorKind::TimedOut.into()),
            Err(io::ErrorKind::InvalidData.into()),
            Ok(ControlResponse::Ok),
        ] {
            let value = runtime_status_json(result);
            assert_eq!(value["available"], false);
            assert!(value["running"].is_null());
            assert!(value["error"]
                .as_str()
                .unwrap()
                .contains("could not be confirmed"));
        }
        let stopped = runtime_status_json(Err(io::ErrorKind::NotFound.into()));
        assert_eq!(stopped["available"], true);
        assert_eq!(stopped["running"], false);
    }

    // APFS rejects this filename before the service runs (EILSEQ). Exercise
    // genuinely valid non-Unicode paths on the native filesystems that accept them.
    #[cfg(any(target_os = "linux", windows))]
    #[test]
    fn native_non_unicode_config_path_keeps_the_service_protocol_valid() {
        let directory = tempfile::tempdir().unwrap();
        #[cfg(unix)]
        let name = {
            use std::os::unix::ffi::OsStringExt;
            std::ffi::OsString::from_vec(b"config-\xff.toml".to_vec())
        };
        #[cfg(windows)]
        let name = {
            use std::os::windows::ffi::OsStringExt;
            std::ffi::OsString::from_wide(&[99, 0xd800, 46, 116, 111, 109, 108])
        };
        let path = directory.path().join(name);
        Config::default().save_atomic(&path).unwrap();
        let response = request(
            &path,
            br#"{"protocol":1,"request_id":19,"command":{"op":"read"}}"#,
        );
        assert_eq!(response["ok"], true);
        assert!(response["data"].get("path").is_none());
        assert_eq!(
            response["data"]["fields"].as_array().unwrap().len(),
            schema::FIELDS.len()
        );
    }

    fn request(path: &Path, bytes: &[u8]) -> Value {
        let mut response = Vec::new();
        serve(bytes, &mut response, path).unwrap();
        serde_json::from_slice(&response).unwrap()
    }

    #[test]
    fn protocol_rejects_wrong_versions_extra_fields_and_oversize_frames_without_writes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        for bytes in [
            br#"{"protocol":2,"request_id":1,"command":{"op":"read"}}"#.to_vec(),
            br#"{"protocol":1,"request_id":1,"command":{"op":"read","shell":"whoami"}}"#.to_vec(),
            vec![b' '; MAX_REQUEST as usize + 1],
            br#"{"protocol":1,"request_id":1,"command":{"op":"read"}} {}"#.to_vec(),
        ] {
            assert_eq!(request(&path, &bytes)["ok"], false);
        }
        assert!(!path.exists());
    }

    #[test]
    fn catalogue_covers_every_editable_config_leaf_once_and_patch_is_typed() {
        let config = Config::default();
        let serialized = serde_json::to_value(&config).unwrap();
        let mut keys = std::collections::BTreeSet::new();
        for (section, value) in serialized.as_object().unwrap() {
            // Remote control (ADR 0059) is configured in config.toml until the native window
            // gains a secret-aware field; Rust saves still preserve the section.
            if section == "goose_config_version" || section == "remote" {
                continue;
            }
            for name in value.as_object().unwrap().keys() {
                if !cfg!(target_os = "linux") && section == "platform" && name == "wayland" {
                    continue;
                }
                keys.insert(format!("{section}.{name}"));
            }
        }
        assert_eq!(keys.len(), schema::FIELDS.len());
        assert_eq!(
            keys,
            schema::FIELDS
                .iter()
                .map(|field| field.key.to_owned())
                .collect()
        );
        assert!(apply_patch(
            &config,
            BTreeMap::from([("audio.enabled".into(), json!("yes"))])
        )
        .is_err());
        assert!(apply_patch(
            &config,
            BTreeMap::from([("goose_config_version".into(), json!(99))])
        )
        .is_err());
        assert!(apply_patch(
            &config,
            BTreeMap::from([("speeds.walk_speed".into(), json!(-1))])
        )
        .is_err());
    }

    #[test]
    fn settings_edits_preserve_remote_control_and_cannot_patch_it() {
        let mut config = Config::default();
        config.remote.enabled = true;
        config.remote.token = "0123456789abcdef".into();
        let edited = apply_patch(
            &config,
            BTreeMap::from([("audio.enabled".into(), json!(false))]),
        )
        .unwrap();
        assert_eq!(edited.remote, config.remote);
        assert!(apply_patch(
            &config,
            BTreeMap::from([("remote.enabled".into(), json!(false))])
        )
        .is_err());
    }

    #[test]
    fn wayland_visibility_is_platform_specific_and_other_edits_preserve_its_value() {
        let mut config = Config::default();
        config.platform.wayland = true;
        let fields = serde_json::to_value(schema::fields(&config)).unwrap();
        assert_eq!(
            fields
                .as_array()
                .unwrap()
                .iter()
                .any(|field| field["key"] == "platform.wayland"),
            cfg!(target_os = "linux")
        );
        let edited = apply_patch(
            &config,
            BTreeMap::from([("audio.enabled".into(), json!(false))]),
        )
        .unwrap();
        assert!(edited.platform.wayland);
        assert!(!edited.audio.enabled);
    }

    #[test]
    fn stale_gui_save_and_validation_cannot_mutate_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let snapshot = ConfigSnapshot::load(&path).unwrap();
        Config::default().save_atomic(&path).unwrap();
        let contents = std::fs::read(&path).unwrap();
        let patch = BTreeMap::from([("audio.enabled".to_owned(), json!(false))]);
        let bytes = serde_json::to_vec(&json!({"protocol":1,"request_id":9,"command":{"op":"save","revision":snapshot.revision,"patch":patch}})).unwrap();
        let result = request(&path, &bytes);
        assert_eq!(result["request_id"], 9);
        assert_eq!(result["error"]["code"], "conflict");
        assert_eq!(std::fs::read(&path).unwrap(), contents);
    }

    #[test]
    fn validated_patch_preserves_unedited_fields_and_comments() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, "# mine\n[audio]\nfuture = 3 # preserve\n").unwrap();
        let before = ConfigSnapshot::load(&path).unwrap();
        let (_, edited) = edited(
            &path,
            &before.revision,
            BTreeMap::from([("appearance.expressions".into(), json!(false))]),
        )
        .unwrap();
        assert!(!edited.appearance.expressions);
        assert_eq!(edited.audio, before.config.audio);
        edited.save_if_revision(&path, &before.revision).unwrap();
        let contents = std::fs::read_to_string(path).unwrap();
        assert!(contents.contains("# mine") && contents.contains("future = 3 # preserve"));
    }
}
