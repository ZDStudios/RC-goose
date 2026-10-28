# AGENTS.md

## Project and current work

Honk300 is a cross-platform Rust desktop goose with a procedural renderer and Windows,
macOS, X11, and opt-in reduced Wayland runtimes. **Goose** is the user-facing application name and `goose` is the recommended command.
`honk300` remains the internal primary binary; `honk` and `goose` remain compatible aliases. The Cargo workspace uses Rust 1.95, edition 2021.

Public stable is v1.11.3, source `c36d8e92847045d8b931618e014ae46cbd5228ab`.
Its publication and qualification record is `docs/readiness/v1.11.3-readiness.md`.
All required publication and fresh-public checks pass; all 47 public assets are independently verified.
Task #g11 and ADR 0052 track the combined application, icon, leaf and affection update.
The user confirmed that the installed v1.11.0 Windows application displays its logo
and opens the graphical controls; preserve that historical acceptance record.
Task #g12 and ADR 0054 track the published settings-navigation and shared tray-artwork update.
ADR 0055 adds default-off seasonal effects, Appearance leaf switches and intentional
petting: 2.5 seconds of active rubbing before the first petting reaction. Preserve saved
seasonal choices. A user-reported intermittent Windows black rectangle remains under
investigation; a clean rerun is not a demonstrated renderer fix.
ADR 0056 and task #g13 retain the original native performance budgets and unfinished
extended qualification. On 2026-10-19 the operator explicitly authorized publication
without waiting for repeated comparisons and packaged endurance; do not claim they passed.
Preserve exact viewport sizes while reusing bounded Windows bitmap capacity. Task #g14
retains the separate, unresolved transparency investigation.

Task #g15 and ADR 0057 correct Windows installers restoring an obsolete login-startup
selection after an app opt-out. Fresh startup stays default-off; migration uses the current
owned preference and preserves explicit choices. The v1.11.2 publication, exact installed
payloads, startup-off preservation, scrolling and runtime lifecycle passed; its failed direct
settings-controller checks remain recorded in `docs/readiness/v1.11.2-readiness.md`.
Task #g16 and ADR 0058 replace input-idle recognition with a bounded response from the exact
child window, retaining verified file leases, kill-and-wait cleanup and normal DLL imports.
The approved official v1.11.3 update, all six public-MSI payload comparisons, all four real installed settings routes, fifty native scroll traces and runtime lifecycle checks pass. Independent final readback preserves the exact original configuration, startup off in the receipt and both Run hives, and the stopped state.
Tasks #g16, #g15 and #g12 are complete with their required evidence retained; #g13 remains deferred and #g14 remains unresolved.
The original input-idle scheduling cause and first candidate UI Automation failure remain
unknown. Preserve their failed evidence and unchanged successful retries. Existing release
tags and assets remain unchanged.

The approved refinement is published through `.tasks/milestones/refine.md` and ADR 0041:
the redesigned continuously projected goose, reliability cleanup, Native SDK settings
with retained TUI, Linux owned props, experimental Pi guidance, and separately qualified
KDE/portal, Sway, Hyprland and GNOME integrations. Independent fullscreen/DND status and
the Mac fullscreen observer complete the final stage; Mac/Linux DND remains unsupported.

The website retains the user's restored r6 artwork; application renderer exports remain
available separately. Preserve that explicit website choice during release handoffs.
Native GNU/musl and ARM64 labwc proof is not physical Pi acceptance. Hosted Mac checks do
not complete physical permission or screen-reader user acceptance. Development exports, local checks and public
distribution evidence retain their separate boundaries in `docs/refinement-audit.md` and
the readiness records.

## Source of truth

- `honk300_plan.md` is the canonical original plan; later accepted ADRs supersede only the
  decisions they identify. `claude_plan.md` and `codex_plan.md` are superseded research drafts.
- `docs/adr/` records architectural decisions. Add a numbered ADR for changed platform, renderer,
  engine/backend, packaging, permission, or milestone decisions; preserve historical records.
- `crates/honk-engine/src/` and its tests pin the engine constants, rig geometry, task ordering,
  fixed 120 Hz simulation, planted contacts, and bounds. The engine forbids unsafe code and OS deps.
- `src/runtime/` shares ordering and bounded rendering across platform adapters. Use one overlay
  per monitor, signed desktop coordinates, on-dirty/rate-capped presentation, and bounded buffers.
