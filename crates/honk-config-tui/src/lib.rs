//! Terminal config editor for honk300.

pub mod app;
mod terminal;
pub mod ui;

use app::{Action, AppState, CommandResult, TuiCommand};
use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyEvent, KeyEventKind};
use honk_config::{Config, ConfigError, ConfigSnapshot};
use honk_control::{
    send_command, wait_for_shutdown, ControlCommand, ControlResponse, RuntimeStatus,
};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

pub fn run(config_path: PathBuf) -> Result<()> {
    run_with_save_hook(config_path, |_| Ok(()))
}

pub fn run_with_save_hook<F>(config_path: PathBuf, save_hook: F) -> Result<()>
where
    F: Fn(&Config) -> std::result::Result<(), String> + Send + Sync + 'static,
{
    run_with_hooks(config_path, save_hook, |_| {
        Err("update actions require the honk300 host".into())
    })
}

pub fn run_with_hooks<F, G>(config_path: PathBuf, save_hook: F, update_hook: G) -> Result<()>
where
    F: Fn(&Config) -> std::result::Result<(), String> + Send + Sync + 'static,
    G: Fn(TuiCommand) -> std::result::Result<String, String> + Send + Sync + 'static,
{
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()?;
    runtime.block_on(run_async(
        config_path,
        Arc::new(save_hook),
        Arc::new(update_hook),
    ))
}

type ConfigSaveHook = Arc<dyn Fn(&Config) -> std::result::Result<(), String> + Send + Sync>;
type UpdateHook = Arc<dyn Fn(TuiCommand) -> std::result::Result<String, String> + Send + Sync>;

async fn run_async(
    config_path: PathBuf,
    save_hook: ConfigSaveHook,
    update_hook: UpdateHook,
) -> Result<()> {
    terminal::install_panic_hook()?;
    let loaded = load_tui_config(config_path.clone())?;
    let mut app = AppState::new(loaded.config, config_path);
    app.revision = loaded.revision;
    if let Some(warning) = loaded.warning {
        app.set_status(format!("config warning: {warning}"), false);
    }
    app.apply(Action::Status);

    let (_guard, mut terminal) = terminal::TerminalGuard::enter(terminal::TerminalOptions {
        alt_screen: true,
        mouse: false,
    })?;

    let mut keys = spawn_key_reader();
    let mut tick = tokio::time::interval(Duration::from_millis(100));
    let (command_result_tx, mut command_results) = mpsc::unbounded_channel();
    let mut command_busy = false;

    loop {
        terminal.draw(|frame| ui::render(frame, &app))?;
        if app.should_quit {
            break;
        }

        tokio::select! {
            _ = tick.tick() => {}
            maybe_key = keys.recv() => {
                if let Some(key) = maybe_key {
                    let action = app.resolve_key(key);
                    app.apply(action);
                }
            }
            maybe_result = command_results.recv(), if command_busy => {
                if let Some(command_result) = maybe_result {
                    app.apply(Action::CommandResult(Box::new(command_result)));
                }
                command_busy = false;
            }
        }

        if !command_busy {
            if let Some(command) = app.take_pending_command() {
                let snapshot = app.clone();
                let tx = command_result_tx.clone();
                let save_hook = Arc::clone(&save_hook);
                app.set_status("working...".into(), false);
                let update_hook = Arc::clone(&update_hook);
                spawn_blocking_operation(tx, move || {
                    if matches!(command, TuiCommand::CheckUpdates | TuiCommand::Update) {
                        match update_hook(command) {
                            Ok(message) => result(message, false, false),
                            Err(error) => result(error, true, false),
                        }
                    } else {
                        handle_command(&snapshot, command, save_hook.as_ref())
                    }
                });
                command_busy = true;
            }
        }
    }

    Ok(())
}

fn spawn_blocking_operation<F>(
    tx: mpsc::UnboundedSender<CommandResult>,
    operation: F,
) -> std::thread::JoinHandle<()>
where
    F: FnOnce() -> CommandResult + Send + 'static,
{
    std::thread::spawn(move || {
        let _ = tx.send(operation());
    })
}

fn load_tui_config(path: PathBuf) -> Result<ConfigSnapshot, ConfigError> {
    ConfigSnapshot::load(&path)
}

