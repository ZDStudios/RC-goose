use honk_control::{ControlSurfaceCommand, RemoteIndicator};
use std::collections::VecDeque;
use std::ffi::OsStr;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use windows::core::{w, Error, PCWSTR};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_GUID, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_SHOWTIP, NIF_TIP, NIIF_INFO,
    NIIF_WARNING, NIM_ADD, NIM_DELETE, NIM_MODIFY, NIM_SETFOCUS, NIM_SETVERSION, NIN_SELECT,
    NOTIFYICONDATAW, NOTIFYICON_VERSION_4,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyIcon, DestroyMenu,
    DestroyWindow, EndMenu, GetCursorPos, LoadImageW, PostMessageW, RegisterClassExW,
    RegisterWindowMessageW, SetForegroundWindow, TrackPopupMenu, HICON, IMAGE_ICON,
    LR_DEFAULTCOLOR, MF_GRAYED, MF_SEPARATOR, MF_STRING, SM_CXSMICON, SM_CYSMICON, TPM_RETURNCMD,
    TPM_RIGHTBUTTON, WM_CONTEXTMENU, WM_DESTROY, WM_NULL, WM_USER, WNDCLASSEXW, WS_EX_NOACTIVATE,
    WS_EX_TOOLWINDOW, WS_POPUP,
};

const TRAY_CALLBACK_MESSAGE: u32 = WM_USER + 0x300;
const NIN_KEYSELECT: u32 = NIN_SELECT + 1;
const CONFIGURE_COMMAND_ID: usize = 1;
const QUIT_COMMAND_ID: usize = 2;
const UPDATE_COMMAND_ID: usize = 3;
const DISCONNECT_REMOTE_COMMAND_ID: usize = 4;
const REMOTE_STATUS_ID: usize = 5;
const TRAY_ICON_ID: u32 = 1;
const ADD_RETRY_DELAY: Duration = Duration::from_millis(100);
const ADD_FAILURE_NOTICE_AFTER: Duration = Duration::from_secs(3);
const DEGRADED_ADD_RETRY_DELAY: Duration = Duration::from_secs(1);
const TRAY_ICON_GUID: windows::core::GUID =
    windows::core::GUID::from_u128(0x1282_821f_82b6_42e2_945b_ef2f_e8d9_fbda);

static TASKBAR_CREATED_MESSAGE: AtomicU32 = AtomicU32::new(0);
static SMOKE_TRAY_QUIT_MESSAGE: AtomicU32 = AtomicU32::new(0);
static TASKBAR_READD_REQUESTED: AtomicBool = AtomicBool::new(false);
static COMMANDS: OnceLock<Mutex<VecDeque<ControlSurfaceCommand>>> = OnceLock::new();
/// Remote-control state mirrored for the menu, which the window procedure builds on demand.
static REMOTE: Mutex<RemoteMenuState> = Mutex::new(RemoteMenuState {
    indicator: RemoteIndicator::Off,
    host: String::new(),
});

struct RemoteMenuState {
    indicator: RemoteIndicator,
    host: String,
}

fn remote_state() -> (RemoteIndicator, String) {
    let state = REMOTE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    (state.indicator, state.host.clone())
}

/// Visible label for the remote-control state; `None` while remote control is off.
fn remote_status_label(indicator: RemoteIndicator, host: &str) -> Option<String> {
    match indicator {
        RemoteIndicator::Off => None,
        RemoteIndicator::Connecting => Some(format!("Remote control: connecting to {host}…")),
        RemoteIndicator::Connected => Some(format!("● Remote control ON — {host}")),
    }
}

fn tooltip_text(indicator: RemoteIndicator) -> &'static str {
    match indicator {
        RemoteIndicator::Off => "Goose controls",
        RemoteIndicator::Connecting => "Goose controls — remote control connecting",
        RemoteIndicator::Connected => "Goose controls — REMOTE CONTROL ON",
    }
}

fn command_queue() -> &'static Mutex<VecDeque<ControlSurfaceCommand>> {
    COMMANDS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn with_commands<R>(f: impl FnOnce(&mut VecDeque<ControlSurfaceCommand>) -> R) -> R {
    let mut commands = command_queue()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    f(&mut commands)
}

