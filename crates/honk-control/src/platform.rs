#![cfg_attr(not(windows), allow(dead_code))]

use super::protocol::{ControlCommand, ControlResponse};
use std::io;
use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

/// How long the transport waits for the sim to answer a command before giving up.
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(2);

#[cfg(any(test, unix))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpcNodeKind {
    RuntimeDirectory,
    Socket,
}

#[cfg(any(test, unix))]
fn owner_only_mode(kind: IpcNodeKind) -> u32 {
    match kind {
        IpcNodeKind::RuntimeDirectory => 0o700,
        IpcNodeKind::Socket => 0o600,
    }
}

#[cfg(any(test, unix))]
fn peer_identity_allowed(server_identity: u32, peer_identity: u32) -> bool {
    server_identity == peer_identity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SingletonStatus {
    Acquired,
    AlreadyRunning,
}

/// A decoded command paired with a one-shot channel back to the waiting transport.
///
/// The server thread hands this to the sim and blocks (briefly) on the response, so a caller
/// of `do <action>` or `reload` learns the real outcome instead of a blanket "received".
pub struct ControlRequest {
    command: ControlCommand,
    responder: Sender<ControlResponse>,
}

impl ControlRequest {
    pub(crate) fn new(command: ControlCommand, responder: Sender<ControlResponse>) -> Self {
        Self { command, responder }
    }

    /// The requested command.
    pub fn command(&self) -> ControlCommand {
        self.command
    }

    /// Answer the waiting transport with this command's outcome. Consumes the request so each
    /// command is answered once; if the transport already timed out, the send is dropped.
    pub fn respond(self, response: ControlResponse) {
        let _ = self.responder.send(response);
    }
}

/// Hand a decoded command to the sim and block until it answers (or the wait times out).
#[derive(Debug, Clone, PartialEq, Eq)]
struct DispatchOutcome {
    response: ControlResponse,
    hard_stop: bool,
}

fn dispatch(tx: &Sender<ControlRequest>, command: ControlCommand) -> DispatchOutcome {
    // A forced stop must not depend on the renderer, native menu, compositor, or AppKit event
    // loop making progress. The authenticated transport thread acknowledges it directly and then
    // terminates the process after writing the response. Ordinary stops remain engine-owned so
    // they can perform the shared walk-off animation.
    if command == ControlCommand::ForceStop {
        return DispatchOutcome {
            response: ControlResponse::Ok,
            hard_stop: true,
        };
    }

    let (resp_tx, resp_rx) = mpsc::channel();
    if tx.send(ControlRequest::new(command, resp_tx)).is_err() {
        return DispatchOutcome {
            response: ControlResponse::Err("SERVER_CLOSED".into()),
            hard_stop: false,
        };
    }
    let response = resp_rx
        .recv_timeout(RESPONSE_TIMEOUT)
        .unwrap_or_else(|_| ControlResponse::Err("TIMEOUT".into()));
    DispatchOutcome {
        response,
        hard_stop: false,
    }
}

fn finish_hard_stop() -> ! {
    // `process::exit` deliberately skips stack unwinding. Kernel-owned singleton, pipe/socket,
    // window, and process resources are still released. That is the exact contract requested by
    // `stop --force`; graceful cleanup belongs to the unforced path.
    std::process::exit(0)
}

pub struct Singleton {
    _imp: imp::Singleton,
}

impl Singleton {
    pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
        imp::Singleton::acquire().map(|(imp, status)| (Self { _imp: imp }, status))
    }
}

/// Exclusive ownership of a user-initiated update-helper transaction.
///
/// This guard is independent of the runtime singleton and [`LifecycleLease`]: it prevents two
/// tray helpers from racing one another before either helper reaches the lifecycle transaction.
/// Its platform lock is released by the kernel if the helper exits or crashes.
pub struct UpdateGuard {
    _imp: imp::UpdateGuard,
}

impl UpdateGuard {
    /// Try to become the one active update helper for the current user.
    pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
        imp::UpdateGuard::acquire().map(|(imp, status)| (Self { _imp: imp }, status))
    }
}

/// Exclusive ownership of the process singleton for a managed lifecycle transaction.
///
/// Unlike [`wait_for_shutdown`], this guard deliberately retains the singleton after the old
/// runtime exits. Keeping it alive across install/update/uninstall closes the race where a login
/// item or concurrent `start` could launch between the shutdown probe and on-disk replacement.
pub struct LifecycleLease {
    _singleton: Singleton,
    was_running: bool,
}

impl LifecycleLease {
    /// Ask any active runtime to walk off-screen, then retain exclusive singleton ownership.
    pub fn acquire() -> io::Result<Self> {
        let was_running = stop_runtime_for_lifecycle_with(|| send_command(ControlCommand::Stop))?;
        let singleton = wait_for_shutdown_lease()?;
        Ok(Self {
            _singleton: singleton,
            was_running,
        })
    }

    /// Whether an active command server accepted the stop request before the lease was acquired.
    pub const fn was_running(&self) -> bool {
        self.was_running
    }
}

pub struct CommandServer {
    _imp: imp::CommandServer,
    rx: Receiver<ControlRequest>,
}

impl CommandServer {
    pub fn start() -> io::Result<Self> {
        let (tx, rx) = mpsc::channel();
        let imp = imp::CommandServer::start(tx)?;
        Ok(Self { _imp: imp, rx })
    }