fn spawn_key_reader() -> mpsc::UnboundedReceiver<KeyEvent> {
    let (tx, rx) = mpsc::unbounded_channel();
    std::thread::spawn(move || loop {
        match event::read() {
            Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => {
                if tx.send(key).is_err() {
                    break;
                }
            }
            Ok(_) => {}
            Err(_) => break,
        }
    });
    rx
}

fn handle_command(
    app: &AppState,
    command: TuiCommand,
    save_hook: &(dyn Fn(&Config) -> std::result::Result<(), String> + Send + Sync),
) -> CommandResult {
    match command {
        TuiCommand::Save => {
            let mut saved_revision = None;
            let mut command_result = match app
                .config
                .validate()
                .and_then(|_| app.config.save_if_revision(&app.path, &app.revision))
            {
                Ok(revision) => {
                    saved_revision = Some(revision);
                    match reconcile_saved_config(
                        app,
                        saved_revision.as_ref().expect("saved revision"),
                        save_hook,
                    ) {
                        Err(error) => result(
                            format!("saved; login autostart reconcile failed: {error}"),
                            true,
                            true,
                        ),
                        Ok(()) => match saved_revision
                            .as_ref()
                            .expect("save returned a revision")
                            .reload_token(&app.path)
                            .map_err(std::io::Error::other)
                            .and_then(|identity| send_command(ControlCommand::ReloadIf(identity)))
                        {
                            Ok(ControlResponse::Ok) => result("saved; reload sent", false, true),
                            Ok(ControlResponse::Err(code)) => {
                                result(format!("saved; reload rejected: {code}"), true, true)
                            }
                            Ok(
                                ControlResponse::Status(_)
                                | ControlResponse::Session(_)
                                | ControlResponse::Wayland(_)
                                | ControlResponse::Presence(_)
                                | ControlResponse::Leaf(_)
                                | ControlResponse::Snapshot(_)
                                | ControlResponse::Leaves(_),
                            ) => result("saved; unexpected status response", true, true),
                            Err(error)
                                if matches!(
                                    error.kind(),
                                    std::io::ErrorKind::NotFound
                                        | std::io::ErrorKind::ConnectionRefused
                                ) =>
                            {
                                result("saved; no running goose to reload", false, true)
                            }
                            Err(error) => {
                                result(format!("saved; reload not confirmed: {error}"), true, true)
                            }
                        },
                    }
                }
                Err(err) => result(format!("save failed: {err}"), true, false),
            };
            if command_result.mark_saved {
                command_result.saved_config = Some(app.config.clone());
            }
            command_result.saved_revision = saved_revision;
            command_result
        }
        TuiCommand::CheckUpdates | TuiCommand::Update => unreachable!("host hook handles updates"),
        TuiCommand::ReloadFile => match load_tui_config(app.path.clone()) {
            Ok(snapshot) => {
                let mut response = result("saved file reloaded", false, false);
                response.loaded_config = Some((app.config.clone(), snapshot.config));
                response.saved_revision = Some(snapshot.revision);
                response
            }
            Err(error) => result(format!("file reload failed: {error}"), true, false),
        },
        TuiCommand::Reload => match send_command(ControlCommand::Reload) {
            Ok(ControlResponse::Ok) => result("reload sent", false, false),
            Ok(ControlResponse::Err(code)) => {
                result(format!("reload rejected: {code}"), true, false)
            }
            Ok(
                ControlResponse::Status(_)
                | ControlResponse::Session(_)
                | ControlResponse::Wayland(_)
                | ControlResponse::Presence(_)
                | ControlResponse::Leaf(_)
                | ControlResponse::Snapshot(_)
                | ControlResponse::Leaves(_),
            ) => result("reload got unexpected status", true, false),
            Err(err) => result(format!("reload failed: {err}"), true, false),
        },
        TuiCommand::Status => match send_command(ControlCommand::Status) {
            Ok(ControlResponse::Status(status)) => {
                let mut result = status_result(status);
                result.presence_status = Some(if status.running {
                    match send_command(ControlCommand::PresenceStatus) {
                        Ok(ControlResponse::Presence(value)) => value,
                        _ => honk_control::PresenceStatus::unprobed(),
                    }
                } else {
                    honk_control::PresenceStatus::unprobed()
                });
                if status.running && status.platform == honk_control::PlatformStatus::Linux {
                    if let Ok(ControlResponse::Session(session)) =
                        send_command(ControlCommand::Session)
                    {
                        result.status = format!(
                            "{}; session: {}; note placement: {}",
                            session.backend.label(),
                            session.desktop.label(),
                            session.prop_positioning.label()
                        );
                    }
                }
                result
            }
            Ok(
                ControlResponse::Session(_)
                | ControlResponse::Wayland(_)
                | ControlResponse::Presence(_)
                | ControlResponse::Leaf(_)
                | ControlResponse::Snapshot(_)
                | ControlResponse::Leaves(_),
            ) => result("status got unexpected session detail", true, false),
            Ok(ControlResponse::Ok) => result("status got unexpected ok", true, false),
            Ok(ControlResponse::Err(code)) => {
                result(format!("status rejected: {code}"), true, false)
            }
            Err(err)
                if matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::ConnectionRefused
                ) =>
            {
                let mut r = result("not running", false, false);
                r.runtime_status = Some(RuntimeStatus::not_running());
                r
            }
            Err(err) => result(format!("status failed: {err}"), true, false),
        },
        TuiCommand::Stop => match send_command(ControlCommand::Stop) {
            Ok(ControlResponse::Ok) => match wait_for_shutdown() {
                Ok(()) => result("stopped", false, false),
                Err(err) => result(format!("stop stalled: {err}"), true, false),
            },
            Ok(ControlResponse::Err(code)) => result(format!("stop rejected: {code}"), true, false),
            Ok(
                ControlResponse::Status(_)
                | ControlResponse::Session(_)
                | ControlResponse::Wayland(_)
                | ControlResponse::Presence(_)
                | ControlResponse::Leaf(_)
                | ControlResponse::Snapshot(_)
                | ControlResponse::Leaves(_),
            ) => result("stop got unexpected status", true, false),
            Err(err) => result(format!("stop failed: {err}"), true, false),
        },
        TuiCommand::Poke(action) => match send_command(ControlCommand::Do(action)) {
            Ok(ControlResponse::Ok) => result(format!("poke sent: {action:?}"), false, false),
            Ok(ControlResponse::Err(code)) => result(format!("poke rejected: {code}"), true, false),
            Ok(
                ControlResponse::Status(_)
                | ControlResponse::Session(_)
                | ControlResponse::Wayland(_)
                | ControlResponse::Presence(_)
                | ControlResponse::Leaf(_)
                | ControlResponse::Snapshot(_)
                | ControlResponse::Leaves(_),
            ) => result("poke got unexpected status", true, false),
            Err(err) => result(format!("poke failed: {err}"), true, false),
        },
        TuiCommand::Start => handle_start_with_hook(app, start_from_config, save_hook),
    }
}

