use crate::runtime::core::RuntimeCore;
use honk_control::ControlSurfaceCommand;
use honk_engine::World;
use std::io;

#[cfg(target_os = "macos")]
pub(crate) use honk_platform_macos::open_update_helper;

pub(crate) fn open_configuration(config_path: &std::path::Path) -> io::Result<()> {
    crate::settings::launch(Some(config_path.to_owned()))
}

#[cfg(windows)]
pub(crate) fn open_update_helper() -> io::Result<()> {
    open_windows_console("__control-surface-update")
}

#[cfg(windows)]
fn open_windows_console(argument: &str) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        CreateProcessW, CREATE_NEW_CONSOLE, PROCESS_INFORMATION, STARTUPINFOW,
    };

    let executable = std::env::current_exe()?;
    let executable_units = executable.as_os_str().encode_wide().collect::<Vec<_>>();
    let mut application = executable_units.clone();
    application.push(0);
    let mut command_line = windows_command_line(&executable_units, argument);
    let startup = STARTUPINFOW {
        cb: std::mem::size_of::<STARTUPINFOW>() as u32,
        ..Default::default()
    };
    let mut process = PROCESS_INFORMATION::default();

    unsafe {
        CreateProcessW(
            PCWSTR(application.as_ptr()),
            PWSTR(command_line.as_mut_ptr()),
            None,
            None,
            false,
            CREATE_NEW_CONSOLE,
            None,
            PCWSTR::null(),
            &startup,
            &mut process,
        )
        .map_err(|error| io::Error::other(error.to_string()))?;
        let _ = CloseHandle(process.hThread);
        let _ = CloseHandle(process.hProcess);
    }
    Ok(())
}

#[cfg(any(test, windows))]
fn windows_command_line(executable: &[u16], argument: &str) -> Vec<u16> {
    let mut command_line = quote_windows_argument(executable);
    command_line.push(b' ' as u16);
    command_line.extend(argument.encode_utf16());
    command_line.push(0);
    command_line
}

#[cfg(any(test, windows))]
fn quote_windows_argument(argument: &[u16]) -> Vec<u16> {
    let mut quoted = vec![b'"' as u16];
    let mut backslashes = 0usize;
    for &unit in argument {
        if unit == b'\\' as u16 {
            backslashes += 1;
            continue;
        }
        if unit == b'"' as u16 {
            quoted.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2 + 1));
        } else {
            quoted.extend(std::iter::repeat_n(b'\\' as u16, backslashes));
        }
        backslashes = 0;
        quoted.push(unit);
    }
    quoted.extend(std::iter::repeat_n(b'\\' as u16, backslashes * 2));
    quoted.push(b'"' as u16);
    quoted
}

#[cfg(target_os = "linux")]
pub(crate) fn open_update_helper() -> io::Result<()> {
    open_linux_terminal("__control-surface-update")
}

#[cfg(target_os = "linux")]
fn open_linux_terminal(argument: &str) -> io::Result<()> {
    let executable = std::env::current_exe()?;
    let Some(launcher) = linux_terminal_launchers()
        .into_iter()
        .find(|launcher| executable_on_path(launcher.program))
    else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no supported terminal launcher found (tried xdg-terminal-exec and common desktop terminals)",
        ));
    };

    let mut command = std::process::Command::new(launcher.program);
    command.args(linux_terminal_arguments(launcher, &executable, argument));
    command.spawn().map(|_| ())
}

#[cfg(any(test, target_os = "linux"))]
fn linux_terminal_arguments(
    launcher: LinuxTerminalLauncher,
    executable: &std::path::Path,
    argument: &str,
) -> Vec<std::ffi::OsString> {
    launcher
        .arguments
        .iter()
        .copied()
        .map(std::ffi::OsString::from)
        .chain([executable.as_os_str().to_owned(), argument.into()])
        .collect()
}