    pub fn try_recv(&self) -> Option<ControlRequest> {
        self.rx.try_recv().ok()
    }
}

pub fn send_command(command: ControlCommand) -> io::Result<ControlResponse> {
    imp::send_command(command)
}

// A graceful edge walk can take several seconds from the middle of a large display. Keep the
// restart-safety wait long enough for that animation while still failing a genuinely stuck stop.
const SHUTDOWN_PROBE_ATTEMPTS: usize = 1_200;
const SHUTDOWN_PROBE_INTERVAL: Duration = Duration::from_millis(25);

/// Wait until the runtime has actually released its singleton after acknowledging `Stop`.
///
/// A stop response is sent just before the runtime unwinds. Waiting on the same singleton used by
/// startup closes the small interval where an immediate restart could see the old process and
/// exit, only for that old process to finish a moment later and leave no instance running.
pub fn wait_for_shutdown() -> io::Result<()> {
    drop(wait_for_shutdown_lease()?);
    Ok(())
}

/// Wait for shutdown and return the acquired singleton instead of releasing it immediately.
pub fn wait_for_shutdown_lease() -> io::Result<Singleton> {
    wait_for_shutdown_lease_with(SHUTDOWN_PROBE_ATTEMPTS, SHUTDOWN_PROBE_INTERVAL, || {
        Singleton::acquire()
    })
}

fn wait_for_shutdown_lease_with<T>(
    attempts: usize,
    interval: Duration,
    mut probe: impl FnMut() -> io::Result<(T, SingletonStatus)>,
) -> io::Result<T> {
    for attempt in 0..attempts {
        let (guard, status) = probe()?;
        if status == SingletonStatus::Acquired {
            return Ok(guard);
        }
        drop(guard);
        if attempt + 1 < attempts {
            std::thread::sleep(interval);
        }
    }
    Err(io::Error::new(
        io::ErrorKind::TimedOut,
        "runtime accepted stop but did not release its singleton within thirty seconds",
    ))
}

fn stop_runtime_for_lifecycle_with(
    stop: impl FnOnce() -> io::Result<ControlResponse>,
) -> io::Result<bool> {
    match stop() {
        Ok(ControlResponse::Ok) => Ok(true),
        Ok(ControlResponse::Err(code)) => Err(io::Error::other(format!(
            "runtime rejected stop before lifecycle mutation: {code}"
        ))),
        Ok(
            ControlResponse::Status(_)
            | ControlResponse::Session(_)
            | ControlResponse::Wayland(_)
            | ControlResponse::Presence(_)
            | ControlResponse::Toy(_)
            | ControlResponse::Snapshot(_)
            | ControlResponse::Toys(_),
        ) => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "runtime returned status to lifecycle stop request",
        )),
        Err(error)
            if matches!(
                error.kind(),
                io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused
            ) =>
        {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RuntimeStatus;
    use std::cell::Cell;
    use std::rc::Rc;

    struct TestGuard(Rc<Cell<bool>>);

    impl Drop for TestGuard {
        fn drop(&mut self) {
            self.0.set(true);
        }
    }

    #[test]
    fn local_ipc_nodes_use_owner_only_modes() {
        assert_eq!(owner_only_mode(IpcNodeKind::RuntimeDirectory), 0o700);
        assert_eq!(owner_only_mode(IpcNodeKind::Socket), 0o600);
    }

    #[test]
    fn ipc_rejects_a_different_peer_identity() {
        let server_identity = 1000_u32;
        let peer_identity = 1001_u32;
        assert!(!peer_identity_allowed(server_identity, peer_identity));
        assert!(peer_identity_allowed(server_identity, server_identity));
    }

    #[test]
    fn control_request_delivers_response_to_the_waiter() {
        // The transport hands the sim a request, then blocks until the sim answers; this is the
        // primitive that lets `do <action>` report the real outcome instead of a blind "OK".
        let (tx, rx) = mpsc::channel::<ControlRequest>();
        let (resp_tx, resp_rx) = mpsc::channel();
        tx.send(ControlRequest::new(ControlCommand::Stop, resp_tx))
            .unwrap();

        let request = rx.try_recv().unwrap();
        assert_eq!(request.command(), ControlCommand::Stop);
        request.respond(ControlResponse::Err("BUSY".into()));

        assert_eq!(resp_rx.recv().unwrap(), ControlResponse::Err("BUSY".into()));
    }

    #[test]
    fn stop_waits_until_the_singleton_is_released_before_returning() {
        let mut probes = [
            SingletonStatus::AlreadyRunning,
            SingletonStatus::AlreadyRunning,
            SingletonStatus::Acquired,
        ]
        .into_iter();

        let returned_dropped = Rc::new(Cell::new(false));
        let guard = wait_for_shutdown_lease_with(4, Duration::ZERO, || {
            let status = probes.next().expect("shutdown probe count");
            let dropped = if status == SingletonStatus::Acquired {
                Rc::clone(&returned_dropped)
            } else {
                Rc::new(Cell::new(false))
            };
            Ok((TestGuard(dropped), status))
        })
        .unwrap();
        assert!(
            !returned_dropped.get(),
            "the acquired singleton must remain held after the wait"
        );
        drop(guard);
        assert!(returned_dropped.get());
    }

    #[test]
    fn graceful_stop_wait_budget_is_thirty_seconds_and_accepts_the_final_probe() {
        assert_eq!(SHUTDOWN_PROBE_ATTEMPTS, 1_200);
        assert_eq!(SHUTDOWN_PROBE_INTERVAL, Duration::from_millis(25));
        assert_eq!(
            (SHUTDOWN_PROBE_ATTEMPTS - 1) as u128 * SHUTDOWN_PROBE_INTERVAL.as_millis(),
            29_975
        );

        let mut probes = 0usize;
        wait_for_shutdown_lease_with(SHUTDOWN_PROBE_ATTEMPTS, Duration::ZERO, || {
            probes += 1;
            let status = if probes == SHUTDOWN_PROBE_ATTEMPTS {
                SingletonStatus::Acquired
            } else {
                SingletonStatus::AlreadyRunning
            };
            Ok(((), status))
        })
        .expect("the final bounded probe still preserves immediate-restart safety");
        assert_eq!(probes, SHUTDOWN_PROBE_ATTEMPTS);
    }

    #[test]
    fn stop_reports_a_bounded_timeout_when_the_runtime_never_releases() {
        let err = wait_for_shutdown_lease_with(2, Duration::ZERO, || {
            Ok(((), SingletonStatus::AlreadyRunning))
        })
        .unwrap_err();

        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    }

    #[test]
    fn lifecycle_stop_is_fail_closed_but_accepts_an_absent_runtime() {
        assert!(stop_runtime_for_lifecycle_with(|| Ok(ControlResponse::Ok)).unwrap());
        assert!(!stop_runtime_for_lifecycle_with(|| Err(io::Error::new(
            io::ErrorKind::NotFound,
            "no command server",
        )))
        .unwrap());
        assert!(
            stop_runtime_for_lifecycle_with(|| Ok(ControlResponse::Err("BUSY".into()))).is_err()
        );
        assert!(
            stop_runtime_for_lifecycle_with(|| Ok(ControlResponse::Status(
                RuntimeStatus::not_running(),
            )))
            .is_err()
        );
    }

    #[test]
    fn force_stop_bypasses_a_blocked_or_closed_sim_thread() {
        let (tx, rx) = mpsc::channel();
        drop(rx);

        let outcome = dispatch(&tx, ControlCommand::ForceStop);

        assert_eq!(outcome.response, ControlResponse::Ok);
        assert!(outcome.hard_stop);
    }

    #[test]
    fn graceful_stop_still_requires_the_sim_thread() {
        let (tx, rx) = mpsc::channel();
        drop(rx);

        let outcome = dispatch(&tx, ControlCommand::Stop);

        assert_eq!(
            outcome.response,
            ControlResponse::Err("SERVER_CLOSED".into())
        );
        assert!(!outcome.hard_stop);
    }
}