#[cfg(test)]
fn handle_start_with<F>(app: &AppState, launch: F) -> CommandResult
where
    F: FnOnce(&Path) -> Result<String, String>,
{
    handle_start_with_hook(app, launch, &|_| Ok(()))
}

fn handle_start_with_hook<F>(
    app: &AppState,
    launch: F,
    save_hook: &(dyn Fn(&Config) -> std::result::Result<(), String> + Send + Sync),
) -> CommandResult
where
    F: FnOnce(&Path) -> Result<String, String>,
{
    let saved = app.dirty();
    let mut saved_revision = None;
    if saved {
        match app.config.save_if_revision(&app.path, &app.revision) {
            Ok(revision) => saved_revision = Some(revision),
            Err(error) => {
                return result(format!("start blocked; save failed: {error}"), true, false)
            }
        }
        if let Err(error) = reconcile_saved_config(
            app,
            saved_revision.as_ref().expect("saved revision"),
            save_hook,
        ) {
            let mut command_result = result(
                format!("start blocked; saved, but login autostart reconcile failed: {error}"),
                true,
                true,
            );
            command_result.saved_config = Some(app.config.clone());
            command_result.saved_revision = saved_revision;
            return command_result;
        }
    }
    let mut command_result = match launch(&app.path) {
        Ok(message) => result(message, false, saved),
        Err(error) => result(format!("start failed: {error}"), true, saved),
    };
    if saved {
        command_result.saved_config = Some(app.config.clone());
    }
    command_result.saved_revision = saved_revision;
    command_result
}

fn reconcile_saved_config(
    app: &AppState,
    revision: &honk_config::ConfigRevision,
    save_hook: &(dyn Fn(&Config) -> std::result::Result<(), String> + Send + Sync),
) -> Result<(), String> {
    revision
        .with_guard(&app.path, || {
            save_hook(&app.config).map_err(ConfigError::InvalidTarget)
        })
        .map_err(|error| error.to_string())
}

