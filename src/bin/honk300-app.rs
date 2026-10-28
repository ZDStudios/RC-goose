//! Windowless Windows entry point for terminals, Explorer, shortcuts, and login startup.
//!
//! `honk300.exe` intentionally remains a console-subsystem executable for public commands. This
//! GUI-subsystem companion starts its exact sibling through the private runtime command with no
//! console or shell, waits for bounded IPC readiness, and then exits. The hidden CLI child owns
//! the singleton, overlay, IPC, and notification-area controls for the runtime lifetime.
//! Application-menu activation passes `--settings` to open the verified graphical controls.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
#[path = "../windows_stdio.rs"]
mod windows_stdio;

#[cfg(windows)]
const LAUNCH_FAILURE: i32 = 10;
#[cfg(windows)]
const RUNTIME_EXITED: i32 = 11;
#[cfg(windows)]
const READINESS_TIMEOUT: i32 = 12;
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
#[cfg(windows)]
const CREATE_BREAKAWAY_FROM_JOB: u32 = 0x0100_0000;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;
#[cfg(windows)]
const BASE_RUNTIME_CREATION_FLAGS: u32 = CREATE_NO_WINDOW | CREATE_NEW_PROCESS_GROUP;

#[cfg(windows)]
fn main() {
    let code = run().unwrap_or_else(|code| {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        if std::env::args_os().nth(1).as_deref() == Some(std::ffi::OsStr::new("--settings")) {
            let message = w!("Goose could not open. Please reinstall it using the official installer to repair its application files.");
            unsafe {
                MessageBoxW(None, message, w!("Goose"), MB_OK | MB_ICONERROR);
            }
        }
        code
    });
    std::process::exit(code);
}

#[cfg(windows)]
fn run() -> Result<i32, i32> {
    windows_stdio::prevent_inheritance().map_err(|_| LAUNCH_FAILURE)?;

    let launcher = std::env::current_exe().map_err(|_| LAUNCH_FAILURE)?;
    let bin = launcher.parent().ok_or(LAUNCH_FAILURE)?;
    let runtime = bin.join("honk300.exe");
    if !runtime.is_file() {
        return Err(LAUNCH_FAILURE);
    }

    let forwarded = std::env::args_os().skip(1).collect::<Vec<_>>();
    // App-menu activation opens the same verified Rust-owned settings route, including when
    // the goose is stopped or already running. Login and explicit start keep their runtime route.
    if forwarded.as_slice() == [std::ffi::OsString::from("--settings")] {
        let mut child = spawn_runtime(&runtime, bin, &forwarded).map_err(|_| LAUNCH_FAILURE)?;
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(12);
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    return if status.success() {
                        Ok(0)
                    } else {
                        Err(LAUNCH_FAILURE)
                    }
                }
                Ok(None) if std::time::Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(50))
                }
                _ => {
                    stop_unready_child(&mut child, ChildState::Running);
                    return Err(LAUNCH_FAILURE);
                }
            }
        }
    }
    if probe_runtime() == RuntimeProbe::Ready {
        return Ok(0);
    }
    let mut child = spawn_runtime(&runtime, bin, &forwarded).map_err(|_| LAUNCH_FAILURE)?;

    let started = std::time::Instant::now();
    let mut child_state = ChildState::Running;
    loop {
        if child_state == ChildState::Running {
            child_state = match child.try_wait() {
                Ok(Some(status)) if status.success() => ChildState::ExitedSuccessfully,
                Ok(Some(_)) => ChildState::ExitedWithFailure,
                Ok(None) => ChildState::Running,
                Err(_) => {
                    stop_unready_child(&mut child, child_state);
                    return Err(LAUNCH_FAILURE);
                }
            };
        }
        let timed_out = started.elapsed() >= std::time::Duration::from_secs(10);
        match readiness_decision(probe_runtime(), child_state, timed_out) {
            ReadinessDecision::Ready => return Ok(0),
            ReadinessDecision::Continue => {
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
            ReadinessDecision::RuntimeExited => return Err(RUNTIME_EXITED),
            ReadinessDecision::LaunchFailure => {
                stop_unready_child(&mut child, child_state);
                return Err(LAUNCH_FAILURE);
            }
            ReadinessDecision::TimedOut => {
                stop_unready_child(&mut child, child_state);
                return Err(READINESS_TIMEOUT);
            }
        }
    }
}

#[cfg(windows)]
fn spawn_runtime(
    runtime: &std::path::Path,
    bin: &std::path::Path,
    forwarded: &[std::ffi::OsString],
) -> std::io::Result<std::process::Child> {
    // Integrated terminals commonly place the launching shell in a kill-on-close job. A new
    // process group does not escape that job, so the terminal's command runner can continue
    // waiting on the otherwise independent runtime. Prefer an explicit job breakaway while the
    // app is still transient. Corporate jobs may deliberately deny breakaway; access-denied is
    // therefore the one safe fallback to the original windowless launch rather than a startup
    // failure or a shell intermediary.
    match runtime_command(
        runtime,
        bin,
        forwarded,
        BASE_RUNTIME_CREATION_FLAGS | CREATE_BREAKAWAY_FROM_JOB,
    )
    .spawn()
    {
        Ok(child) => Ok(child),
        Err(error) if error.raw_os_error() == Some(5) => {
            runtime_command(runtime, bin, forwarded, BASE_RUNTIME_CREATION_FLAGS).spawn()
        }
        Err(error) => Err(error),
    }
}