#[cfg(windows)]
mod imp {
    use super::{
        dispatch, finish_hard_stop, ControlCommand, ControlRequest, ControlResponse,
        DispatchOutcome, SingletonStatus,
    };
    use std::collections::hash_map::DefaultHasher;
    use std::fs::OpenOptions;
    use std::hash::{Hash, Hasher};
    use std::io::{self, Read, Write};
    use std::os::windows::io::FromRawHandle;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::{Duration, Instant};
    use windows::core::{PCWSTR, PWSTR};
    use windows::Win32::Foundation::{
        CloseHandle, GetLastError, LocalFree, ERROR_ALREADY_EXISTS, ERROR_NO_DATA,
        ERROR_PIPE_CONNECTED, ERROR_PIPE_LISTENING, HANDLE, HLOCAL,
    };
    use windows::Win32::Security::Authorization::{
        ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW,
        SDDL_REVISION_1,
    };
    use windows::Win32::Security::{
        GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
        TOKEN_USER,
    };
    use windows::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
    use windows::Win32::System::Pipes::{
        ConnectNamedPipe, CreateNamedPipeW, NAMED_PIPE_MODE, PIPE_NOWAIT, PIPE_READMODE_MESSAGE,
        PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_MESSAGE, PIPE_UNLIMITED_INSTANCES,
    };
    use windows::Win32::System::Threading::{CreateMutexW, GetCurrentProcess, OpenProcessToken};

    const PIPE_CONNECT_SLICE: Duration = Duration::from_millis(250);
    const CLIENT_CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
    const PIPE_READ_TIMEOUT: Duration = Duration::from_secs(2);

    pub struct Singleton {
        handle: Option<HANDLE>,
    }