/// Runtime-owned Windows notification-area surface.
///
/// The hidden owner window shares the existing runtime thread's Win32 message pump. Explorer
/// recreation only requests a re-add; the retained Rust owner performs the shell calls on the
/// next runtime iteration and keeps the icon handle alive until the final walk-off completes.
pub struct StatusTray {
    hwnd: HWND,
    icon: windows::Win32::UI::WindowsAndMessaging::HICON,
    added: bool,
    next_add_attempt: Instant,
    add_failure_started: Option<Instant>,
    unavailable_reported: bool,
    taskbar_restore_pending: bool,
}

impl StatusTray {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            with_commands(VecDeque::clear);
            TASKBAR_READD_REQUESTED.store(false, Ordering::Release);

            let taskbar_created = RegisterWindowMessageW(w!("TaskbarCreated"));
            if taskbar_created == 0 {
                return Err(Error::from_win32());
            }
            TASKBAR_CREATED_MESSAGE.store(taskbar_created, Ordering::Release);
            SMOKE_TRAY_QUIT_MESSAGE.store(register_smoke_tray_quit_message()?, Ordering::Release);

            let module = GetModuleHandleW(None)?;
            let instance = HINSTANCE(module.0);
            let class_name = w!("honk300_status_tray_owner");
            let class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                lpfnWndProc: Some(tray_wndproc),
                hInstance: instance,
                lpszClassName: class_name,
                ..Default::default()
            };
            let _ = RegisterClassExW(&class);

            let hwnd = CreateWindowExW(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name,
                w!("Honk300 controls"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                instance,
                None,
            )?;
            let icon = match create_status_icon(hwnd) {
                Ok(icon) => icon,
                Err(error) => {
                    let _ = DestroyWindow(hwnd);
                    return Err(error);
                }
            };
            let mut tray = Self {
                hwnd,
                icon,
                added: false,
                next_add_attempt: Instant::now(),
                add_failure_started: None,
                unavailable_reported: false,
                taskbar_restore_pending: false,
            };
            if let Err(error) = tray.add_to_shell() {
                let now = Instant::now();
                tray.add_failure_started = Some(now);
                tray.next_add_attempt = now + ADD_RETRY_DELAY;
                eprintln!(
                    "honk300: Windows notification-area controls are not ready; retrying while CLI controls remain active ({error})"
                );
            }
            Ok(tray)
        }
    }

    /// Re-adds the retained icon after Explorer/taskbar recreation.
    pub fn maintain(&mut self) -> windows::core::Result<()> {
        let now = Instant::now();
        let restoring_taskbar = TASKBAR_READD_REQUESTED.swap(false, Ordering::AcqRel);
        if restoring_taskbar {
            self.added = false;
            self.next_add_attempt = now;
            self.add_failure_started.get_or_insert(now);
            self.taskbar_restore_pending = true;
        }
        if !self.added && now >= self.next_add_attempt {
            match self.add_to_shell() {
                Ok(()) => {
                    if self.taskbar_restore_pending {
                        eprintln!("honk300: Windows taskbar recreated; restored Honk300 controls.");
                    } else if self.add_failure_started.is_some() {
                        eprintln!("honk300: Windows notification-area controls are now available.");
                    }
                    self.add_failure_started = None;
                    self.unavailable_reported = false;
                    self.taskbar_restore_pending = false;
                }
                Err(error) => {
                    let started = *self.add_failure_started.get_or_insert(now);
                    if !self.unavailable_reported
                        && now.saturating_duration_since(started) >= ADD_FAILURE_NOTICE_AFTER
                    {
                        eprintln!(
                            "honk300: Windows notification-area controls are unavailable; CLI controls remain active ({error})"
                        );
                        self.unavailable_reported = true;
                    }
                    self.next_add_attempt = now
                        + if self.unavailable_reported {
                            DEGRADED_ADD_RETRY_DELAY
                        } else {
                            ADD_RETRY_DELAY
                        };
                }
            }
        }
        Ok(())
    }

    /// Mirror the runtime's remote-control state into the tooltip and menu. Becoming connected
    /// also raises a one-time notification so remote control is never silent.
    pub fn set_remote_indicator(&mut self, indicator: RemoteIndicator, host: &str) {
        let previous = {
            let mut state = REMOTE
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            if state.indicator == indicator && state.host == host {
                return;
            }
            let previous = state.indicator;
            state.indicator = indicator;
            state.host = host.to_string();
            previous
        };
        if !self.added {
            return;
        }
        let mut data = self.notify_data();
        data.uFlags = NIF_GUID | NIF_TIP | NIF_SHOWTIP;
        if indicator == RemoteIndicator::Connected && previous != RemoteIndicator::Connected {
            data.uFlags |= NIF_INFO;
            data.dwInfoFlags = NIIF_INFO;
            write_utf16(&mut data.szInfoTitle, "Goose remote control is on");
            write_utf16(
                &mut data.szInfo,
                &format!(
                    "This goose can be controlled through {host}. Use the tray menu to disconnect."
                ),
            );
        }
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    pub fn take_command(&self) -> Option<ControlSurfaceCommand> {
        with_commands(VecDeque::pop_front)
    }

    /// Make an explicit menu-action failure visible without stopping the goose's event loop.
    pub fn show_action_error(&self, action: &str, error: &std::io::Error) {
        let mut data = self.notify_data();
        data.uFlags = NIF_GUID | NIF_INFO;
        data.dwInfoFlags = NIIF_WARNING;
        write_utf16(&mut data.szInfoTitle, "Goose");
        write_utf16(
            &mut data.szInfo,
            &format!("{action} could not open: {error}"),
        );
        unsafe {
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    fn add_to_shell(&mut self) -> windows::core::Result<()> {
        unsafe {
            let mut data = self.notify_data();
            if !Shell_NotifyIconW(NIM_ADD, &data).as_bool() {
                return Err(Error::from_win32());
            }
            data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
            if !Shell_NotifyIconW(NIM_SETVERSION, &data).as_bool() {
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
                return Err(Error::from_win32());
            }
            self.added = true;
            Ok(())
        }
    }

    fn notify_data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP | NIF_SHOWTIP | NIF_GUID,
            uCallbackMessage: TRAY_CALLBACK_MESSAGE,
            hIcon: self.icon,
            guidItem: TRAY_ICON_GUID,
            ..Default::default()
        };
        write_utf16(&mut data.szTip, tooltip_text(remote_state().0));
        data
    }
}