- Shared crates own configuration, TUI, and IPC. `settings/` is the primary graphical control surface, implemented as a separate pinned Native SDK
  Zig companion using the bounded versioned Rust settings service. Application-menu Goose, bare `goose`, `goose settings`, and tray Configure launch the GUI;
  `goose config` retains the secondary terminal interface. Explicit start and login startup retain
  the existing runtime route. ADR 0052 changes entry routing, not shared ownership.
- `docs/history/` preserves the pre-refinement instructions and their detailed release chronology.
  Historical commands and stage descriptions are evidence, not overriding current instructions.
- Sibling `qube-machine-report`, `qube-network-diagnostics`, and `qube-workbranch-view` repositories
  provide family conventions for install/update, packaging, and paired changelogs.

## Locked contracts

- Name `honk300` (binary `honk300`, optional `honk` alias); fresh permanent WiX/Inno GUIDs.
- Procedural/clean-room goose. Sounds bundled 1:1 (personal use). M9 bundles screened original
  meme/note assets 1:1 for personal-use builds **plus one complete custom in-house counterpart
  per original** in the clumsy MS Paint house style. User-supplied `Meme8.png` is approved.
  Old developer donation pages, Patreon links, social handles, and old-project branding do not
  ship.
- Linux: **X11-first** (runs under XWayland); native Wayland behind an opt-in `--wayland`
  flag (reduced mischief).
- Packaging: recommend the stable versionless official bootstrap on every platform: PowerShell
  `irm ... | iex` on Windows and the no-sudo `curl ... | sh` bootstrap on macOS/Linux. The Mac
  bootstrap installs the real signed universal app in `~/Applications`; the graphical DMG,
  x64/ARM64 Global/Corporate MSI/EXE packages, and architecture-matched Debian packages remain
  supported native alternatives. Raw Cargo/source/portable installs are unmanaged and never
  retire another installer owner. **No crates.io.**
- Every general stable tag builds the complete platform set in GitHub Actions, including a fresh
  GitHub-macOS-produced signed/notarized/stapled app and DMG plus both Debian packages regardless
  of the operator's trigger host. Stable unversioned `latest/download` names advance only after
  atomic publication; existing tagged assets never change. The updater discovers through the
  latest manifest but downloads exact-tag bytes and requires platform, architecture, install
  provenance, artifact kind, size, and SHA-256 to agree before mutation. The DMG is a graphical
  install artifact; managed Mac CLI updates consume the exact-tag universal app ZIP through the
  pinned bootstrap.
- Release-mode macOS artifacts must fail closed without Developer ID and App Store Connect API
  credentials. No ad-hoc release fallback, `codesign --deep` signing, or DMG `/Applications`
  symlink is permitted.
- Automatic macOS Accessibility UI is limited to the exact DMG- or shell-receipted app at
  `~/Applications/Honk300.app`. It prompts at most once per installed update, waits calmly at a
  safe screen edge while denied, and handles grants/revocations in the same process. Development,
  bare, source-tree, and mounted-DMG launches must not open permission UI automatically.
- macOS transparent presentation uses reusable premultiplied-RGBA AppKit image views in the
  ordinary window backing store. Keep canvas/bitmap capacity bounded after transiently large
  damage so screen capture stays alpha-correct and normal walking does not redraw stale space.
  The alpha-last bitmap is Device RGB, the overlay window has a stable standard-sRGB destination,
  and final display-profile composition belongs to WindowServer rather than a per-frame
  application-side Device-RGB-to-Display-P3 conversion.
- Startup and graceful shutdown are locomotion states: stage the goose fully beyond a real exposed
  edge, walk in, and keep ticking/presenting on stop until the full pose has walked out. Shared
  monitor seams are continuous; only genuinely exposed edges may use the occasional 20% hidden
  wrap, and deliberate puddle/prank errands never wrap.
- Ordinary `stop`/`quit`/`exit` and native Quit always use that graceful walk-off. The explicit
  `--force` variants on all three command names terminate immediately through separate IPC.
- Windows collected notes are Honk300-owned native edit windows, never Notepad or global input.
  Windows/macOS/Linux notes and images share the ADR 0032 monitor-relative fit: hard 48% per-dimension
  ceiling, aspect-preserving complete-image downscale, no crop, and no upscaling.