#[cfg(windows)]
fn runtime_command(
    runtime: &std::path::Path,
    bin: &std::path::Path,
    forwarded: &[std::ffi::OsString],
    creation_flags: u32,
) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    use std::process::Stdio;

    let mut command = std::process::Command::new(runtime);
    if forwarded == [std::ffi::OsString::from("--settings")] {
        command.arg("settings");
    } else {
        command.arg("__windows-app-runtime").args(forwarded);
    }
    command
        .current_dir(bin)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(creation_flags);
    command
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RuntimeProbe {
    Ready,
    Transient,
    Fatal,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChildState {
    Running,
    ExitedSuccessfully,
    ExitedWithFailure,
}

#[cfg(windows)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadinessDecision {
    Ready,
    Continue,
    RuntimeExited,
    LaunchFailure,
    TimedOut,
}

#[cfg(windows)]
fn probe_runtime() -> RuntimeProbe {
    use honk_control::{send_command, ControlCommand, ControlResponse};

    match send_command(ControlCommand::Status) {
        Ok(ControlResponse::Status(status)) if status.running => RuntimeProbe::Ready,
        Ok(ControlResponse::Status(_)) => RuntimeProbe::Transient,
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::NotFound
                    | std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::WouldBlock
                    | std::io::ErrorKind::TimedOut
            ) =>
        {
            RuntimeProbe::Transient
        }
        Ok(
            ControlResponse::Ok
            | ControlResponse::Err(_)
            | ControlResponse::Session(_)
            | ControlResponse::Wayland(_)
            | ControlResponse::Presence(_)
            | ControlResponse::Toy(_)
            | ControlResponse::Snapshot(_)
            | ControlResponse::Toys(_),
        )
        | Err(_) => RuntimeProbe::Fatal,
    }
}

#[cfg(windows)]
fn readiness_decision(
    probe: RuntimeProbe,
    child: ChildState,
    timed_out: bool,
) -> ReadinessDecision {
    if probe == RuntimeProbe::Ready {
        return ReadinessDecision::Ready;
    }
    if child == ChildState::ExitedWithFailure {
        return ReadinessDecision::RuntimeExited;
    }
    if probe == RuntimeProbe::Fatal {
        return ReadinessDecision::LaunchFailure;
    }
    if timed_out {
        return ReadinessDecision::TimedOut;
    }
    ReadinessDecision::Continue
}

#[cfg(windows)]
fn stop_unready_child(child: &mut std::process::Child, state: ChildState) {
    if state == ChildState::Running {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(not(windows))]
fn main() {
    // cargo-dist excludes this binary from every non-Windows target. Keeping a buildable stub
    // preserves ordinary cross-platform `cargo check --all-targets` behavior.
    eprintln!("honk300-app is an internal Windows-only launcher");
    std::process::exit(1);
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn readiness_requires_a_live_runtime_and_classifies_bounded_failures() {
        assert_eq!(
            readiness_decision(RuntimeProbe::Ready, ChildState::Running, false),
            ReadinessDecision::Ready
        );
        assert_eq!(
            readiness_decision(
                RuntimeProbe::Transient,
                ChildState::ExitedWithFailure,
                false
            ),
            ReadinessDecision::RuntimeExited
        );
        assert_eq!(
            readiness_decision(RuntimeProbe::Fatal, ChildState::Running, false),
            ReadinessDecision::LaunchFailure
        );
        assert_eq!(
            readiness_decision(RuntimeProbe::Transient, ChildState::Running, true),
            ReadinessDecision::TimedOut
        );
        assert_eq!(
            readiness_decision(
                RuntimeProbe::Transient,
                ChildState::ExitedSuccessfully,
                false
            ),
            ReadinessDecision::Continue
        );
    }

    #[test]
    fn concurrent_launcher_observes_the_singleton_winner_after_its_child_exits() {
        assert_eq!(
            readiness_decision(
                RuntimeProbe::Transient,
                ChildState::ExitedSuccessfully,
                false
            ),
            ReadinessDecision::Continue
        );
        assert_eq!(
            readiness_decision(RuntimeProbe::Ready, ChildState::ExitedSuccessfully, false),
            ReadinessDecision::Ready
        );
        assert_eq!(
            readiness_decision(
                RuntimeProbe::Transient,
                ChildState::ExitedSuccessfully,
                true
            ),
            ReadinessDecision::TimedOut
        );
    }

    #[test]
    fn hidden_runtime_prefers_terminal_job_breakaway_without_detached_process() {
        assert_eq!(CREATE_BREAKAWAY_FROM_JOB, 0x0100_0000);
        assert_eq!(BASE_RUNTIME_CREATION_FLAGS, 0x0800_0200);
        assert_eq!(
            BASE_RUNTIME_CREATION_FLAGS | CREATE_BREAKAWAY_FROM_JOB,
            0x0900_0200
        );
    }
}