impl Drop for StatusTray {
    fn drop(&mut self) {
        unsafe {
            SMOKE_TRAY_QUIT_MESSAGE.store(0, Ordering::Release);
            if self.added {
                let _ = Shell_NotifyIconW(NIM_DELETE, &self.notify_data());
            }
            let _ = DestroyWindow(self.hwnd);
            let _ = DestroyIcon(self.icon);
        }
    }
}

fn write_utf16<const N: usize>(target: &mut [u16; N], value: &str) {
    target.fill(0);
    for (slot, value) in target
        .iter_mut()
        .take(N.saturating_sub(1))
        .zip(value.encode_utf16())
    {
        *slot = value;
    }
}

unsafe extern "system" fn tray_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == TASKBAR_CREATED_MESSAGE.load(Ordering::Acquire) {
        TASKBAR_READD_REQUESTED.store(true, Ordering::Release);
        return LRESULT(0);
    }
    let smoke_tray_quit = SMOKE_TRAY_QUIT_MESSAGE.load(Ordering::Acquire);
    if smoke_tray_quit != 0 && message == smoke_tray_quit {
        // The disposable Windows qualification runner opens the real native menu first. Ending
        // that process-owned menu on its UI thread and enqueueing the same finite command avoids
        // global keyboard/mouse input and proves the exact graceful-Quit route without touching
        // whichever foreign application happens to be focused.
        let _ = EndMenu();
        enqueue_menu_selection(QUIT_COMMAND_ID);
        return LRESULT(0);
    }
    if message == TRAY_CALLBACK_MESSAGE {
        let event = (lparam.0 as u32) & 0xffff;
        if matches!(event, WM_CONTEXTMENU | NIN_SELECT | NIN_KEYSELECT) {
            show_menu(hwnd, point_from_callback(wparam));
        }
        return LRESULT(0);
    }
    if message == WM_DESTROY {
        return LRESULT(0);
    }
    DefWindowProcW(hwnd, message, wparam, lparam)
}