- A user—not program cleanup—closing a spawned note or meme gets an independent 30% annoyed
  reaction roll. The reaction may chain only the existing bounded cursor nab and only after live
  capability, permission/pointer, configuration, and manners checks. Linux props are owned
  by the verified companion: X11 positions only those windows. Native Wayland uses normal
  compositor placement unless explicitly enabled KDE owned-prop support is active. User
  closes and connection-owned cleanup remain distinct.
- Install/update/uninstall retain the real runtime singleton for the whole mutation. Unix signals
  roll back with explicit nonzero status. Windows payloads remain pinned from same-stream
  size/hash verification through execution; generated bootstrap delegation must reacquire without
  an ambient bypass, machine-wide paths are checked across sessions, reboot-deferred MSI results
  fail closed, and every pre-READY helper error kills and waits for the child.
- Update provenance is authoritative and never guessed from a Windows install path. A protected
  v2 receipt preserves installer family, edition, scope, stable track, target, owned root, active
  release, and exact artifact identity. Windows and shell-managed Linux activate immutable slots
  through neutral selectors while the initiating process stays mapped to its old release; DMG,
  Debian, and package/bootstrap origins remain distinct. A fresh verified installer may
  intentionally downgrade and becomes the user's latest intent. Conflicting Windows registrations
  are retired only after commit through a protected journal; an opposite-scope cleanup that lacks
  an administrator grant stays nonzero `cleanup_pending` and must not claim public alias takeover.
  One hidden elevated active-slot coordinator runs the validated native uninstall and retires only
  that old root's exact PATH/Run entries before verifying the active PATH; it must not prompt twice.
- `[lifecycle].autostart_on_login` is default-off and reconciles only through the receipt-owned
  Windows Run value, managed Mac LaunchAgent, or per-user Linux XDG entry. Fresh installer intent
  outranks stale config; later explicit config edits update the same owned mechanism. Foreign or
  ambiguous startup ownership fails closed and must never create a duplicate persistence path.
- Windows keeps the three public aliases as console-subsystem commands for intentional CLI use.
  Every typed start spelling is a bounded controller that invokes the exact sibling independently
  hashed GUI-subsystem `honk300-app.exe`, forwards all start options, waits for IPC readiness, and
  returns. Start Menu/desktop shortcuts and login startup target the same branded app directly.
  The app starts only `honk300.exe __windows-app-runtime` with `CREATE_NO_WINDOW`, a new process
  group, null handles, and no shell intermediary. When its parent job permits breakaway, the app
  also uses `CREATE_BREAKAWAY_FROM_JOB` so integrated command runners do not retain the runtime;
  an access-denied job falls back to the same original windowless launch. That hidden process owns
  the runtime and tray independently of terminal lifetime. Background restarts/helpers must never
  create or focus a terminal; the explicit user-invoked control-surface Update is the sole visible
  helper exception. Product startup performs no screen calibration; full-desktop
  compositor surfaces are disposable-CI-only.
- Starting, stopping, configuration, and updating remain backed by the shared Rust services on every platform.
  The macOS menu-bar item, Windows notification-area item, and compatible Linux StatusNotifier
  item expose the same shared icon, accessible naming, native-settings Configure action, explicit
  terminal-backed Update action, and engine-owned graceful Quit. Opening a menu performs no
  network check. The GUI edits the existing versioned configuration through Rust; it has no separate updater or lifecycle owner. There is no global quit key; an unavailable shell
  host is non-fatal and does not change overlay or mischief capability claims.
- Remote control (ADR 0059) is opt-in through `[remote]`, default off, outbound-only WSS to a
  self-hosted relay (`relay/`). Relay commands become existing local IPC frames; remote mischief
  respects manners, `FORCE_STOP`/reload/update/config stay unreachable, and the client runs only
  while a tray indicator with a Disconnect item is present (Windows today).
- Terminal windows are protected: the goose may visually overlay them, but must never move,
  focus, type into, drag, ride, collect, or otherwise manipulate terminal windows, including in
  spicy/default-off modes. Conservatively treat Codex and Visual Studio Code surfaces as terminal
  windows across platforms, including the ChatGPT-titled Codex desktop surface observed on
  Windows.

## Implementation and qualification