#[cfg(any(test, target_os = "linux"))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LinuxTerminalLauncher {
    program: &'static str,
    arguments: &'static [&'static str],
}

#[cfg(any(test, target_os = "linux"))]
fn linux_terminal_launchers() -> [LinuxTerminalLauncher; 11] {
    [
        LinuxTerminalLauncher {
            program: "xdg-terminal-exec",
            arguments: &[],
        },
        LinuxTerminalLauncher {
            program: "x-terminal-emulator",
            arguments: &["-e"],
        },
        LinuxTerminalLauncher {
            program: "kgx",
            arguments: &["--"],
        },
        LinuxTerminalLauncher {
            program: "gnome-terminal",
            arguments: &["--"],
        },
        LinuxTerminalLauncher {
            program: "konsole",
            arguments: &["-e"],
        },
        LinuxTerminalLauncher {
            program: "mate-terminal",
            arguments: &["--"],
        },
        LinuxTerminalLauncher {
            program: "foot",
            arguments: &["-e"],
        },
        LinuxTerminalLauncher {
            program: "alacritty",
            arguments: &["-e"],
        },
        LinuxTerminalLauncher {
            program: "kitty",
            arguments: &[],
        },
        LinuxTerminalLauncher {
            program: "wezterm",
            arguments: &["start", "--"],
        },
        LinuxTerminalLauncher {
            program: "xterm",
            arguments: &["-e"],
        },
    ]
}

#[cfg(target_os = "linux")]
fn executable_on_path(program: &str) -> bool {
    let program = std::path::Path::new(program);
    if program.components().count() > 1 {
        return program.is_file();
    }
    std::env::var_os("PATH").is_some_and(|path| {
        std::env::split_paths(&path).any(|directory| directory.join(program).is_file())
    })
}

/// Applies the shared native control-surface actions.
///
/// Backends own native UI and only emit a finite command. Keeping the action router here makes
/// it impossible for one platform's Quit menu item to bypass the engine-owned walk-off and final
/// transparent frame.
pub(crate) fn handle_command(
    command: ControlSurfaceCommand,
    world: &mut World,
    open_configuration: impl FnOnce() -> io::Result<()>,
    open_update: impl FnOnce() -> io::Result<()>,
) -> io::Result<()> {
    match command {
        ControlSurfaceCommand::Configure => open_configuration(),
        ControlSurfaceCommand::Update => open_update(),
        ControlSurfaceCommand::Quit => {
            println!("honk300: control-surface Quit received; walking home.");
            RuntimeCore::begin_graceful_stop(world);
            Ok(())
        }
        // The runtime owns the remote connection and intercepts this before dispatch; reaching
        // the shared router means remote control is not running on this platform.
        ControlSurfaceCommand::DisconnectRemote => Ok(()),
    }
}

pub(crate) fn command_name(command: ControlSurfaceCommand) -> &'static str {
    match command {
        ControlSurfaceCommand::Configure => "Configure",
        ControlSurfaceCommand::Update => "Update",
        ControlSurfaceCommand::Quit => "Quit",
        ControlSurfaceCommand::DisconnectRemote => "Disconnect remote control",
    }
}

#[cfg(test)]
mod tests {
    use super::{
        command_name, handle_command, linux_terminal_arguments, linux_terminal_launchers,
        quote_windows_argument, windows_command_line,
    };
    use honk_control::ControlSurfaceCommand;
    use honk_engine::{Rect, Vec2, World};

    fn world() -> World {
        World::new(Rect::new(Vec2::ZERO, Vec2::new(1200.0, 800.0)), 7)
    }

    #[test]
    fn configure_uses_the_existing_tui_launcher_without_stopping() {
        let mut world = world();
        let mut opened = false;

        handle_command(
            ControlSurfaceCommand::Configure,
            &mut world,
            || {
                opened = true;
                Ok(())
            },
            || panic!("Configure must not invoke the update helper"),
        )
        .expect("configuration launcher should succeed");

        assert!(opened);
        assert!(!world.graceful_exit_requested());
    }