unsafe fn show_menu(hwnd: HWND, mut point: POINT) {
    if point.x == -1 && point.y == -1 && GetCursorPos(&mut point).is_err() {
        return;
    }
    let Ok(menu) = CreatePopupMenu() else {
        return;
    };
    let result = (|| -> windows::core::Result<()> {
        let (indicator, host) = remote_state();
        if let Some(label) = remote_status_label(indicator, &host) {
            AppendMenuW(
                menu,
                MF_STRING | MF_GRAYED,
                REMOTE_STATUS_ID,
                PCWSTR(wide(&label).as_ptr()),
            )?;
            AppendMenuW(
                menu,
                MF_STRING,
                DISCONNECT_REMOTE_COMMAND_ID,
                PCWSTR(wide("Disconnect remote control").as_ptr()),
            )?;
            AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        }
        AppendMenuW(
            menu,
            MF_STRING,
            CONFIGURE_COMMAND_ID,
            PCWSTR(wide("Configure Goose…").as_ptr()),
        )?;
        AppendMenuW(
            menu,
            MF_STRING,
            UPDATE_COMMAND_ID,
            PCWSTR(wide("Update Goose…").as_ptr()),
        )?;
        AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null())?;
        AppendMenuW(
            menu,
            MF_STRING,
            QUIT_COMMAND_ID,
            PCWSTR(wide("Quit Goose").as_ptr()),
        )?;
        let _ = SetForegroundWindow(hwnd);
        let selected = TrackPopupMenu(
            menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            None,
        );
        enqueue_menu_selection(selected.0 as usize);
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_GUID,
            guidItem: TRAY_ICON_GUID,
            ..Default::default()
        };
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        let _ = Shell_NotifyIconW(NIM_SETFOCUS, &data);
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));
        Ok(())
    })();
    let _ = DestroyMenu(menu);
    if let Err(error) = result {
        eprintln!("honk300: Windows tray menu could not open ({error})");
    }
}

fn command_for_menu_selection(selection: usize) -> Option<ControlSurfaceCommand> {
    match selection {
        CONFIGURE_COMMAND_ID => Some(ControlSurfaceCommand::Configure),
        UPDATE_COMMAND_ID => Some(ControlSurfaceCommand::Update),
        QUIT_COMMAND_ID => Some(ControlSurfaceCommand::Quit),
        DISCONNECT_REMOTE_COMMAND_ID => Some(ControlSurfaceCommand::DisconnectRemote),
        _ => None,
    }
}

fn enqueue_menu_selection(selection: usize) {
    if let Some(command) = command_for_menu_selection(selection) {
        with_commands(|commands| commands.push_back(command));
    }
}

fn register_smoke_tray_quit_message() -> windows::core::Result<u32> {
    let token = std::env::var_os("HONK300_WINDOWS_SMOKE_TRAY_QUIT_TOKEN");
    let Some(message_name) = smoke_tray_quit_message_name(token.as_deref())? else {
        return Ok(0);
    };
    let message_name = wide(&message_name);
    let message = unsafe { RegisterWindowMessageW(PCWSTR(message_name.as_ptr())) };
    if message == 0 {
        Err(Error::from_win32())
    } else {
        Ok(message)
    }
}