fn wait_for_readiness<F>(
    timeout: Duration,
    poll_interval: Duration,
    mut probe: F,
) -> Result<RuntimeStatus, String>
where
    F: FnMut() -> std::io::Result<ControlResponse>,
{
    let started = Instant::now();
    loop {
        let last_error = match probe() {
            Ok(ControlResponse::Status(status)) if status.running => return Ok(status),
            Ok(ControlResponse::Status(_)) => "runtime reported not running".into(),
            Ok(ControlResponse::Err(code)) => format!("status rejected: {code}"),
            Ok(ControlResponse::Ok) => "status returned an unexpected OK".into(),
            Ok(
                ControlResponse::Session(_)
                | ControlResponse::Wayland(_)
                | ControlResponse::Presence(_)
                | ControlResponse::Leaf(_)
                | ControlResponse::Snapshot(_)
                | ControlResponse::Leaves(_),
            ) => "status returned unexpected session detail".into(),
            Err(err) => {
                let message = err.to_string();
                if !matches!(
                    err.kind(),
                    std::io::ErrorKind::NotFound
                        | std::io::ErrorKind::ConnectionRefused
                        | std::io::ErrorKind::BrokenPipe
                        | std::io::ErrorKind::ConnectionReset
                        | std::io::ErrorKind::WouldBlock
                        | std::io::ErrorKind::TimedOut
                ) {
                    return Err(message);
                }
                message
            }
        };
        if started.elapsed() >= timeout {
            return Err(format!(
                "runtime did not become ready within {:.1}s: {last_error}",
                timeout.as_secs_f32()
            ));
        }
        if !poll_interval.is_zero() {
            std::thread::sleep(poll_interval);
        }
    }
}

pub fn start_from_config(config_path: &Path) -> Result<String, String> {
    let (mut child, launcher_may_exit) = spawn_start(config_path).map_err(|err| err.to_string())?;
    let mut child_exited = false;
    wait_for_readiness(Duration::from_secs(10), Duration::from_millis(100), || {
        if !child_exited {
            if let Some(status) = child.try_wait()? {
                child_exited = true;
                if !status.success() || !launcher_may_exit {
                    let mut stderr = String::new();
                    if let Some(mut stream) = child.stderr.take() {
                        let _ = stream.read_to_string(&mut stderr);
                    }
                    let detail = if stderr.trim().is_empty() {
                        format!("start process exited with {status}")
                    } else {
                        stderr.trim().to_string()
                    };
                    return Err(std::io::Error::other(detail));
                }
            }
        }
        // Only this read-only readiness probe is retried. Every attempt first
        // checks the child and stays inside the original startup deadline.
        send_command(ControlCommand::Status)
    })
    .map(|_| "start ready".into())
}

fn result(status: impl Into<String>, is_error: bool, mark_saved: bool) -> CommandResult {
    CommandResult {
        status: status.into(),
        is_error,
        mark_saved,
        saved_revision: None,
        loaded_config: None,
        saved_config: None,
        runtime_status: None,
        presence_status: None,
    }
}

fn status_result(status: RuntimeStatus) -> CommandResult {
    CommandResult {
        status: if status.running {
            "status refreshed".into()
        } else {
            "not running".into()
        },
        is_error: false,
        mark_saved: false,
        saved_revision: None,
        loaded_config: None,
        saved_config: None,
        runtime_status: Some(status),
        presence_status: None,
    }
}

fn spawn_start(config_path: &Path) -> std::io::Result<(Child, bool)> {
    let exe = std::env::current_exe()?;
    #[cfg(target_os = "macos")]
    let launcher_may_exit = macos_bundle_root_from_exe(&exe).is_some();
    // Windows `start` is now a bounded controller around the GUI-subsystem app launcher. The
    // controller may exit successfully as soon as the hidden runtime answers IPC readiness.
    #[cfg(windows)]
    let launcher_may_exit = true;
    #[cfg(not(any(windows, target_os = "macos")))]
    let launcher_may_exit = false;
    let mut command = build_start_command(exe, config_path);
    command.spawn().map(|child| (child, launcher_may_exit))
}

fn build_start_command(exe: PathBuf, config_path: &Path) -> Command {
    #[cfg(target_os = "macos")]
    {
        build_macos_start_command(exe, config_path)
    }
    #[cfg(not(target_os = "macos"))]
    {
        build_direct_start_command(exe, config_path)
    }
}