    #[test]
    fn update_uses_the_helper_launcher_without_stopping() {
        let mut world = world();
        let mut opened = false;

        handle_command(
            ControlSurfaceCommand::Update,
            &mut world,
            || panic!("Update must not invoke the configuration launcher"),
            || {
                opened = true;
                Ok(())
            },
        )
        .expect("update helper should launch");

        assert!(opened);
        assert!(!world.graceful_exit_requested());
    }

    #[test]
    fn quit_enters_the_engine_owned_graceful_walk_off() {
        let mut world = world();

        handle_command(
            ControlSurfaceCommand::Quit,
            &mut world,
            || panic!("Quit must not invoke the configuration launcher"),
            || panic!("Quit must not invoke the update helper"),
        )
        .expect("Quit should be accepted");

        assert!(world.graceful_exit_requested());
    }

    #[test]
    fn configure_errors_do_not_turn_into_shutdown() {
        let mut world = world();
        let error = handle_command(
            ControlSurfaceCommand::Configure,
            &mut world,
            || {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no terminal",
                ))
            },
            || panic!("Configure must not invoke the update helper"),
        )
        .expect_err("launcher error should remain visible");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(!world.graceful_exit_requested());
    }

    #[test]
    fn update_errors_do_not_turn_into_shutdown() {
        let mut world = world();
        let error = handle_command(
            ControlSurfaceCommand::Update,
            &mut world,
            || panic!("Update must not invoke the configuration launcher"),
            || {
                Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "no terminal",
                ))
            },
        )
        .expect_err("launcher error should remain visible");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        assert!(!world.graceful_exit_requested());
    }

    #[test]
    fn linux_terminal_launchers_never_require_shell_interpolation() {
        let launchers = linux_terminal_launchers();
        assert_eq!(launchers[0].program, "xdg-terminal-exec");
        assert!(launchers.iter().all(|launcher| {
            !launcher.program.chars().any(char::is_whitespace)
                && launcher
                    .arguments
                    .iter()
                    .all(|argument| !argument.contains("sh -c"))
        }));
    }

    #[test]
    fn linux_update_launcher_uses_exact_literal_arguments() {
        let launcher = linux_terminal_launchers()[3];
        let arguments = linux_terminal_arguments(
            launcher,
            std::path::Path::new("/opt/Honk 300/honk300"),
            "__control-surface-update",
        );
        assert_eq!(
            arguments,
            [
                std::ffi::OsString::from("--"),
                std::ffi::OsString::from("/opt/Honk 300/honk300"),
                std::ffi::OsString::from("__control-surface-update"),
            ]
        );
    }

    #[test]
    fn windows_exact_executable_argument_escapes_quotes_and_trailing_slashes() {
        let raw = r#"C:\Program Files\Honk "300"\"#.encode_utf16().collect::<Vec<_>>();
        let quoted = String::from_utf16(&quote_windows_argument(&raw)).unwrap();
        assert_eq!(quoted, r#""C:\Program Files\Honk \"300\"\\""#);
    }

    #[test]
    fn windows_update_launcher_uses_exact_literal_argument() {
        let executable = r#"C:\Program Files\Honk300\honk300.exe"#
            .encode_utf16()
            .collect::<Vec<_>>();
        let command_line = windows_command_line(&executable, "__control-surface-update");
        let command_line = String::from_utf16(&command_line[..command_line.len() - 1]).unwrap();
        assert_eq!(
            command_line,
            r#""C:\Program Files\Honk300\honk300.exe" __control-surface-update"#
        );
    }

    #[test]
    fn diagnostics_name_each_control_surface_action() {
        assert_eq!(command_name(ControlSurfaceCommand::Configure), "Configure");
        assert_eq!(command_name(ControlSurfaceCommand::Update), "Update");
        assert_eq!(command_name(ControlSurfaceCommand::Quit), "Quit");
    }
}