fn smoke_tray_quit_message_name(token: Option<&OsStr>) -> windows::core::Result<Option<String>> {
    let Some(token) = token else {
        return Ok(None);
    };
    if token.is_empty() {
        return Ok(None);
    }
    let token = token
        .to_str()
        .filter(|token| {
            (32..=64).contains(&token.len())
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
        .ok_or_else(|| failure("invalid Windows tray smoke token"))?;
    Ok(Some(format!("Honk300SmokeTrayQuit-{token}")))
}

fn point_from_callback(value: WPARAM) -> POINT {
    let packed = value.0 as u32;
    POINT {
        x: (packed as u16 as i16) as i32,
        y: ((packed >> 16) as u16 as i16) as i32,
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn create_status_icon(hwnd: HWND) -> windows::core::Result<HICON> {
    // Resource 1 is the same multi-resolution ICO embedded in the app, CLI and
    // settings window. Ask Windows for the native small-icon size and retain
    // our own handle so the existing DestroyIcon lifecycle stays authoritative.
    unsafe {
        let module = GetModuleHandleW(None)?;
        let dpi = GetDpiForWindow(hwnd);
        if dpi == 0 {
            return Err(failure("could not determine tray icon DPI"));
        }
        let handle = LoadImageW(
            HINSTANCE(module.0),
            PCWSTR(std::ptr::without_provenance(1)), // MAKEINTRESOURCEW(1), never dereferenced.
            IMAGE_ICON,
            GetSystemMetricsForDpi(SM_CXSMICON, dpi),
            GetSystemMetricsForDpi(SM_CYSMICON, dpi),
            LR_DEFAULTCOLOR,
        )?;
        Ok(HICON(handle.0))
    }
}

fn failure(message: impl AsRef<str>) -> Error {
    Error::new(windows::core::HRESULT(0x8000_4005u32 as i32), message)
}

#[cfg(test)]
mod tests {
    use super::{
        command_for_menu_selection, point_from_callback, remote_status_label,
        smoke_tray_quit_message_name, tooltip_text, write_utf16, ADD_FAILURE_NOTICE_AFTER,
        ADD_RETRY_DELAY, CONFIGURE_COMMAND_ID, DEGRADED_ADD_RETRY_DELAY,
        DISCONNECT_REMOTE_COMMAND_ID, QUIT_COMMAND_ID, REMOTE_STATUS_ID, UPDATE_COMMAND_ID,
    };
    use honk_control::{ControlSurfaceCommand, RemoteIndicator};
    use std::ffi::OsStr;
    use windows::Win32::Foundation::WPARAM;

    #[test]
    fn initial_registration_retry_is_nonblocking_and_reports_bounded_degradation() {
        assert_eq!(ADD_RETRY_DELAY, std::time::Duration::from_millis(100));
        assert_eq!(ADD_FAILURE_NOTICE_AFTER, std::time::Duration::from_secs(3));
        assert_eq!(DEGRADED_ADD_RETRY_DELAY, std::time::Duration::from_secs(1));
        assert!(ADD_FAILURE_NOTICE_AFTER < std::time::Duration::from_secs(10));
    }

    #[test]
    fn v4_callback_coordinates_preserve_signed_virtual_desktop_points() {
        let packed = ((1200u16 as usize) << 16) | ((-320i16 as u16) as usize);
        assert_eq!(point_from_callback(WPARAM(packed)).x, -320);
        assert_eq!(point_from_callback(WPARAM(packed)).y, 1200);
    }

    #[test]
    fn accessible_tooltip_is_nul_terminated() {
        let mut target = [0u16; 128];
        write_utf16(&mut target, "Honk300 controls");
        let length = "Honk300 controls".encode_utf16().count();
        assert_eq!(target[length], 0);
        assert_eq!(
            String::from_utf16(&target[..length]).unwrap(),
            "Honk300 controls"
        );
    }

    #[test]
    fn native_menu_and_ci_hook_share_the_finite_command_mapping() {
        assert_eq!(
            command_for_menu_selection(CONFIGURE_COMMAND_ID),
            Some(ControlSurfaceCommand::Configure)
        );
        assert_eq!(
            command_for_menu_selection(UPDATE_COMMAND_ID),
            Some(ControlSurfaceCommand::Update)
        );
        assert_eq!(
            command_for_menu_selection(QUIT_COMMAND_ID),
            Some(ControlSurfaceCommand::Quit)
        );
        assert_eq!(
            command_for_menu_selection(DISCONNECT_REMOTE_COMMAND_ID),
            Some(ControlSurfaceCommand::DisconnectRemote)
        );
        assert_eq!(command_for_menu_selection(REMOTE_STATUS_ID), None);
        assert_eq!(command_for_menu_selection(0), None);
        assert_eq!(command_for_menu_selection(usize::MAX), None);
    }

    #[test]
    fn remote_control_is_always_visible_while_active() {
        assert_eq!(remote_status_label(RemoteIndicator::Off, "relay"), None);
        assert!(
            remote_status_label(RemoteIndicator::Connected, "relay.example.com")
                .unwrap()
                .contains("relay.example.com")
        );
        assert!(remote_status_label(RemoteIndicator::Connecting, "relay").is_some());
        assert_eq!(tooltip_text(RemoteIndicator::Off), "Goose controls");
        assert!(tooltip_text(RemoteIndicator::Connected).contains("REMOTE CONTROL ON"));
        for indicator in [
            RemoteIndicator::Off,
            RemoteIndicator::Connecting,
            RemoteIndicator::Connected,
        ] {
            assert!(tooltip_text(indicator).encode_utf16().count() < 128);
        }
    }

    #[test]
    fn smoke_tray_quit_hook_ignores_absent_or_empty_environment_but_rejects_bad_tokens() {
        assert_eq!(smoke_tray_quit_message_name(None).unwrap(), None);
        assert_eq!(
            smoke_tray_quit_message_name(Some(OsStr::new(""))).unwrap(),
            None
        );
        assert_eq!(
            smoke_tray_quit_message_name(Some(OsStr::new("0123456789abcdef0123456789abcdef")))
                .unwrap()
                .as_deref(),
            Some("Honk300SmokeTrayQuit-0123456789abcdef0123456789abcdef")
        );
        assert!(smoke_tray_quit_message_name(Some(OsStr::new("too-short"))).is_err());
        assert!(
            smoke_tray_quit_message_name(Some(OsStr::new("0123456789abcdef0123456789abcde_")))
                .is_err()
        );
    }
}