- ADR 0041 replaces the old dual-view renderer with one continuous projected model. Preserve
  planted contacts, complete silhouette bounds, interaction anchors, transparent composition,
  and clear expressions. Review actual animation frames before blessing image goldens.
- Settings remains a separate executable in each immutable release, with exact identity in
  manifests/receipts, signing where required, and the same installation/removal ownership as Rust.
  Keep config comments, unknown keys, concurrent edits, validation, and restart distinctions.
- Typography is Makira Bold for headings and Makira Light for body text. Embed the provided faces.
  Respect native appearance, reduced motion, and high contrast. Verify native accessibility;
  internal automation trees alone do not prove screen-reader support.
- Keep archive traversal rejection, Windows verified file leases, rollback, provenance, update
  recovery, and retained helper ownership fail-closed. Never relax an oracle to accept broken art.
- Windows layered overlays require `UpdateLayeredWindow`, not softbuffer. Use per-pixel natural
  hit-testing; do not set `WS_EX_TRANSPARENT`. X11 input shaping follows the goose bounds.
- Windows IPC `PIPE_NOWAIT` zero-byte polls are transient until the bounded deadline, not EOF.
- macOS uses the stable signed app identity for Accessibility. AppKit work stays on the main
  thread; denied/live grant/live revocation and managed prompting retain their existing contract.
- A normal Wayland client cannot portably move other windows or warp/inject the global pointer.
  Report unsupported capabilities honestly; integrations are explicit and opt-in.
- Local full-desktop color calibration is prohibited. Full compositor tests are disposable-CI-only.
  Keep hosted screenshot/DIB proof distinct from physical hardware, permissions, and UAC acceptance.
- All eight architecture/platform lanes, signed/notarized/stapled Mac production, native Windows
  installers and Debian packages, same-source CI/candidate/main gates, atomic immutable publication,
  and fresh-public-byte verification remain release gates. Preserve every existing tag and asset.

## Task board

`.tasks/TASKS.md` is the board source of truth. Each task has `.tasks/tasks/<id>.md`;
keep `## Status` and `## Activity` current during work. Required small steps are indented
checkbox subtasks on the board, with optional indented descriptions. Give larger work its own
linked task using `(needs #id)`. Do not bury dashboard-trackable steps in prose.

Use the SHAUGHV tasks skills: `tasks-start` for initialization/repair/resume, `tasks-create`
for well-formed tasks and subtasks, `tasks-management` for format/completion, `tasks-update`
for reconciliation, `tasks-memory` for board memory, and `tasks-remove` for removal. If an
installed skill is missing or stale, prefer the harness-native update, then the relevant
current source under `RealEmmettS/shaughv-tasks/skills/`.

## Checks

- `cargo fmt --all -- --check`
- `cargo clippy --locked --all-targets --workspace -- -D warnings`
- `cargo test --locked --workspace`
- `cargo build --locked --release`
- `python -m unittest discover -s script/tests`
- Settings: `npm test`, `npm run check`, and `npm run build` in `settings/`; Zig lazy analysis
  means tests do not replace a real build. Native UI qualification uses an isolated config.
- Windows welcomed lifecycle-only checks: `pwsh -File script/smoke_windows_overlay.ps1
  -Binary target/release/honk300.exe -EvidenceDirectory target/windows-overlay-evidence -LifecycleOnly`.
- Linux host checks: `HONK300_BIN="$PWD/target/release/honk300" bash script/smoke_m17_m18_linux.sh`.
- Packaging uses cargo-dist plus project-owned atomic release, installer, and app workflows.
  There is no crates.io publication. Use the intended new version when planning release artifacts.

## Media and changelogs

`Assets/` is the approved built-in catalog. `THIRD_PARTY_ASSETS.md` identifies personal-use
compatibility media: do not claim redistribution rights or publish those assets separately.
Mutable user media lives in platform user-data directories, never sealed app/package locations.
The goose is clean-room procedural; old developer branding, donation pages, and social links
must not ship.

Update `CHANGELOG.md` and `HUMAN_CHANGELOG.md` together in the same commit. Every technical
entry needs a plain-English counterpart explaining what changed and why it helps, with no
version numbers, file/symbol references, metrics, PR/issue numbers, or jargon. Use Added,
Improved, Fixed, Removed, Security, or Behind the scenes. Internal changes still get an entry.
Keep current guidance in README, AGENTS, CLAUDE, ADRs, and the task board consistent.