    impl Singleton {
        pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
            let name = wide_null(&mutex_name());
            let handle = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }
                .map_err(|_| io::Error::last_os_error())?;
            let already_running = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
            let status = if already_running {
                SingletonStatus::AlreadyRunning
            } else {
                SingletonStatus::Acquired
            };
            Ok((
                Self {
                    handle: Some(handle),
                },
                status,
            ))
        }
    }

    impl Drop for Singleton {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                unsafe {
                    let _ = CloseHandle(handle);
                }
            }
        }
    }

    pub struct UpdateGuard {
        handle: Option<HANDLE>,
    }

    impl UpdateGuard {
        pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
            Self::acquire_named(&update_mutex_name())
        }

        fn acquire_named(name: &str) -> io::Result<(Self, SingletonStatus)> {
            let name = wide_null(name);
            let handle = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) }
                .map_err(|_| io::Error::last_os_error())?;
            if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
                unsafe {
                    let _ = CloseHandle(handle);
                }
                return Ok((Self { handle: None }, SingletonStatus::AlreadyRunning));
            }
            Ok((
                Self {
                    handle: Some(handle),
                },
                SingletonStatus::Acquired,
            ))
        }
    }

    impl Drop for UpdateGuard {
        fn drop(&mut self) {
            if let Some(handle) = self.handle.take() {
                unsafe {
                    let _ = CloseHandle(handle);
                }
            }
        }
    }

    pub struct CommandServer {
        shutdown: Arc<AtomicBool>,
        join: Option<JoinHandle<()>>,
    }

    impl CommandServer {
        pub fn start(tx: Sender<ControlRequest>) -> io::Result<Self> {
            let shutdown = Arc::new(AtomicBool::new(false));
            let thread_shutdown = Arc::clone(&shutdown);
            let join = thread::spawn(move || server_loop(tx, thread_shutdown));
            Ok(Self {
                shutdown,
                join: Some(join),
            })
        }
    }

    impl Drop for CommandServer {
        fn drop(&mut self) {
            self.shutdown.store(true, Ordering::SeqCst);
            let _ = send_command(ControlCommand::Reload);
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    pub fn send_command(command: ControlCommand) -> io::Result<ControlResponse> {
        let path = pipe_path();
        let mut pipe = open_pipe_bounded(&path, CLIENT_CONNECT_TIMEOUT)?;
        pipe.write_all(command.encode().as_bytes())?;
        pipe.flush()?;
        let mut buf = [0u8; 128];
        let len = read_bounded(&mut pipe, &mut buf, PIPE_READ_TIMEOUT)?;
        ControlResponse::decode(&buf[..len])
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn open_pipe_bounded(path: &str, timeout: Duration) -> io::Result<std::fs::File> {
        let deadline = Instant::now() + timeout;
        let mut last_error = match OpenOptions::new().read(true).write(true).open(path) {
            Ok(pipe) => return Ok(pipe),
            Err(err) => err,
        };
        loop {
            if Instant::now() >= deadline {
                break;
            }
            thread::sleep(Duration::from_millis(10));
            match OpenOptions::new().read(true).write(true).open(path) {
                Ok(pipe) => return Ok(pipe),
                Err(err) => last_error = err,
            }
        }
        if last_error.kind() == io::ErrorKind::NotFound {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "no running honk300 instance",
            ))
        } else {
            Err(last_error)
        }
    }

    fn server_loop(tx: Sender<ControlRequest>, shutdown: Arc<AtomicBool>) {
        while !shutdown.load(Ordering::SeqCst) {
            let pipe = match create_pipe() {
                Ok(pipe) => pipe,
                Err(_) => {
                    thread::sleep(Duration::from_millis(100));
                    continue;
                }
            };
            let connected = wait_for_client(pipe, &shutdown);
            if shutdown.load(Ordering::SeqCst) {
                unsafe {
                    let _ = CloseHandle(pipe);
                }
                break;
            }
            if !connected {
                unsafe {
                    let _ = CloseHandle(pipe);
                }
                continue;
            }

            let mut file = unsafe { std::fs::File::from_raw_handle(pipe.0) };
            let mut buf = [0u8; 128];
            let outcome = match read_bounded(&mut file, &mut buf, PIPE_READ_TIMEOUT) {
                Ok(0) => DispatchOutcome {
                    response: ControlResponse::Err("EMPTY".into()),
                    hard_stop: false,
                },
                Ok(len) => match ControlCommand::decode(&buf[..len]) {
                    Ok(command) => dispatch(&tx, command),
                    Err(err) => DispatchOutcome {
                        response: ControlResponse::Err(protocol_code(&err.to_string())),
                        hard_stop: false,
                    },
                },
                Err(_) => DispatchOutcome {
                    response: ControlResponse::Err("READ_FAILED".into()),
                    hard_stop: false,
                },
            };
            let _ = file.write_all(outcome.response.encode().as_bytes());
            if outcome.hard_stop {
                drop(file);
                finish_hard_stop();
            }
            let _ = file.flush();
        }
    }

    fn create_pipe() -> io::Result<HANDLE> {
        let name = wide_null(&pipe_path());
        let security = PipeSecurity::new()?;
        let attributes = security.attributes();
        let handle = unsafe {
            CreateNamedPipeW(
                PCWSTR(name.as_ptr()),
                PIPE_ACCESS_DUPLEX,
                pipe_mode(),
                PIPE_UNLIMITED_INSTANCES,
                128,
                128,
                0,
                Some(&attributes),
            )
        };
        if handle.is_invalid() {
            Err(io::Error::last_os_error())
        } else {
            Ok(handle)
        }
    }

    fn mutex_name() -> String {
        format!("Local\\honk300-{}", user_hash())
    }

    fn update_mutex_name() -> String {
        format!("Local\\honk300-update-{}", user_hash())
    }

    fn pipe_path() -> String {
        format!(r"\\.\pipe\honk300-{}", user_hash())
    }

    fn user_hash() -> u64 {
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "unknown".into());
        let domain = std::env::var("USERDOMAIN").unwrap_or_default();
        let mut hasher = DefaultHasher::new();
        domain.hash(&mut hasher);
        user.hash(&mut hasher);
        hasher.finish()
    }

    fn wide_null(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    fn protocol_code(message: &str) -> String {
        message
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect()
    }

    fn pipe_mode() -> NAMED_PIPE_MODE {
        PIPE_TYPE_MESSAGE | PIPE_READMODE_MESSAGE | PIPE_NOWAIT | PIPE_REJECT_REMOTE_CLIENTS
    }

    fn wait_for_client(pipe: HANDLE, shutdown: &AtomicBool) -> bool {
        let deadline = Instant::now() + PIPE_CONNECT_SLICE;
        loop {
            if shutdown.load(Ordering::SeqCst) {
                return false;
            }
            if unsafe { ConnectNamedPipe(pipe, None).is_ok() } {
                return true;
            }
            let error = unsafe { GetLastError() };
            if error == ERROR_PIPE_CONNECTED {
                return true;
            }
            if error != ERROR_PIPE_LISTENING && error != ERROR_NO_DATA {
                return false;
            }
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }

    fn read_bounded(
        file: &mut std::fs::File,
        buf: &mut [u8],
        timeout: Duration,
    ) -> io::Result<usize> {
        read_bounded_with(buf, timeout, |buffer| file.read(buffer))
    }

    fn read_bounded_with(
        buf: &mut [u8],
        timeout: Duration,
        mut read: impl FnMut(&mut [u8]) -> io::Result<usize>,
    ) -> io::Result<usize> {
        let deadline = Instant::now() + timeout;
        loop {
            match read(buf) {
                // A byte-mode wrapper over a PIPE_NOWAIT named pipe can surface the
                // connected-but-not-yet-written state as a successful zero-byte read. It is not
                // EOF: returning immediately lets the server reject a command as EMPTY even
                // though the client writes its frame a scheduler slice later.
                Ok(0) => {
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "named-pipe peer did not send data before the deadline",
                        ));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Ok(len) => return Ok(len),
                Err(err)
                    if err.kind() == io::ErrorKind::WouldBlock
                        || err.raw_os_error() == Some(ERROR_NO_DATA.0 as i32) =>
                {
                    if Instant::now() >= deadline {
                        return Err(io::Error::new(
                            io::ErrorKind::TimedOut,
                            "named-pipe peer did not send data before the deadline",
                        ));
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(err) => return Err(err),
            }
        }
    }

    struct PipeSecurity {
        descriptor: PSECURITY_DESCRIPTOR,
    }

    impl PipeSecurity {
        fn new() -> io::Result<Self> {
            let sid = current_user_sid_string()?;
            let sddl = wide_null(&pipe_security_sddl(&sid));
            let mut descriptor = PSECURITY_DESCRIPTOR::default();
            unsafe {
                ConvertStringSecurityDescriptorToSecurityDescriptorW(
                    PCWSTR(sddl.as_ptr()),
                    SDDL_REVISION_1,
                    &mut descriptor,
                    None,
                )
            }
            .map_err(|_| io::Error::last_os_error())?;
            Ok(Self { descriptor })
        }

        fn attributes(&self) -> SECURITY_ATTRIBUTES {
            SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: self.descriptor.0,
                bInheritHandle: false.into(),
            }
        }
    }

    impl Drop for PipeSecurity {
        fn drop(&mut self) {
            if !self.descriptor.0.is_null() {
                unsafe {
                    let _ = LocalFree(HLOCAL(self.descriptor.0));
                }
            }
        }
    }

    fn current_user_sid_string() -> io::Result<String> {
        unsafe {
            let mut token = HANDLE::default();
            OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token)
                .map_err(|_| io::Error::last_os_error())?;

            let result = (|| {
                let mut required = 0u32;
                let _ = GetTokenInformation(token, TokenUser, None, 0, &mut required);
                if required == 0 {
                    return Err(io::Error::last_os_error());
                }
                let mut storage = vec![0u8; required as usize];
                GetTokenInformation(
                    token,
                    TokenUser,
                    Some(storage.as_mut_ptr().cast()),
                    required,
                    &mut required,
                )
                .map_err(|_| io::Error::last_os_error())?;
                let token_user = &*(storage.as_ptr().cast::<TOKEN_USER>());
                let mut string_sid = PWSTR::null();
                ConvertSidToStringSidW(token_user.User.Sid, &mut string_sid)
                    .map_err(|_| io::Error::last_os_error())?;
                let sid = string_sid
                    .to_string()
                    .map_err(|_| io::Error::last_os_error());
                let _ = LocalFree(HLOCAL(string_sid.0.cast()));
                sid
            })();

            let _ = CloseHandle(token);
            result
        }
    }

    fn pipe_security_sddl(user_sid: &str) -> String {
        format!("D:P(A;;GA;;;SY)(A;;GA;;;{user_sid})")
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn update_mutex_is_user_scoped_and_distinct_from_the_runtime_singleton() {
            let name = update_mutex_name();
            assert!(name.starts_with("Local\\honk300-update-"));
            assert_ne!(name, mutex_name());
        }

        #[test]
        fn update_mutex_denies_a_concurrent_helper_and_recovers_after_release() {
            let name = format!(
                "Local\\honk300-update-test-{}-{}",
                std::process::id(),
                Instant::now().elapsed().as_nanos()
            );
            let (first, status) = UpdateGuard::acquire_named(&name).unwrap();
            assert_eq!(status, SingletonStatus::Acquired);

            let (second, status) = UpdateGuard::acquire_named(&name).unwrap();
            assert_eq!(status, SingletonStatus::AlreadyRunning);
            drop(first);

            let (third, status) = UpdateGuard::acquire_named(&name).unwrap();
            assert_eq!(status, SingletonStatus::Acquired);
            drop(second);
            drop(third);
        }

        #[test]
        fn named_pipe_rejects_remote_clients() {
            assert_ne!(pipe_mode().0 & PIPE_REJECT_REMOTE_CLIENTS.0, 0);
        }

        #[test]
        fn named_pipe_acl_is_limited_to_user_and_system() {
            assert_eq!(
                pipe_security_sddl("S-1-5-21-TEST"),
                "D:P(A;;GA;;;SY)(A;;GA;;;S-1-5-21-TEST)"
            );
        }

        #[test]
        fn missing_named_pipe_returns_before_status_ui_deadline() {
            let path = format!(
                r"\\.\pipe\honk300-missing-test-{}-{}",
                std::process::id(),
                Instant::now().elapsed().as_nanos()
            );
            let started = Instant::now();
            assert!(open_pipe_bounded(&path, Duration::from_millis(250)).is_err());
            assert!(
                started.elapsed() < Duration::from_millis(500),
                "missing runtime took {:?}",
                started.elapsed()
            );
        }

        #[test]
        fn connected_zero_byte_pipe_polls_are_not_treated_as_empty_frames() {
            let mut polls = 0_u8;
            let mut buffer = [0_u8; 8];
            let length = read_bounded_with(&mut buffer, Duration::from_secs(1), |destination| {
                polls += 1;
                if polls < 3 {
                    Ok(0)
                } else {
                    destination[..3].copy_from_slice(b"OK\n");
                    Ok(3)
                }
            })
            .expect("a connected nonblocking pipe must wait for its frame");

            assert_eq!(length, 3);
            assert_eq!(&buffer[..length], b"OK\n");
            assert_eq!(polls, 3);
        }

        #[test]
        fn zero_byte_pipe_poll_still_obeys_the_read_deadline() {
            let mut buffer = [0_u8; 8];
            let error = read_bounded_with(&mut buffer, Duration::ZERO, |_| Ok(0))
                .expect_err("a peer that never sends bytes must time out");

            assert_eq!(error.kind(), io::ErrorKind::TimedOut);
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::{
        dispatch, finish_hard_stop, owner_only_mode, peer_identity_allowed, ControlCommand,
        ControlRequest, ControlResponse, DispatchOutcome, IpcNodeKind, SingletonStatus,
    };
    use rustix::fs::{flock, FlockOperation};
    use rustix::io::Errno;
    use socket2::{Domain, SockAddr, Socket, Type};
    use std::fs::{self, File, OpenOptions};
    use std::io::{self, Read, Write};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    #[cfg(target_os = "macos")]
    use std::os::unix::io::AsRawFd;
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::mpsc::Sender;
    use std::sync::Arc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    const CONNECT_TIMEOUT: Duration = Duration::from_millis(250);
    const IO_TIMEOUT: Duration = Duration::from_secs(2);

    /// Single-instance guard backed by an advisory `flock` on a lock file.
    ///
    /// The exclusive lock is held for the lifetime of the process and released by the kernel on
    /// any exit — clean or crashed — so a leftover lock file never falsely reports "already
    /// running". The file is intentionally never unlinked: unlinking it would let a second process
    /// create a fresh inode at the same path and lock that independently, defeating the guard.
    pub struct Singleton {
        _lock: Option<File>,
    }

    impl Singleton {
        pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
            let dir = secure_runtime_dir()?;
            Self::acquire_at(&dir.join("lock"))
        }

        fn acquire_at(path: &Path) -> io::Result<(Self, SingletonStatus)> {
            // Open (creating if needed) without truncating: the file is a lock target only, and a
            // concurrent holder's inode must be preserved so its lock stays meaningful.
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)?;
            match flock(&file, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => Ok((Self { _lock: Some(file) }, SingletonStatus::Acquired)),
                Err(err) if err == Errno::WOULDBLOCK || err == Errno::AGAIN => {
                    Ok((Self { _lock: None }, SingletonStatus::AlreadyRunning))
                }
                Err(err) => Err(io::Error::from(err)),
            }
        }
    }

    pub struct UpdateGuard {
        _lock: Option<File>,
    }

    impl UpdateGuard {
        pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
            let dir = secure_runtime_dir()?;
            Self::acquire_at(&dir.join("update.lock"))
        }

        fn acquire_at(path: &Path) -> io::Result<(Self, SingletonStatus)> {
            let file = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(path)?;
            match flock(&file, FlockOperation::NonBlockingLockExclusive) {
                Ok(()) => Ok((Self { _lock: Some(file) }, SingletonStatus::Acquired)),
                Err(err) if err == Errno::WOULDBLOCK || err == Errno::AGAIN => {
                    Ok((Self { _lock: None }, SingletonStatus::AlreadyRunning))
                }
                Err(err) => Err(io::Error::from(err)),
            }
        }
    }

    pub struct CommandServer {
        shutdown: Arc<AtomicBool>,
        join: Option<JoinHandle<()>>,
        path: PathBuf,
    }

    impl CommandServer {
        pub fn start(tx: Sender<ControlRequest>) -> io::Result<Self> {
            let path = socket_path()?;
            // A crashed prior server can leave a stale socket file that would make `bind` fail with
            // EADDRINUSE. We only reach here after the `flock` singleton was acquired, so no other
            // honk300 owns this path — unlinking any leftover before binding is safe and crash-safe.
            let _ = fs::remove_file(&path);
            let listener = UnixListener::bind(&path)?;
            fs::set_permissions(
                &path,
                fs::Permissions::from_mode(owner_only_mode(IpcNodeKind::Socket)),
            )?;
            verify_owned_mode(&path, IpcNodeKind::Socket)?;
            listener.set_nonblocking(true)?;
            let shutdown = Arc::new(AtomicBool::new(false));
            let thread_shutdown = Arc::clone(&shutdown);
            let join = thread::spawn(move || server_loop(listener, tx, thread_shutdown));
            Ok(Self {
                shutdown,
                join: Some(join),
                path,
            })
        }
    }

    impl Drop for CommandServer {
        fn drop(&mut self) {
            self.shutdown.store(true, Ordering::SeqCst);
            let _ = send_command(ControlCommand::Reload);
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
            let _ = fs::remove_file(&self.path);
        }
    }

    pub fn send_command(command: ControlCommand) -> io::Result<ControlResponse> {
        let path = socket_path()?;
        let socket = Socket::new(Domain::UNIX, Type::STREAM, None)?;
        socket.connect_timeout(&SockAddr::unix(&path)?, CONNECT_TIMEOUT)?;
        let fd: std::os::fd::OwnedFd = socket.into();
        let mut stream = UnixStream::from(fd);
        configure_stream_timeouts(&stream)?;
        validate_peer(&stream)?;
        stream.write_all(command.encode().as_bytes())?;
        stream.flush()?;
        let mut buf = [0u8; 128];
        let len = stream.read(&mut buf)?;
        ControlResponse::decode(&buf[..len])
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))
    }

    fn server_loop(listener: UnixListener, tx: Sender<ControlRequest>, shutdown: Arc<AtomicBool>) {
        while !shutdown.load(Ordering::SeqCst) {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    if shutdown.load(Ordering::SeqCst) {
                        break;
                    }
                    if configure_stream_timeouts(&stream).is_err()
                        || validate_peer(&stream).is_err()
                    {
                        let _ = stream.write_all(
                            ControlResponse::Err("UNAUTHORIZED".into())
                                .encode()
                                .as_bytes(),
                        );
                        continue;
                    }
                    let mut buf = [0u8; 128];
                    let outcome = match stream.read(&mut buf) {
                        Ok(0) => DispatchOutcome {
                            response: ControlResponse::Err("EMPTY".into()),
                            hard_stop: false,
                        },
                        Ok(len) => match ControlCommand::decode(&buf[..len]) {
                            Ok(command) => dispatch(&tx, command),
                            Err(err) => DispatchOutcome {
                                response: ControlResponse::Err(err.to_string()),
                                hard_stop: false,
                            },
                        },
                        Err(_) => DispatchOutcome {
                            response: ControlResponse::Err("READ_FAILED".into()),
                            hard_stop: false,
                        },
                    };
                    let _ = stream.write_all(outcome.response.encode().as_bytes());
                    if outcome.hard_stop {
                        drop(stream);
                        finish_hard_stop();
                    }
                }
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                    thread::sleep(Duration::from_millis(25));
                }
                Err(_) => thread::sleep(Duration::from_millis(100)),
            }
        }
    }

    fn socket_path() -> io::Result<PathBuf> {
        Ok(secure_runtime_dir()?.join("control.sock"))
    }

    fn runtime_dir_path() -> PathBuf {
        let uid = current_uid();
        let preferred = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .map(|dir| dir.join("honk300"))
            .unwrap_or_else(|| std::env::temp_dir().join(format!("honk300-{uid}")));
        choose_runtime_dir(preferred, uid)
    }

    fn choose_runtime_dir(preferred: PathBuf, uid: u32) -> PathBuf {
        if SockAddr::unix(preferred.join("control.sock")).is_ok() {
            preferred
        } else {
            // macOS has a 104-byte sockaddr_un path limit and hosted/managed environments often
            // provide a much longer TMPDIR. /tmp is the portable short fallback. The directory
            // remains UID-namespaced and secure_runtime_dir still rejects symlinks, foreign
            // owners, non-directories, and non-0700 permissions before either lock or socket use.
            PathBuf::from("/tmp").join(format!("honk300-{uid}"))
        }
    }

    fn secure_runtime_dir() -> io::Result<PathBuf> {
        let dir = runtime_dir_path();
        fs::create_dir_all(&dir)?;
        let metadata = fs::symlink_metadata(&dir)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "IPC runtime path is not a real directory: {}",
                    dir.display()
                ),
            ));
        }
        if !peer_identity_allowed(current_uid(), metadata.uid()) {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "IPC runtime directory is owned by another user: {}",
                    dir.display()
                ),
            ));
        }
        fs::set_permissions(
            &dir,
            fs::Permissions::from_mode(owner_only_mode(IpcNodeKind::RuntimeDirectory)),
        )?;
        verify_owned_mode(&dir, IpcNodeKind::RuntimeDirectory)?;
        Ok(dir)
    }

    fn verify_owned_mode(path: &Path, kind: IpcNodeKind) -> io::Result<()> {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink()
            || !peer_identity_allowed(current_uid(), metadata.uid())
            || metadata.mode() & 0o777 != owner_only_mode(kind)
        {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("IPC path is not owner-only: {}", path.display()),
            ));
        }
        Ok(())
    }

    fn configure_stream_timeouts(stream: &UnixStream) -> io::Result<()> {
        stream.set_read_timeout(Some(IO_TIMEOUT))?;
        stream.set_write_timeout(Some(IO_TIMEOUT))
    }

    fn validate_peer(stream: &UnixStream) -> io::Result<()> {
        let peer = peer_uid(stream)?;
        if peer_identity_allowed(current_uid(), peer) {
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "IPC peer belongs to another user",
            ))
        }
    }

    fn current_uid() -> u32 {
        rustix::process::geteuid().as_raw()
    }

    #[cfg(target_os = "linux")]
    fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
        rustix::net::sockopt::socket_peercred(stream)
            .map(|credentials| credentials.uid.as_raw())
            .map_err(io::Error::from)
    }

    #[cfg(target_os = "macos")]
    fn peer_uid(stream: &UnixStream) -> io::Result<u32> {
        let mut uid: libc::uid_t = 0;
        let mut gid: libc::gid_t = 0;
        let status = unsafe { libc::getpeereid(stream.as_raw_fd(), &mut uid, &mut gid) };
        if status == 0 {
            Ok(uid)
        } else {
            Err(io::Error::last_os_error())
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn peer_uid(_stream: &UnixStream) -> io::Result<u32> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "Unix peer credentials are unsupported on this platform",
        ))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn secure_runtime_directory_and_socket_modes_are_owner_only() {
            let dir = PathBuf::from("/tmp").join(format!(
                "h3-{}-{:x}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or_default()
            ));
            fs::create_dir_all(&dir).unwrap();
            fs::set_permissions(
                &dir,
                fs::Permissions::from_mode(owner_only_mode(IpcNodeKind::RuntimeDirectory)),
            )
            .unwrap();
            verify_owned_mode(&dir, IpcNodeKind::RuntimeDirectory).unwrap();

            let socket = dir.join("control.sock");
            let listener = UnixListener::bind(&socket).unwrap();
            fs::set_permissions(
                &socket,
                fs::Permissions::from_mode(owner_only_mode(IpcNodeKind::Socket)),
            )
            .unwrap();
            verify_owned_mode(&socket, IpcNodeKind::Socket).unwrap();

            drop(listener);
            let _ = fs::remove_dir_all(dir);
        }

        #[test]
        fn overlong_runtime_path_uses_short_uid_scoped_fallback() {
            let preferred = PathBuf::from("/").join("x".repeat(256));
            let selected = choose_runtime_dir(preferred, 4242);
            assert_eq!(selected, PathBuf::from("/tmp/honk300-4242"));
            assert!(SockAddr::unix(selected.join("control.sock")).is_ok());
        }

        #[test]
        fn flock_singleton_is_exclusive_and_survives_crash() {
            let dir = std::env::temp_dir().join(format!(
                "honk300-flock-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or_default()
            ));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("lock");

            // First acquirer takes the exclusive advisory lock.
            let (first, status) = Singleton::acquire_at(&path).unwrap();
            assert_eq!(status, SingletonStatus::Acquired);

            // A second acquirer, while the first holds the lock, is denied -> AlreadyRunning.
            let (second, status) = Singleton::acquire_at(&path).unwrap();
            assert_eq!(status, SingletonStatus::AlreadyRunning);
            drop(second);

            // Dropping the holder closes the fd, which releases the kernel lock exactly as a crash
            // would. The lock file still exists on disk, yet a fresh acquire must succeed — proving
            // the old create_new stale-lock false positive is gone.
            drop(first);
            assert!(path.exists());
            let (third, status) = Singleton::acquire_at(&path).unwrap();
            assert_eq!(status, SingletonStatus::Acquired);
            drop(third);

            let _ = fs::remove_dir_all(&dir);
        }

        #[test]
        fn update_flock_is_exclusive_and_recovers_after_release() {
            let dir = std::env::temp_dir().join(format!(
                "honk300-update-flock-test-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or_default()
            ));
            fs::create_dir_all(&dir).unwrap();
            let path = dir.join("update.lock");

            let (first, status) = UpdateGuard::acquire_at(&path).unwrap();
            assert_eq!(status, SingletonStatus::Acquired);
            let (second, status) = UpdateGuard::acquire_at(&path).unwrap();
            assert_eq!(status, SingletonStatus::AlreadyRunning);
            drop(second);
            drop(first);

            assert!(path.exists());
            let (third, status) = UpdateGuard::acquire_at(&path).unwrap();
            assert_eq!(status, SingletonStatus::Acquired);
            drop(third);

            let _ = fs::remove_dir_all(&dir);
        }
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    use super::{ControlCommand, ControlRequest, ControlResponse, SingletonStatus};
    use std::io;
    use std::sync::mpsc::Sender;

    pub struct Singleton;

    impl Singleton {
        pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
            Ok((Self, SingletonStatus::Acquired))
        }
    }

    pub struct UpdateGuard;

    impl UpdateGuard {
        pub fn acquire() -> io::Result<(Self, SingletonStatus)> {
            Ok((Self, SingletonStatus::Acquired))
        }
    }

    pub struct CommandServer;

    impl CommandServer {
        pub fn start(_tx: Sender<ControlRequest>) -> io::Result<Self> {
            Ok(Self)
        }
    }

    pub fn send_command(_command: ControlCommand) -> io::Result<ControlResponse> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "IPC is unsupported on this platform",
        ))
    }
}