fn build_direct_start_command(exe: PathBuf, config_path: &Path) -> Command {
    let mut command = Command::new(exe);
    command
        .arg("start")
        .arg("--config")
        .arg(config_path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    apply_detached_flags(&mut command);
    command
}

#[cfg(any(target_os = "macos", test))]
fn build_macos_start_command(exe: PathBuf, config_path: &Path) -> Command {
    if let Some(bundle) = macos_bundle_root_from_exe(&exe) {
        let mut command = Command::new("/usr/bin/open");
        command
            .arg("-n")
            .arg(bundle)
            .arg("--args")
            .arg("start")
            .arg("--config")
            .arg(config_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        return command;
    }
    build_direct_start_command(exe, config_path)
}

#[cfg(any(target_os = "macos", test))]
fn macos_bundle_root_from_exe(exe: &Path) -> Option<PathBuf> {
    exe.ancestors()
        .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("app"))
        .map(Path::to_path_buf)
}

#[cfg(windows)]
fn apply_detached_flags(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    command.creation_flags(CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP);
}

#[cfg(not(windows))]
fn apply_detached_flags(_command: &mut Command) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_command_includes_config_path() {
        let command = build_start_command(
            PathBuf::from("honk300.exe"),
            Path::new("C:/tmp/config.toml"),
        );
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert_eq!(args, vec!["start", "--config", "C:/tmp/config.toml"]);
    }

    #[test]
    fn detects_macos_bundle_root_from_exe() {
        let exe = Path::new("/Applications/Honk300.app/Contents/MacOS/honk300");
        assert_eq!(
            macos_bundle_root_from_exe(exe),
            Some(PathBuf::from("/Applications/Honk300.app"))
        );
    }

    #[test]
    fn macos_bundle_start_command_uses_open() {
        let command = build_macos_start_command(
            PathBuf::from("/Applications/Honk300.app/Contents/MacOS/honk300"),
            Path::new("/tmp/config.toml"),
        );
        assert_eq!(command.get_program().to_string_lossy(), "/usr/bin/open");
        let args: Vec<_> = command
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        assert_eq!(
            args,
            vec![
                "-n",
                "/Applications/Honk300.app",
                "--args",
                "start",
                "--config",
                "/tmp/config.toml",
            ]
        );
    }

    #[test]
    fn command_result_can_mark_saved() {
        let r = result("saved", false, true);
        assert!(r.mark_saved);
        assert!(!r.is_error);
        assert!(r.runtime_status.is_none());
    }

    #[test]
    fn tui_loader_rejects_malformed_and_newer_configs_without_rewriting_them() {
        let dir = tempfile::tempdir().unwrap();
        for (name, original) in [
            ("malformed.toml", "[audio\nenabled = true\n"),
            ("newer.toml", "goose_config_version = 99\nfuture = true\n"),
        ] {
            let path = dir.path().join(name);
            std::fs::write(&path, original).unwrap();
            assert!(load_tui_config(path.clone()).is_err());
            assert_eq!(std::fs::read_to_string(path).unwrap(), original);
        }
    }

    #[test]
    fn tui_loader_uses_defaults_only_for_a_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let loaded = load_tui_config(path.clone()).unwrap();
        assert_eq!(loaded.config, Config::default());
        assert!(!path.exists());
    }

    #[test]
    fn dirty_config_is_saved_before_start_is_launched() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        Config::default().save_atomic(&path).unwrap();
        let mut app = AppState::new(Config::default(), path.clone());
        app.revision = honk_config::ConfigRevision::read(&path).unwrap();
        app.config.audio.enabled = false;
        let launched = std::cell::Cell::new(false);

        let command_result = handle_start_with(&app, |launch_path| {
            assert!(!Config::load_existing(launch_path).unwrap().audio.enabled);
            launched.set(true);
            Ok("running and ready".into())
        });

        assert!(launched.get());
        assert!(command_result.mark_saved);
        assert!(!command_result.is_error);
    }

    #[test]
    fn dirty_start_is_blocked_truthfully_when_login_start_reconciliation_fails() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        Config::default().save_atomic(&path).unwrap();
        let mut app = AppState::new(Config::default(), path.clone());
        app.revision = honk_config::ConfigRevision::read(&path).unwrap();
        app.config.lifecycle.autostart_on_login = true;

        let command_result = handle_start_with_hook(
            &app,
            |_| -> Result<String, String> {
                panic!("start must not run after a failed login-start reconciliation")
            },
            &|_| Err("foreign startup entry".into()),
        );

        assert!(command_result.is_error);
        assert!(command_result.mark_saved);
        assert!(command_result.status.contains("start blocked"));
        assert!(command_result.status.contains("foreign startup entry"));
        assert!(
            Config::load_existing(&path)
                .unwrap()
                .lifecycle
                .autostart_on_login
        );
    }

    #[test]
    fn dirty_start_holds_the_config_lock_through_its_login_hook() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        Config::default().save_atomic(&path).unwrap();
        let mut app = AppState::new(Config::default(), path.clone());
        app.revision = honk_config::ConfigRevision::read(&path).unwrap();
        app.config.lifecycle.autostart_on_login = true;
        let result = handle_start_with_hook(&app, |_| Ok("started".into()), &|config| {
            assert!(config.lifecycle.autostart_on_login);
            let latest = ConfigSnapshot::load(&path).unwrap();
            let mut rival = latest.config;
            rival.lifecycle.autostart_on_login = false;
            assert!(rival.save_if_revision(&path, &latest.revision).is_err());
            Ok(())
        });
        assert!(!result.is_error, "{}", result.status);
    }

    #[test]
    fn invalid_dirty_config_prevents_start_launch() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");
        Config::default().save_atomic(&path).unwrap();
        let mut app = AppState::new(Config::default(), path);
        app.config.speeds.walk_speed = 0.0;

        let command_result = handle_start_with(&app, |_| -> Result<String, String> {
            panic!("launch must not run when the dirty config cannot be saved")
        });
        assert!(command_result.is_error);
        assert!(!command_result.mark_saved);
    }

    #[test]
    fn readiness_poll_returns_running_status_after_transient_error() {
        for kind in [
            std::io::ErrorKind::NotFound,
            std::io::ErrorKind::ConnectionRefused,
            std::io::ErrorKind::WouldBlock,
            std::io::ErrorKind::TimedOut,
            std::io::ErrorKind::BrokenPipe,
            std::io::ErrorKind::ConnectionReset,
        ] {
            let mut attempts = 0;
            let status = wait_for_readiness(Duration::from_millis(100), Duration::ZERO, || {
                attempts += 1;
                if attempts == 1 {
                    Err(std::io::Error::new(kind, "not ready yet"))
                } else {
                    let mut status = RuntimeStatus::not_running();
                    status.running = true;
                    Ok(ControlResponse::Status(status))
                }
            })
            .unwrap_or_else(|error| panic!("{kind:?} blocked startup recovery: {error}"));
            assert!(status.running);
            assert_eq!(attempts, 2);
        }
    }

    #[test]
    fn readiness_dropped_connection_obeys_the_existing_deadline() {
        let mut attempts = 0;
        let error = wait_for_readiness(Duration::ZERO, Duration::ZERO, || {
            attempts += 1;
            Err(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "connection dropped",
            ))
        })
        .unwrap_err();
        assert!(
            error.contains("did not become ready within 0.0s"),
            "{error}"
        );
        assert!(error.contains("connection dropped"), "{error}");
        assert_eq!(attempts, 1);
    }

    #[test]
    fn readiness_does_not_retry_permission_protocol_or_child_exit_errors() {
        for kind in [
            std::io::ErrorKind::PermissionDenied,
            std::io::ErrorKind::InvalidData,
            std::io::ErrorKind::Other,
        ] {
            let mut attempts = 0;
            let error = wait_for_readiness(Duration::from_secs(10), Duration::ZERO, || {
                attempts += 1;
                Err(std::io::Error::new(kind, "permanent startup failure"))
            })
            .unwrap_err();
            assert_eq!(error, "permanent startup failure");
            assert_eq!(attempts, 1, "{kind:?} must not be retried");
        }
    }

    #[test]
    fn blocking_command_work_is_dispatched_off_the_caller_path() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        let started = std::time::Instant::now();
        let worker = spawn_blocking_operation(tx, || {
            std::thread::sleep(Duration::from_millis(100));
            result("finished", false, false)
        });
        assert!(started.elapsed() < Duration::from_millis(50));
        assert_eq!(rx.blocking_recv().unwrap().status, "finished");
        worker.join().unwrap();
    }
}
