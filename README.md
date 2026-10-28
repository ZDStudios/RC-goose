# Goose

Goose is a clean-room, procedural desktop goose for Windows, macOS, and Linux. It walks
across real monitor layouts, honks, leaves muddy footprints, reacts to the pointer, and performs
bounded desktop pranks. Open **Goose** from your applications menu to start or stop the goose,
change settings, and check for updates. The native application is the primary interface;
the existing Rust services, command line, and terminal editor remain its backend and secondary controls.

**Current stable release:** [v1.11.3](https://github.com/RealEmmettS/goose/releases/tag/v1.11.3),
published from exact commit `c36d8e92847045d8b931618e014ae46cbd5228ab`.
All required publication and fresh-public checks pass; all 47 public assets are independently verified.
Goose opens its graphical controls from application menus and the bare `goose` command,
with recognizable application icons and installed Configure actions. Maple,
oak, birch and ginkgo leaves fade away on their own; sustained rubbing earns hearts
and a short, gentle follow without grabbing the pointer.

The completed refinement also brings one continuously projected goose, expressive
motion, bounded rendering resources, native graphical settings, Linux notes and
pictures, experimental Pi 4/5 64-bit Desktop guidance, and independently qualified
KDE, Sway, Hyprland and GNOME integrations. Each desktop exposes only its proven
capabilities. See [desktop setup](docs/wayland-integrations.md),
[release evidence](docs/readiness/v1.11.3-readiness.md),
[Pi guidance](docs/raspberry-pi.md), and [the refinement audit](docs/refinement-audit.md).

**Goose**, the bare `goose` command, `goose settings`, and tray Configure open the
graphical controls. `goose start` or `goose plz` starts the goose directly;
`goose config` opens the secondary terminal editor.
All routes use the same Rust-owned configuration and lifecycle. The compatible `honk300`
and `honk` aliases remain.

Windows settings startup now recognizes a response from its own verified window within
the existing deadline. All four installed entry routes pass with loaded native controls and successful controllers in 491-695 ms, while saved preferences remain unchanged.
See [the qualification record](docs/readiness/v1.11.3-readiness.md).

The settings-navigation update keeps the current tab visibly highlighted,
keeps scrolling short and smooth, limits the Wayland switch to Linux, and uses the
application artwork in native trays and menus. It also puts both seasonal switches on
Appearance, defaults seasonal effects off, and requires about 2½ seconds of active
rubbing before the first petting reaction. Existing saved seasonal choices are retained.
See [ADR 0054](docs/adr/0054-settings-navigation-and-shared-artwork.md) and
[ADR 0055](docs/adr/0055-opt-in-seasons-and-intentional-petting.md).

Windows installers keep login startup off on a fresh installation and preserve your current
choice through updates and repairs, including when you turned it off in Goose after an
earlier installer opt-in. Explicit installer choices still apply. See
[ADR 0057](docs/adr/0057-current-login-startup-installer-selection.md).

Earlier releases, fixes and acceptance limits remain in the
[readiness records](docs/readiness/). Public download verification and hosted native
checks do not establish physical-device or administrator-prompt acceptance.

Actively rubbing the goose for about 2½ seconds earns its first petting reaction and hearts,
and can invite a gentle, 15-second follow at a comfortable distance. Quick swipes and
stationary hovering do not qualify. The **React to petting** preference controls this behavior;
following never grabs the pointer. Autumn leaves have recognizable shapes, scatter more
strongly when the goose runs through them, and fade away even without being touched.

## Install

### Windows

The recommended Windows install is the official versionless PowerShell bootstrap:

```powershell
irm https://github.com/RealEmmettS/goose/releases/latest/download/honk300-installer.ps1 | iex
```

It discovers the current stable release, downloads and verifies its exact Global-MSI and
lifecycle bytes, installs the managed PowerShell channel under Program Files, and verifies the
three public commands. It may request the normal Windows administrator approval; its app,
shortcut, and login-start paths use the windowless launcher and do not open a background console.

For native package deployment, use the Global MSI directly:

- [Global MSI for x64 (most Windows PCs)](https://github.com/RealEmmettS/goose/releases/latest/download/honk300-x86_64-pc-windows-msvc.msi)
- [Global MSI for ARM64 (Windows on ARM)](https://github.com/RealEmmettS/goose/releases/latest/download/honk300-aarch64-pc-windows-msvc.msi)

The MSI owns repair, upgrade, rollback, and uninstall. Corporate per-user packages, EXE
installers, portable archives, and checksums remain under each
[GitHub release](https://github.com/RealEmmettS/goose/releases/latest) for administrators and
compatibility.

Slot-aware releases keep each verified Windows payload in an immutable version/target directory
and move the stable `honk300`, `honk`, and `goose` command paths through a neutral `current`
selector. `goose update` reads the protected installation receipt and delegates to that exact
origin—Global/Corporate MSI, Global/Corporate EXE, or PowerShell—while the invoking old executable
remains untouched and alive. Unknown or conflicting ownership stops with an assisted reinstall
link instead of guessing Global MSI. A newly run installer, including an intentional downgrade,
is the user's latest intent and becomes authoritative only after staged verification.
If that change crosses from a machine-wide install to a per-user Corporate install, Windows may
require one administrator grant to retire the old machine owner. Cancelling that grant preserves
the new staged slot but reports `cleanup_pending`; Honk300 does not pretend the older machine PATH
has stopped winning. Running the new slot's `goose update` retries only the validated Honk300
registration. The same hidden elevated coordinator removes only that retired root's exact PATH
and login-start entries, verifies the active PATH, and clears the pending state without a second
elevation prompt.

Windows users on v1.0.2 or earlier must rerun the current installer once. Those older immutable
executables cannot self-repair their update-discovery bug; v1.0.3 and later perform subsequent
update checks correctly. Supported installer upgrades preserve settings and user content.

While Honk300 is running, an accessible **Honk300 controls** notification-area icon offers
**Configure Goose…**, which opens the matching graphical settings companion;
**Update Goose…**, which opens a new terminal and completes the verified update without
manual typing; and **Quit Goose**, which sends the goose through its normal full walk-off before
exit. Opening the menu performs no update check. The update terminal stays open with an explicit
success, no-op, or failure/recovery result until the user closes it.
The fixed product icon returns after Explorer recreates the taskbar. If an unusual interactive
session cannot host notification icons, Honk300 reports the limitation and keeps CLI/TUI/IPC
control available.

On Windows, typing `goose start` (or an explicit `start`/`plz` through a compatible alias)
uses the branded app launcher and returns the prompt only after the hidden runtime answers its
readiness check. The PowerShell or terminal window is then only a controller and may be closed;
the runtime and notification-area icon continue independently. A developer build must include
both `honk300.exe` and its exact sibling `honk300-app.exe`.

### macOS

The recommended Apple Silicon/Intel install is the official versionless terminal bootstrap:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/RealEmmettS/goose/releases/latest/download/honk300-installer.sh | sh
```

It installs the real signed universal `Honk300.app` without `sudo` at
`~/Applications/Honk300.app`, creates the three command aliases, and retains normal managed
Accessibility onboarding. For a graphical install, use the signed and notarized universal DMG:

- [Download the latest Honk300 DMG](https://github.com/RealEmmettS/goose/releases/latest/download/honk300-universal2.dmg)

The DMG remains the fresh graphical alternative. A receipted app at
`~/Applications/Honk300.app` updates synchronously with the exact-tag signed app ZIP while
preserving the same bundle and Accessibility identity. Mounted-DMG, foreign, read-only, and
unreceipted launches use the latest DMG as an assisted reinstall and do not claim update success.

Open the disk image and run **Install Honk300**. The universal Intel/Apple Silicon helper verifies
the adjacent app's Developer ID team and bundle identity, installs it without `sudo` into
`~/Applications/Honk300.app`, and opens the installed app. Receipts and user media live under
`~/Library/Application Support/honk300`; all three command aliases live under `~/.local/bin`.
Desktop pranks require Accessibility permission.

Launch the installed app by double-clicking `~/Applications/Honk300.app`, or keep that app in
the Dock as a launcher. While Honk300 is running, an accessible goose menu-bar icon offers
**Configure Goose…**, which opens the matching graphical settings companion; **Update Goose…**,
which opens the signed bundle's exact update command in Terminal; and **Quit Goose**, which sends
the goose walking fully offscreen before the app exits. The update helper relaunches the verified
installed app and leaves its final terminal result visible until the user closes it. The item
exists only while the Mac app is running. The goose runtime remains an agent app without
a running Dock control surface. Graphical settings open in their own companion process.

When the exact managed app starts without Accessibility permission, it records a secure
per-update prompt marker before asking macOS for consent and opening Privacy & Security >
Accessibility. The goose walks to a calm lower-right screen-edge perch while permission is
denied. Status, reload, honk, and stop continue to work; other direct actions report busy. A
second denied launch of the same update waits without reopening Settings. If permission is
granted or revoked while Honk300 is running, the same process notices within about a second and
either starts the normal introduction or returns to the safe wait. Development binaries, bare
copies, source-tree bundles, and an app launched directly from a mounted DMG do not open
permission UI automatically.

The app and graphical helper use hardened Developer ID signatures. The app sealed inside the ZIP
and the DMG are Apple-notarized and stapled, and release checks publish SHA-256 values for every
artifact. Every general release uses GitHub's macOS runners to produce a fresh signed/notarized
DMG even when the release was initiated from Windows or Linux. The stable link above advances to
the newest complete release; each tag and its DMG bytes remain immutable.

### Linux

Linux includes Honk300-owned notes and pictures, [experimental Raspberry Pi 4/5
desktop guidance](docs/raspberry-pi.md), and native ARM64 labwc evidence. Physical
Pi performance and complete desktop acceptance remain open; see the
[published Linux release evidence](docs/readiness/v1.5.0-readiness.md).

The recommended Linux install is the no-sudo official versionless bootstrap:

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/RealEmmettS/goose/releases/latest/download/honk300-installer.sh | sh
```

The bootstrap is stamped for one exact release. It detects the architecture and libc, downloads
that exact-tag payload, verifies its embedded SHA-256, stages on the destination filesystem, and
rolls back the payload and owned integrations if installation fails. It does not use `sudo`.

On Debian or Ubuntu, the architecture-matched native package remains an alternative:

- [Debian/Ubuntu x64 package](https://github.com/RealEmmettS/goose/releases/latest/download/honk300-amd64.deb)
- [Debian/Ubuntu ARM64 package](https://github.com/RealEmmettS/goose/releases/latest/download/honk300-arm64.deb)

The package installs machine-wide under `/usr/lib/honk300`, provides all three commands in
`/usr/bin`, and can request `sudo` or a graphical administrator prompt for install, update, and
removal. Configuration and personal memes/notes remain in the current user's XDG directories.

Shell-managed Linux installs use immutable release directories and an atomic `current` symlink.
Debian installs remain dpkg-owned and update only with the matching architecture package; the two
origins never convert into one another during `update`. Because changing between a per-user shell
install and a machine-owned Debian package crosses an authority boundary, a fresh installer stops
before mutation and gives the exact uninstall-and-retry command instead of leaving two competing
owners.

- Linux: managed payload under `${XDG_DATA_HOME:-~/.local/share}/honk300/install`; receipt,
  desktop entry, and user media stay in the corresponding XDG user directories; aliases under
  `~/.local/bin`.

Each general release contains both Debian packages plus x64/ARM64 GNU and musl archives. Stable
`latest` links point to the newest complete release, while the updater resolves the manifest's
exact immutable tag and verifies the selected platform artifact's kind, target, size, and SHA-256
before changing an owned installation.

While Honk300 is running, desktops with a StatusNotifier watcher and host show the shared
**Honk300 controls** item with Configure, Update, and graceful-Quit actions. Configure opens
the matching graphical settings companion. Update prefers `xdg-terminal-exec` and then known
terminal interfaces, passing the exact executable and command as literal arguments without
shell interpolation. It restarts the receipt-owned app after activation and retains its result until the
window is closed. The icon is embedded in portable binaries, and Debian packages also own its
hicolor-theme copy. Sessions without a compatible host or session bus log the explicit non-fatal
reason while overlays, CLI/TUI/IPC, and supported mischief continue independently.

The commands above are product-owned managed installers. Raw `cargo install`, a source-tree
binary, or a portable copy is an advanced unmanaged path: it cannot retire an MSI/EXE/DMG/DEB
owner and is not treated as an installation channel by `goose update`. Honk300 is intentionally
not published to crates.io.

## Use

```text
goose                        Open the graphical application
goose start                  Start the goose (returns after readiness on Windows)
goose status                Show runtime and platform capabilities
goose settings              Open the graphical settings companion
goose config                Open the terminal settings editor
goose reload                Apply reloadable saved settings
goose do honk               Request an action
goose stop                  Stop the running goose
goose stop --force          Stop immediately without the walk-off
```

Friendly aliases include `goose plz`, `honk bad`, `goose no honk`, `goose quit`, and
`goose do mud`. Run `honk300 help` for the complete grammar.

Honk300 enters by walking in from a real exposed screen edge and leaves the same way on a normal
stop. During ordinary roaming, touching monitors remain one continuous desktop; occasionally the
goose can walk completely beyond an exposed edge and, only while fully hidden, return from the
opposite exposed edge. Its deliberate puddle and prank errands always return through their own
departure edge instead. If you personally close a note or meme it opened, there is a roughly 30%
chance it gets visibly annoyed and then tries its existing bounded mouse-steal prank. That second
step still obeys your settings, quiet/fullscreen manners, live permission, and platform support;
program cleanup never triggers it. Packaged Linux notes and pictures use the same close-reaction
checks; their placement depends on the active backend and desktop integration.

Collected notes and pictures are fitted to the monitor receiving the goose. Notes remain readable
without exceeding 48% of either monitor dimension. Pictures preserve their complete source and
aspect ratio, downscale only when needed, and are never cropped or enlarged beyond their natural
size. Windows notes use a Honk300-owned native editable window rather than Notepad, so the prank
cannot restore, focus, type into, or expose a user's editor tabs.

First run materializes schema-current configuration. Existing malformed files and files from a
newer schema are never replaced automatically. To intentionally reset one, use
`goose setup --reset`; the previous bytes are backed up first.

Default config locations:

- Windows: `%LOCALAPPDATA%\honk300\config.toml`
- macOS: `~/Library/Application Support/honk300/config.toml`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/honk300/config.toml`

Settings that affect backend selection, including native Wayland mode, require a restart and are
reported as such. X11/XWayland remains the default Linux path. Native Wayland is an
explicit reduced mode (`goose start --wayland`); optional desktop integrations add only
their separately supported observations and actions. KDE window support and temporary
portal pointer permission are distinct grants. Sway and Hyprland provide observations;
GNOME's explicit companion uses the compatible XWayland overlay. Follow the
[desktop setup guide](docs/wayland-integrations.md) for exact versions and limitations.
Terminal windows are always protected from focus, typing, dragging, riding, and
collection. Codex and Visual Studio Code surfaces receive the same conservative protection across
platforms, including the ChatGPT-titled Codex desktop surface on Windows.

The General settings page also exposes default-off **Start on login**, stored as
`[lifecycle].autostart_on_login`. It reconciles only the startup integration owned by the active
installer: the matching Windows machine/user Run value, the managed Mac LaunchAgent, or the
current Linux user's XDG autostart entry. A fresh installer's selection is latest intent; later
config saves update that same integration. Unknown ownership and foreign startup entries are
refused instead of duplicated. On Windows, shortcuts and login startup target the internal
GUI-subsystem `honk300-app.exe`. Typed starts remain ordinary console commands for diagnostics,
but they now hand their start options to that same exact sibling launcher, wait for runtime
readiness, and return; its console-free child owns the overlay, IPC, and tray after the terminal
closes. Other commands and the configuration TUI continue to use the user's intentional terminal.

User notes and PNG memes can be added without modifying the program:

- Windows: `%LOCALAPPDATA%\honk300\media\Notes` and `...\Memes`
- macOS: `~/Library/Application Support/honk300/media/Notes` and `.../Memes`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/honk300/media/Notes` and `.../Memes`

### Remote control (opt-in, Windows)

A goose can be controlled from another computer through a relay you host yourself, for example
on Coolify. It is off by default, and the goose only makes an outbound connection. While it is
connected, the tray shows it and offers *Disconnect remote control*. Setup, the control panel
and the protocol are described in [docs/remote-control.md](docs/remote-control.md).

## Remove or update

On every platform, all three command names share the provenance-preserving update command.
Windows still uses Add/Remove Programs for graphical removal; update delegates to the exact
installer family named by the active receipt:

```text
goose update
goose update --json
honk300 uninstall
honk300 uninstall --purge
```

A normal uninstall preserves user media. `--purge` backs up user media before removing user
state. Debian package installs update and uninstall through `dpkg` and may request administrator
approval; per-user macOS/Linux installs do not use `sudo`. Login autostart is opt-in and off by
default; use **Start on login** in `goose config` or set `autostart_on_login = true` under
`[lifecycle]`.
Human update progress is stderr-only. `--json` writes exactly one final stdout object, and exit
zero means the selected release, receipt, selector, and aliases were activated and verified.

## Platform support

| Platform | Architectures | Desktop path |
| --- | --- | --- |
| Windows | x64, ARM64 | PMv2 layered overlays, one per monitor; Configure/Update/Quit notification-area item |
| macOS 11+ | Intel, Apple Silicon | Universal LSUIElement app; capture-safe AppKit RGBA overlays, one per display; Configure/Update/Quit menu-bar item |
| Linux X11/XWayland | x64/ARM64, GNU/musl | Full overlay and supported mischief; Configure/Update/Quit when a StatusNotifier host exists |
| Linux native Wayland | x64/ARM64, GNU/musl | Opt-in reduced overlay mode; independent StatusNotifier control when hosted |

The packaged Linux companion supplies owned notes and pictures on X11 and native Wayland.
X11/XWayland positions only Honk300-owned props. Native Wayland normally leaves placement
to the compositor; explicitly enabled KDE support adds bounded owned-prop positioning.
A standalone Rust binary retains CLI/TUI controls and reports props unavailable when its
companion is absent. Fullscreen and do-not-disturb observations are reported independently;
Mac and Linux do-not-disturb observation remains unsupported.

Exact signed-app macOS Accessibility evidence for first denial, non-nagging relaunch, live grant, and live revocation
remains tracked in the readiness docs; hosted bundle and permission-adapter tests are release
gates. Native release gates hold exact Windows/Linux binaries unchanged while checking
body/wing/beak/legs, asymmetric color channels, and transparency after each platform's real
presentation bridge rather than only in renderer goldens. Windows x64 and any host whose capture
API exposes ordinary windows must pass paired composed-desktop captures. GitHub's hosted ARM64
runner may use the narrowly identified ADR 0026 path: a real visible layered HWND plus the exact
cropped premultiplied-BGRA DIB recorded only after a successful native present. That runner-limited
evidence is not described as an ARM64 DWM screenshot; local and self-hosted ARM64 remain strict.

On macOS, tiny-skia's premultiplied RGBA bytes are copied into an alpha-last Device-RGB bitmap;
the transparent overlay window uses a stable standard-sRGB destination, and WindowServer performs
the final conversion for each physical display. This keeps screen capture, wide-gamut monitors,
and the on-screen palette on one explicit, regression-tested path.

## Build and verify

This is a Rust 1.95 workspace (edition 2021):

```text
cargo fmt --all -- --check
cargo clippy --locked --all-targets --workspace -- -D warnings
cargo test --locked --workspace
cargo build --locked --release
python -m unittest discover -s script/tests
dist plan --tag=v1.11.3
cargo audit --version 0.22.2
```

Build and qualify the separate GUI with `npm test`, `npm run check` and
`npm run build` in `settings/`; see [its development guide](settings/README.md).

The [architecture decision index](docs/adr/README.md) records the renderer, Native SDK
accessibility bridge, platform integrations, permissions and installation ownership
contracts. Later accepted decisions identify exactly which earlier decisions they replace.
The [release readiness records](docs/readiness/) preserve every candidate, publication,
fresh-download and installed-acceptance result, including unsuccessful qualifications.
Current implementation work is tracked on [the task board](.tasks/TASKS.md).

## License and assets

The code is licensed under the [PolyForm Noncommercial License 1.0.0](LICENSE). Bundled media has
separate provenance and redistribution terms in [THIRD_PARTY_ASSETS.md](THIRD_PARTY_ASSETS.md).
This repository does not contain the original developers' source/reference tree, donation pages,
or old developer branding.
