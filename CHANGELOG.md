# Changelog

All notable changes are recorded here using [Keep a Changelog](https://keepachangelog.com/)
and [Semantic Versioning](https://semver.org/). Plain-English explanations are in
[the human changelog](HUMAN_CHANGELOG.md). See [the README](README.md) for current
features and installation, and [readiness records](docs/readiness/) for release evidence.

## [Unreleased]

### Added
- Add opt-in remote control (ADR 0059): a `[remote]` config section (default off; `enabled`, `relay_url`, `token`, `device_name`), a Windows runtime client that keeps one outbound rustls WebSocket to a self-hosted relay with bounded reconnect backoff, and a Node relay plus single-page control panel under `relay/` with a Dockerfile for Coolify. Relay commands are translated into local IPC exactly like the CLI.
- Add backward-compatible `HONK300/1` frames `RDO`, `WALK_TO`, `LEAF_PLACE`, `LEAF_MOVE`, `SNAPSHOT` and `LEAVES`, the engine `WalkToTask`, and remotely placed leaf piles that last 120 s, survive while the Autumn season is off, can be moved until kicked, and share the six-pile cap.

### Security
- Gate remote mischief on schedule/presence manners (`QUIET`), keep `FORCE_STOP`/reload/update/config unreachable from the relay, require `wss://` except for loopback, start remote control only when the Windows tray exists, and show its state in the tooltip and menu with a one-time connect notification and a Disconnect item. An invalid `[remote]` section disables only remote control.

## [1.11.3] - 2026-10-20

### Fixed
- Recognize Windows settings startup through a bounded response from the exact spawned window, retaining verified executable/DLL leases and the ten-second readiness deadline instead of rejecting a responsive window whose input queue does not become idle. Preserve kill-and-wait cleanup before releasing those leases.

### Behind the scenes
- Record complete v1.11.3 source/candidate/main/publication qualification, independent public-byte verification, approved installed settings/scroll/runtime acceptance and the website handoff. Preserve failed attempts and unchanged successful retries without assigning an unproven cause. Final receipt: `target/v1.11.3-delivery-evidence.json`.
- Require the Windows settings accessibility DLL as a normal PE import, validate bounded import descriptors and thunk tables, reject unsupported bound/delay imports, and exercise native readiness, process ownership, liveness and failure cleanup with real child-window fixtures.
- Record complete v1.11.2 qualification, independent public assets and website delivery, then the approved installed update with exact payload/configuration and startup-off preservation. Retain canceled/failed attempts and the separate direct-settings readiness failure alongside passed native scrolling and runtime lifecycle.

## [1.11.2] - 2026-10-19

### Fixed
- Initialize Windows MSI and EXE login startup from the current owned Run entry instead of stale installer feature/task history, preserving default-off installation, app opt-out, explicit installer choices and unrelated options. Capture the final MSI choice after feature validation so the receipt agrees with installation.

### Behind the scenes
- Add eight native Windows installer startup lanes covering fresh defaults, app/installer drift, explicit choices, repair and ownership refusal. Exercise MSI upgrade rollback after verified candidate activation and full wizard feature selection/repair, retaining original failures and exact configuration restoration.
- Initialize native MSI accessibility providers from a typed helper and verify wizard ownership, selection, focus and menu geometry. Snapshot native window identity as scalars, handle disappearing windows and retain disposable desktop evidence for owner-drawn menu items without accessible names; isolate expected-failure probes from modal dialogs, cached package substitution and retained COM handles.
- Align settings/package identity and record diagnostic harness, workflow and installed payload revisions separately. Reused candidate artifacts support diagnosis; release gates still rebuild and qualify one unchanged source.
- Record the complete v1.11.1 publication, all eight fresh-public lanes, independent 47-asset audit and exact website deployment. Preserve the canceled Windows administrator-prompt attempt and verified unchanged v1.11.0 state; leave installed-public acceptance, extended performance qualification and the original black-rectangle investigation open.

## [1.11.1] - 2026-10-19

### Improved
- Reuse bounded Windows DIB capacity and copy checked monitor rows directly from the shared render canvas, preserving exact window dimensions and premultiplied alpha while removing the per-frame crop allocation.
- Keep the current native settings tab visibly selected and bound wheel momentum to a short damped tail, with immediate reversal, clamped edges and direct steps under reduced motion.
- Put Seasonal effects and Autumn leaves on Appearance. Default the seasonal master switch off for new or missing settings, preserve existing explicit choices, and clear active leaves immediately when either switch is disabled.
- Require 2.5 seconds of active rubbing before the first petting animation, heart, pat sound, calming effect or invited follow. Reset unearned progress on pointer departure or a motion pause, and discard pre-threshold movement instead of bursting queued reactions.
- Load the Windows tray icon from the exact application resource at native small-icon size. Use the Linux application's artwork with correct StatusNotifier channel/alpha conversion, and package the Mac application's source for its menu-bar template before signing with exact-byte verification.

### Fixed
- Advance native settings wheel decay using measured frame timestamps, with bounded integration after delayed frames and a wheel-specific clock that unrelated pointer input cannot reset. Preserve nominal frame cadence for input-latency accounting.
- Publish updated native accessibility bounds after wheel input stops at a boundary or uses reduced motion; retain post-presentation batching and unchanged-layout deduplication.
- Expose the Wayland settings switch only on Linux builds, including defensive filtering of older service responses. Preserve its saved value when other preferences change on unsupported platforms.

### Behind the scenes
- Add opt-in bounded Windows frame profiling, isolated process/compositor sampling and deterministic leaf benchmarks. Record the operator's explicit deferral of repeated-performance and packaged-endurance qualification; keep the intermittent black-rectangle investigation open after its bounded unreproduced outcome.
- Record the user's successful installed Windows logo and graphical-controls acceptance, and qualify the settings and icon follow-up through the complete release gates.
- Record a complete Markdown export of the Windows installer EULA, verified against all four published MSI license controls and Windows native RTF decoding. Coordinate and verify the complete footer-only website page, identical downloadable Markdown and final website CI/deployment evidence.
- Record the complete v1.11.0 source, candidate, main, publication and fresh-public qualification, exact public asset verification and current installed-acceptance state. Update stable release guidance while preserving the first Linux display failure and its successful unchanged-source rerun.

## [1.11.0] - 2026-09-09

### Added
- After 2.5 seconds of actual rubbing, invite a 15-second cursor follow at configured walking speed with a 90-pixel gap, one invitation per streak, no cursor/window commands, and cancellation for pointer loss, clicks, petting opt-out, manners, permission withdrawal and shutdown.

### Improved
- Make the native Goose application the primary interface across platforms, including the Debian package description. Application-menu entries and bare commands open the existing Rust-owned graphical controls; explicit start/plz and autostart retain their runtime route, and the TUI remains available through goose config. Preserve all binary, installer, receipt, app-bundle and permission identities while supporting the new display names alongside historical installer names.
- Render distinct maple, oak, birch and ginkgo SVG designs in every existing autumn color. Keep source SVGs, sixteen recolors and Rust geometry synchronized, cache bounded supersampled sprites, and vary size, mirroring and continuous rotation independently. Preserve simulation RNG, leaf counts, kick strength, gravity and bounce; expire every pile after 30 seconds with a four-second fade, or sooner through its existing kicked lifetime.

### Fixed
- Recognize atomic GNOME consent replacement during stream open with one fully revalidated read inside the original deadline. Keep private ownership, size, companion identity and repeated-change rejection intact; complete revocation retires observations instead of leaving a misleading failed status. Deterministic production-code regressions reproduce the race and cover malformed, unsafe and stalled replacements.
- Match Linux running settings windows to the existing GTK application identity using a hidden desktop record and full-resolution theme icon, while retaining one visible launcher. Preserve icon/desktop ownership through shell rollback and uninstall; verify actual installed identity and artwork through native GIO/GTK on Debian and shell paths.
- Bind the Global MSI's machine-owned shortcut directory to Windows Installer's standard ProgramMenuFolder before costing, preserving its HKLM component and native ICE validation. The former undefined CommonProgramsFolder resolved to the drive root, leaving the Start menu empty despite successful installation.
- Prefer authoritative current-owned settings receipts over stale external installer records, matching updater provenance precedence. Keep malformed/conflicting owned evidence fail-closed and preserve companion size/hash verification and retained Windows file leases.
- Embed application icons in Windows command and GUI executables, bind the settings window icon, and supply explicit MSI/EXE installed-app and app-menu icons. Qualify actual installed GUI entry points against stale receipt records across installer origins and both native architectures. Include a sealed Mac app icon and localized Goose display name at the existing managed bundle path.
- Preserve MSI icon-table file extensions and verify native ProductIcon artwork instead of requiring an EXE-style registry value. Resolve environment-variable icon descriptors before native decoding and retain registered paths in qualification evidence. Decode packaged Mac icons with AppKit; use the full application icon for Debian and owned manual Linux application entries while retaining the login-startup entry's existing theme-icon choice.
- Report Windows tray Configure/Update launch failures visibly without blocking the runtime event loop.

### Behind the scenes
- Update native tray protocol expectations and accessible descriptions for the requested Goose display name, retaining actual action dispatch and watcher-recovery checks.
- Close the complete refinement milestone with final GNOME/presence source, native,
  publication, fresh-public and website evidence; reconcile adapter/gait/readiness/accessibility
  tasks and audit states, shorten current guidance, archive outdated changelog introductions and preserve historical failures
  plus all unresolved physical and official installed-acceptance records.

## [1.10.1] - 2026-09-08

### Behind the scenes
- Preserve the unpublished immutable v1.10.0 tag after its final GNOME qualification failed. Publish the complete external-revocation fixture record through a private flushed temporary file and atomic rename, matching the production writer. Production Shell regressions distinguish truncated/partial consent from complete revocation and require no observations or retained requests in all cases. Repeat all source, candidate, main, publication and fresh-download gates for the complete fullscreen manners stage.

## [1.10.0] - 2026-09-08 (unpublished)

Publication was blocked by the GNOME consent-fixture write race recorded in
`docs/readiness/v1.10.0-readiness.md`. The features below continue in v1.10.1.

### Added
- Add a retained, bounded macOS Accessibility fullscreen observer with live target identity, permission withdrawal and stale-sample expiry. Add independent fullscreen/DND capabilities through an additive bounded IPC frame, CLI, TUI and Native SDK settings; older runtimes remain unprobed.

### Fixed
- Include ADR 0051's shared native Mac dialog isolation, current-action validation and real assistive text editing. Require the actual production AX client, explicit focus transfer and independent status labels on both Mac architectures.
- Include the shared startup-readiness correction: retry only Status after a broken/reset connection inside the existing deadline, preserve permanent failures and child checks, and require three actual GUI start/stop cycles per native lifecycle lane with failure state retained before cleanup.
- Include ADR 0050's current-motion landing and stopping correction, preserving planted contacts and lift phase through reversal and acceleration. Carry four actual World delivery witnesses, bounded native foot/capture evidence and two visually reviewed turn-golden updates with unchanged comparison tolerances and gait bounds.
- Recover Windows notification-observation status after a successful poll follows an error, and expose the current Linux compositor fullscreen capability independently of unsupported DND observation.
- Report an expired Mac fullscreen sample as failed while its replacement is blocked; reserve unprobed for a new target, preserve the original freshness limit and recover on a fresh result.

### Behind the scenes
- Integrate the reproduced GNOME extension-disable status correction and repeat final-source native and distribution gates, preserving the qualified Mac observer and every held-ride cancellation assertion.
- Keep private GNOME drag input clear of the goose's return route, guarantee an actual initial pointer change and record the compositor's picked surface alongside native input acknowledgments.
- Use GTK's typed click signals for native input acknowledgment on the supported Python bindings, preserving unclaimed event propagation and actual window-handle movement.
- Acknowledge each real GTK motion and primary-button event before issuing the next private GNOME drag step; retain actual Mutter grabs, engine rides, terminal refusal and every held-action cancellation check.
- Integrate the native GNOME drag-handle correction and repeat final-source qualification after cancelling the incomplete candidates; preserve production presence behavior and ordered publication gates.
- Record all four corrected GNOME desktop passes and reviewed native output before freezing the combined final-stage candidate; preserve the qualified Mac implementation and every publication gate.
- Record repeated Intel/Apple Silicon fullscreen and denied-identity qualification after GNOME review integration; carry bounded native recovery and verified Hyprland publication into the final stage while preserving public release order.
- Require native Mac fullscreen/restore, denied app identity, live settings, unresponsive-target recovery and graceful shutdown, plus separate Windows UI Automation and Linux AT-SPI presence labels, as release qualification. Preserve the unqualified app-specific Focus experiment without production prompts or inferred support.
- Run strict native Mac workspace lint with correct platform bindings and Linux-only Debian uninstall helpers; retain explicit permission-loss and timeout boundaries.
- Synchronize private compositor startup, GTK input delivery and Mac fixture completion with actual native evidence, keeping transaction deadlines, peer checks, real grabs and observed fullscreen transitions mandatory.
- Reconcile published Linux and native accessibility guidance, document independent observation status, and require complete same-source platform, package and fresh-public verification for the remaining manners update.

## [1.9.0] - 2026-09-08

### Added
- Add explicit GNOME Shell companion setup and removal through Rust CLI and Native SDK settings, with qualified GNOME 46.0/48.7 window, fullscreen and actual user-drag observations on native x64/ARM64. Preserve the compatible XWayland overlay and independent owned-prop positioning; foreign movement, pointer access and DND remain unsupported.

### Fixed
- Route the modern Mac AXValue setter through actual runtime text input and
  separate snapshot value/focus publication from assistive input setters, after
  the native reader exposed an edit that never reached the settings draft. Keep
  the exact-hash patch reversible across repeated prepare/test/build invocations.
- Bound Mac settings accessibility to the active dialog and validate cached
  native actions against current published permissions under ADR 0051; retain
  both actual native reader checks in the production companion matrix.
- Retry only the startup Status probe after a broken/reset IPC connection within the existing readiness deadline. Preserve child-exit, permission and malformed-response failures; pin recovery/deadline behavior in the actual readiness loop, exercise three GUI start/stop cycles per native lifecycle lane, and retain failure status before cleanup.
- Keep airborne landing predictions current through braking, reversal and acceleration; apply the existing travel cap within an active swing while preserving planted contacts and lift progress. Pin actual native-height World deliveries and export their approach/settling sequences. Review and update only the two affected turn goldens, retaining all comparison tolerances and gait bounds. Retain bounded CI-only foot state and capture timing for native deliveries.
- Withdraw an airborne foot's old walking prediction when locomotion stops, preserving its current position, original step/lift phase and every planted contact. Add actual gait-phase and World delivery regressions plus abrupt-stop Rust preview sequences under ADR 0050.
- Keep GNOME extension loss distinct from durable consent revocation when an in-flight native query completes during disable; withdraw observations and report failed consistently until explicit reconnection.
- Check independent GNOME consent files concurrently within the original deadline, retaining both full consent checks, opened identities and joined cancellation cleanup.
- Recover bounded GNOME query timeouts through the retained authenticated connection after withdrawing stale data. Recheck live consent and the pinned Shell on every query; keep permission, owner, extension and decoding failures terminal.
- Include both embedded GNOME files in Cargo source selection, omit blind XWayland pointer input on GNOME Wayland and enforce the window bound only on reportable native actors.
- Consume authenticated consent revocation from the retained observation worker instead of reading user-data files on every runtime poll. Use bounded cancellable asynchronous GIO reads in Shell, retain opened-file identity checks and cancel pending requests on removal.
- Omit destroyed and zero-area Mutter actors during mapping transitions while retaining strict validation for every reported target. Cancel held rides on configuration changes, target destruction and extension loss.
- Gate GNOME settings actions on their own capability and keep wrapped content clear of the native scrollbar.

### Security
- Authenticate the pinned system Shell and approved runtime executable, bound complete connection and snapshot operations, preserve exact owned files and recoverable consent, and withdraw stale or revoked observations until explicit reconnection.

### Behind the scenes
- Qualify the existing Mac settings AppKit Accessibility provider through an
  independent native reader on Intel and Apple Silicon, using exact completed
  source-CI app artifacts and isolated configuration without the SDK harness.
  Record both native passes and require explicit focus transfer in final-source
  qualification, alongside the unchanged saved-value and modal assertions.
- Preserve the complete 6ff7611 candidate and failed main input evidence; repeat all release gates after a production status-race regression and native per-step drag acknowledgments away from the goose's return route.
- Verify newly captured GNOME handle motion against actual widget bounds and surface ownership instead of an enter-state property that may predate controller attachment.
- Start GNOME qualification gestures on GTK's native client-area WindowHandle after actual pointer entry; require the same Mutter grab, engine ride, terminal exclusion and held-action cancellation across remapping.
- Read actual manners from the bounded GNOME post-tick trace throughout qualification, and use explicit GTK fixture title bars plus real pointer motion across remapping while retaining mandatory native grabs.
- Prove GTK pointer entry inside the native client before a separately verified frame/title-bar drag, preserving real Mutter grabs and ride cancellation across remapped decorations.
- Close Hyprland publication, all eight fresh-public lanes and exact website download verification; update current guidance and dashboard status while retaining GNOME and independent presence qualification gates.
- Distinguish Shell request deadlines and fixed failure phases without forwarding private or arbitrary remote error content; retain the existing fail-closed observer behavior while diagnosing native qualification failures.
- Wait for the private Sway and Hyprland compositor to answer its initial read-only version request; preserve per-transaction bounds, peer checks and non-retried mutation commands.
- Keep native drag qualification clear of the goose's actual post-ride hit region while continuing to require input delivery to the exact GTK window and a real Mutter grab.
- Qualify real native title-bar gestures only after the authenticated private fixture is clear of Shell panels and GTK confirms pointer entry. Capture actual motion and failed desktop state while retaining mandatory compositor grabs, terminal refusal and production engine rides.
- Require the real GNOME runtime, native settings and owned-prop scenarios as same-source release gates, including unapproved callers, actual held input, terminal protection, saved drafts, native note text, uncropped images, helper failures and restart. Preserve private display authentication and omit private consent from evidence.
- Record separately qualified desktop and publication evidence, including the verified Sway website handoff with the user's selected artwork.

## [1.8.0] - 2026-09-08

### Added
- Add explicit Hyprland window/fullscreen observations through Rust CLI, versioned IPC and Native SDK consent dialogs. Keep separate private consent, immediate revocation, unsaved drafts and unsupported movement, user-drag, pointer and DND capabilities.

### Improved
- Authenticate the native compositor and read its coherent monitor/client inventory in one bounded read-only batch. Withdraw expired snapshots immediately; retry only timeouts against the same retained owner, keeping permission loss, identity changes and disconnects terminal.

### Fixed
- Preserve the exact running Hyprland worker and consent when repeated setup overlaps fullscreen recovery, independently from snapshot freshness.

### Behind the scenes
- Record Sway's unchanged-source candidate/main qualification, immutable publication and independent public artifact audit, and close the separate native Hyprland visual/lifecycle gate before its candidate.
- Require completed production worker joins before reporting revoked Hyprland access, then independently bound kernel task retirement in the native fixture; retain the exact thread and capability evidence for both boundaries.
- Qualify Hyprland 0.53.3/0.55.2 on native x64/ARM64 using an isolated system-identified virtual graphics device, responsive GTK fixtures and actual production transport. Exercise impostor refusal, engine fullscreen manners, config changes, native settings, drafts, revocation and graceful/crash recovery.
- Inspect native goose/settings and complete-image captures; qualify owned note/image limits, close handling and child failure recovery. Require painted note-body text as well as native accessibility text before recording delivery evidence.
- Bound cold PowerShell compositor-fixture startup separately from goose readiness, and retain startup-stage timestamps after native ARM hosts exceed the old five-second setup window. Preserve every DIB, alpha, channel, clipping and displayed-pixel assertion.

- Complete the KDE website handoff with deployed setup/capability readback, all 22 verified downloads and matching public manifest hashes, closing the third staged publication record.

### Behind the scenes
- Record the immutable KDE publication, unchanged-source candidate/main qualification, all eight fresh-public installation lanes and the independent audit of every public artifact.

### Behind the scenes
- Record v1.5.0 immutable publication, all-eight fresh-public qualification and independent verification of all 47 assets. Close both website handoffs against the actual renderer SVG deployment and verified Linux/Pi guidance with all 22 download mappings.

### Behind the scenes
- Reconcile v1.4.0 final-source publication and fresh-public evidence across current guidance, accessibility ADRs, the audit and completed foundation/settings/art tasks. Retain separate website/next-stage gates and track unresolved presence observation in its own milestone task.

## [1.7.0] - 2026-09-08

### Fixed
- Retain XRes-derived process identity in the native untitled-client fixture: both qualified XWayland generations report the real PID despite an absent `_NET_WM_PID`. Qualify the actual null title natively and keep nullable-PID behavior covered by its separate production decoder regression.
- Preserve healthy Sway observations during frame-publication contention. Accept legitimate null titles and retain visible fullscreen clients whose process identity is unavailable; regression tests reproduce both decoder failures, and native qualification includes an untitled XWayland client without an advertised PID.
- Replace the Windows smoke's obsolete full-canvas transparency assumption for bounded captures with exact DIB-to-DWM comparison of every pixel over both backgrounds and a complete clear perimeter. Preserve semantic, clipping, channel, premultiplication and black-surface checks; add regressions for bounded poses, opaque slabs, incorrect alpha and stale frames.
- Preserve a healthy Sway observation worker through repeated CLI or native settings setup. Verify the actual worker identity and unchanged consent while fullscreen manners remain enabled.
- Inherit Sway fullscreen state through real split containers while excluding the synthetic workspace flag. Reject observations without an active powered output. Production decoder regressions fail before both corrections and pass afterward; native fixtures require both compositor states. Enable the private fixture's real accessibility bus before querying its native settings provider.

### Behind the scenes
- Exercise the production socket exchange against partial, oversized, mismatched and disconnected replies, retaining one deadline across all fragments and rejecting a replaced socket after readback.

### Added
- Connect explicit Sway CLI and Native SDK setup/removal to a private consent record and the Rust observation worker. Keep request polling off the presentation loop, expire stale frames, retain the exact worker through removal, and require explicit reconnection after socket replacement. Qualify actual fullscreen manners, live configuration changes, native accessibility actions, drafts and restart in disposable Sway desktops.
- Implement the read-only Sway window/fullscreen decoder and bounded native transport for independently qualified 1.9/1.10.1 desktops, requiring private socket ownership and a system-owned compositor peer. ADR 0046 leaves movement and pointer actions unsupported without authoritative user-drag/input evidence.

### Behind the scenes
- Qualify the Sway 1.9/1.10.1 native premise on x64/ARM64 Ubuntu and Debian, checking real socket peer identity, window geometry, bounded owned-fixture motion and fullscreen state before runtime implementation. Retain the unresolved active-drag observation boundary before enabling production movement.

## [1.6.0] - 2026-09-08

### Added
- Add explicit KDE setup/removal in CLI and Native SDK settings, backed by a private consent record and sealed, same-user KWin companion. Report window observation, bounded movement, owned-prop positioning, pointer observation, fullscreen and DND separately.
- Connect ephemeral KDE 6 portal/libei pointer request/cancel controls to the running Rust owner. Require a fresh terminal-safe path for every bounded engine cursor command; preserve independent GNU/musl and KDE generation capabilities.
- Export editable SVG headings, poses and motion frames from the same statically dispatched vector drawing routine as the desktop, with unchanged raster output (ADR 0045).
- Document guided setup, removal, optional system dependencies, session-only pointer permission and independently qualified compositor boundaries.

### Fixed
- Preserve the current KWin bridge and active portal grant when CLI or native settings repeats setup for the same healthy consent. Invalid, changed or stale authority still revokes before reconnection. Include the embedded companion in Cargo's actual source-package file selection.
- Preserve exact floating-point geometry at the KWin JSON boundary. A captured native coordinate regression reproduces a one-ULP decoding change that made the script correctly reject every move after the first fractional position; float_roundtrip fixes the data without relaxing the stale-window guard.
- Scroll native accessibility targets into view through existing bounded scroll containers before activation. The real SDK dispatcher regression reproduces the original failure and preserves disabled-control refusal.
- Recognize only the current process's click-through Wayland layer when guarding pointer motion, retaining terminal and unknown-window checks beneath it. Retain KWin observation timers at script scope and qualify their continued native updates beyond garbage collection.
- Clear queued engine cursor movement on capability loss; cancel portal sessions on adapter expiry, device pause/removal, explicit revocation and shutdown. A restart never restores a pointer grant.
- Apply owned Wayland prop identities after GTK maps the native surface, and match geometry to an unreaped child and per-window token before positioning. Revocation cancels motion and preserves delivered notes.
- Ignore unchanged Wayland layer configuration and effective-scale notifications when reporting topology changes; actual output or scale changes retain immediate action cancellation and a diagnostic.

### Behind the scenes
- Require exact-commit KDE 5/6 x64/ARM64 qualification in candidate and public release orchestration, alongside all existing architecture, native package and signing gates.
- Record the complete native pointer lifecycle results and keep final-source package qualification open; clarify the existing note-retention and active-picture cleanup behavior on integration removal.
- Verify KDE's native device-removal outcome when a portal backend exits during active cursor motion, including stationary readback and rejection of subsequent pranks; preserve intermediate lifecycle evidence on later failures.
- Run native KDE PR qualification without a duplicate development-branch run; exact-commit main, candidate and public gates remain required.
- Capture bounded collection task, pose and geometry diagnostics only in explicitly enabled CI fixtures to distinguish engine release from native movement refusal, without logging window titles or user content.
- Require four independent runtime delivery, revocation and restart repetitions on every native KDE lane, retaining each attempt's bounded observations and state diagnostics.
- Scope the approved Sway, Hyprland and GNOME follow-ons as separate dashboard tasks with native premise, ownership, version and publication gates; no additional compositor support is claimed by this release.
- Qualify real consent, bounded motion, protected targets, native settings drafts and recovery in disposable desktops with normal Plasma service registration and PipeWire. Retain bounded failure/observer evidence without granting container or compositor privileges.
- Await the completed native settings save response before Windows test readback, so the fixture does not deny atomic replacement.
- Wait for the Linux restart probe's complete output line before checking its PID, session identity and literal start argument. Native ARM64 main qualification exposed the file-creation/readback race; the production launch path and exact assertions are unchanged.

## [1.5.0] - 2026-09-08

### Behind the scenes
- Correct the owned-prop launch's module path after native Linux compilation, before qualification of the updated packages.
- Qualify the exact musl archive on native-architecture Alpine with its required GTK libraries and the unchanged compositor/prop readiness checks. Bind the Linux prop launch to the retained verified file descriptor and preserve invalid-receipt failures.

### Added
- Add Honk300-owned Linux note/image deliveries through the existing verified GTK companion, with X11 animated positioning and native Wayland compositor placement. Expose live session and separate positioning capability in CLI/TUI/GUI, and document experimental Pi 4/5 64-bit Desktop support with native ARM64 labwc evidence.

### Fixed
- Keep exact musl archive qualification unprivileged by removing only the private Alpine compositor's realtime file-capability request and preserving its metadata before execution.
- Honor the documented native Wayland opt-in on desktops without X11: a bare launch explains the existing command/config choice before creating an overlay or companion.
- Give the native Linux prop close button the visible label Close; GTK's deferred child label had replaced the separate accessibility label with the multiplication symbol.
- Bound retained owned props to eight windows across supported native controllers. Defer new delivery at capacity, preserve existing notes, and resume after a close; finish an already admitted delivery normally.

### Development
- Prepare the ordered Linux release candidate and reconcile current capability guidance with successful production GNU/musl and ARM64 labwc evidence; retain every final-source package/publication gate and the physical Pi acceptance boundary.
- Use Alpine's available picom XRender compositor for private musl prop qualification, retaining the live compositor-selection and visible-note checks; xcompmgr remains the GNU baseline host.
- Exercise the exact production musl companions with real X11 notes, complete-image pixels, user closes, runtime delivery and child-loss cleanup on native x64/ARM64 Alpine; share the isolated compositor runner with the GNU baseline gate.
- Document experimental Pi 4/5 64-bit Raspberry Pi OS Desktop setup, actual labwc 0.7.1 x64/ARM64 qualification and the remaining physical-device boundary; index the refinement ADRs.
- Exercise actual production GNU companions on the Ubuntu 22.04 GTK baseline after identity verification. Record successful native x64/ARM64 X11/labwc delivery, scale, close and failure evidence; require light note-body pixels so a gray test desktop cannot satisfy visibility.
- Make the shared Linux release smoke expect a ready prop companion when the actual runtime payload includes one, and explicit unsupported capability for a bare Rust binary; failed companions cannot satisfy either case.
- Use client-side X11 composition in the native-delivery fixture and require actual note pixels plus owned geometry, not only AT-SPI text. Read labwc's real private output using the installed wlr-randr text interface instead of its unavailable JSON option.
- Refresh the versioned task dashboard and separate independent Linux implementation from ordered publication gates; retain existing board settings and stopped server state.
- Report the live Linux display backend, bounded desktop-session hint and separate owned-prop positioning capability through an additive SESSION request, CLI/TUI status and graphical settings. Preserve legacy STATUS frames and never enable privileges from environment hints.
- Give the real delivery fixture a valid positive initial-wander duration and report early runtime exits directly; retained logs exposed configuration rejection before overlay initialization.
- Start and verify the disposable X11 compositor before real prop delivery; GTK windows alone do not satisfy the production overlay's transparent-composition contract.
- Read the CLI's reported running state in the native delivery fixture; a successful status command also reports that no runtime is active.
- Exercise malformed/oversized private prop input and standalone runtime/settings-service operation without the companion. Document the actual bounded ownership protocol and preserve fixture pipe cleanup after failure.
- Add native labwc and scaled-display probes using normal Wayland placement, complete-image pixels and real engine delivery, with no physical Pi acceptance claim. The first strict Linux build found a stale installer import; limit it to its actual Windows/test callers.
- Connect the real Linux runtime to a receipt-verified private prop child with bounded nonblocking I/O, readiness/admission, pending identities, image/text limits, failure latching and retained cleanup. Keep native Wayland geometry logical and disable animated placement there. Add native engine-delivery and child-loss probes plus actual four-corner image pixel checks.
- Retain Cairo prop pixels through the surface's final reference, add native header dragging, and capture the GTK action tree when user-close qualification cannot activate Close.
- Exercise a prop's native Close button through AT-SPI; direct X window destruction bypasses GTK's user-close path. Preserve unexpected protocol output as failure evidence.
- Embed the approved Makira faces in native Linux prop text/headers through private anonymous font files; cap command dispatch per GTK iteration while preserving bounded input storage.
- Call the explicit AT-SPI Text interface in the native GTK prop fixture so Ubuntu's similarly named deprecated Accessible method cannot shadow text readback.
- Run the dedicated native prop probe when its isolated implementation branch changes; this also initializes the new workflow before it exists on main.
- Build a bounded private Linux prop mode into the existing GTK settings companion. Preserve Unicode note text, premultiplied full-image rendering, opaque identity, close origin, connection-owned cleanup and a native capacity/geometry fixture; runtime integration and Wayland qualification remain pending.
- Add placement-only collect behavior for a native host that can create its own props but cannot position them globally. Pin the behavior and admission recovery in production engine tests; Linux native host implementation remains pending under ADR 0043.

## [1.4.0] - 2026-09-08

### Added
- Add the Native SDK/Zig settings application, exact-sibling `settings` launcher and tray Configure routing, and a bounded versioned Rust service covering all 53 editable fields. Keep the terminal editor and add GUI/TUI update controls plus read-only `update --check`.
- Separate concise update eligibility text from the detailed JSON diagnostic, so unmanaged or unverifiable copies explain how to enable updates without exposing installer internals in the settings flow.
- Protect GUI and TUI saves with shared revision-aware atomic persistence and conditional runtime reload tokens tied to the selected file and saved revision. Preserve comments, compatible unknown fields, and drafts after validation or conflict failures.

### Changed
- Use embedded Makira Bold headings and Makira Light body text in native settings while retaining system appearance preferences. Tray Configure opens the runtime's selected configuration file.
- Replace the dual-view crossfade with one continuously projected goose (ADR 0041): rounder white body, restrained wings, solid orange bill and webbed feet, continuous heading, foot-contact weight shifts, acceleration/braking lean, bounded honk/pet/anticipation reactions, and default-on expressions with optional reduced motion. Existing saved color and behavior choices remain authoritative.
- Reuse bounded transparent damage canvases across platforms and the supersampled renderer; pace Windows/Linux ticks against the shared accumulator deadline rather than fixed 2 ms polling.

### Fixed
- Reject malformed or conflicting companion receipts before considering unmanaged launch; retain independent portable/source use alongside a valid foreign installation. On Linux, execute the retained verified file descriptor so sibling pathname replacement cannot redirect the graphical settings launcher.
- Hold the same configuration revision lock through login-start reconciliation so a concurrent GUI/TUI save cannot be followed by a stale OS setting. Detect external editors that ignore that lock before reporting success.
- Guard dashboard deletion against changed task-detail contents, and reject completion when task detail is missing or empty; preserve concurrent Status/Activity updates.
- Reconcile receipt-owned autostart intent before the native settings edit snapshot, preserve its resulting revision, and reject concurrent configuration changes during reconciliation.
- Keep native settings editable when runtime status cannot be confirmed, display an unknown state instead of stopped, and omit an unused native path field that could panic on non-Unicode filenames.
- Reject ordinary reload when its configuration file is missing, retaining the running choices instead of silently applying defaults.
- Capture the native Linux startup goose before issuing a Wander command that replaces its entrance task. Preserve the paired-background image oracle and later honk/mud/wander/reload checks; the ARM64 Debian fixture had repeatedly paused a partial edge pose.
- Correct AccessKit's Linux AT-SPI Enabled/Sensitive state for disabled buttons using a pinned, licensed local patch. Test the actual state translation and native disabled-to-available Update now workflow.
- Keep Windows owned note/meme delivery available when foreground activation is denied by normal OS focus policy. A failed `SetForegroundWindow` request no longer invents a Win32 error or permanently disables collect capability; note text still targets the owned edit control directly.
- Clear the inherited standard-handle flags at both Windows process entry points before child creation. Captured CLI, app and settings launches now receive EOF while the detached runtime remains alive; the native settings Start action finishes instead of retaining its service output pipe.
- Give each Native SDK license file its own WiX component and key path, satisfying automatic component identity rules in both Windows MSI editions.
- Stage the exact settings executable and accessibility DLL with the ARM64 installer qualification payload. Make GUI fixtures wait for semantic state after queued commands and reject partially rewritten SDK snapshots; answer the real TUI's initial cursor-position query in the disposable PTY fixture.
- Retire the verified TUI updater helper before closing its fixture PTY, recognize already-exited children, and bind cleanup signals to Linux process descriptors so PID reuse cannot redirect them. Preserve the live-helper, real update, receipt, relaunch and no-op assertions.
- Preserve the complete native accessibility tree after settings become dirty; the pinned toolkit and bridge now accept 256 bounded widgets. Keep test modules separate from platform link inputs even when optimization modes match, and restore the original macOS lifecycle cfg boundaries after module extraction.
- Forward native settings semantics to Windows UI Automation and Linux AT-SPI through bounded AccessKit adapters (ADR 0042). Bind the Windows bridge DLL to archive/slot/receipt identities and retain both file leases until GUI initialization. Pin the Zig compiler bootstrap and repair native CI toolchain selection.
- Keep forward steps planted ahead until the body passes over them, and give the rounded belly enough clearance for visible front/rear feet. Preserve the gait cadence and real ground contacts.
- Do not acknowledge native Wayland presentation while all retained buffers are busy or a required output is unconfigured; retry the pending damage, including graceful final clears.
- Retain and reap a bounded pool of audio voices/processes. Replace synthetic test-only limit checks with a real child-process exhaustion and recovery regression.
- Aim cursor-seeking locomotion at the body position that brings the beak to the pointer, avoiding a stationary-pointer chase that could never reach the grab distance.

### Development
- Give the Debian GUI intent fixture an explicitly older config timestamp: reproducible package unpacking preserves the receipt's build timestamp. Retain the strict newer-receipt, actual Read/Save and unchanged-receipt assertions with recorded timestamp evidence.
- Qualify non-Unicode configuration filenames on Linux and Windows, where the files are valid; retain macOS APFS's native EILSEQ rejection instead of treating an uncreatable filename as a service failure.
- Verify null Windows standard handles and valid inheritance in an isolated native test; the pinned API binding already skips null handles. Refresh the task dashboard while preserving its settings and stopped state.
- Require strict all-target workspace Clippy on both native Linux CI architectures. Limit the installer autostart glob import to its Windows and test callers; the explicit public re-exports remain unchanged.
- Wait for the GUI's new check response before retrying an update in the native Debian fixture; queued accessibility actions must not reuse the previous enabled state. Require a new retained helper before awaiting transaction success.
- Exercise captured Windows CLI/app starts through real process EOF and runtime readiness on x64/ARM64. Give the disposable Linux TUI update fixture its own controlling PTY and preserve failed terminal output for diagnosis.
- Scroll the real settings pane before toggling controls below smaller host viewports, checking every changed switch and all eight saved values while retaining the zero-dispatch-error gate.
- Keep disposable settings lifecycle checks scoped to their CI environment and use Linux's explicit headless fallback where the fixture has no compositor; the independent Linux overlay gate continues to require visible pixels.
- Qualify CLI, real terminal-editor keys and native GTK update actions with genuine Debian installation transactions. Exercise GUI download failure, unchanged receipt/runtime, retry, helper relaunch and public no-op results using an isolated terminal-output adapter. Exercise settings Start/Stop through the real runtime on disposable native desktops.
- Scope the goose's antialiased edge-color oracle to its actual silhouette so independently colored seasonal props do not contaminate the sample. Preserve global alpha, crop, black-rectangle, and channel checks; add paired-compositor and raw-DIB regressions for a separate translucent prop.
- Exercise Linux controls, text, focus and save through native AT-SPI; expose bounded UTF-8 text runs to assistive readers. Use the pinned x64 Zig build host and native ARM64 ReleaseSafe tests on Windows ARM, and install Alpine's separate virtual-display runner.
- Separate installer provenance, receipt-owned autostart, user-media migration, and existing lifecycle regressions into focused modules while retaining the public lifecycle API.
- Add a hosted Windows UI Automation check for actual Invoke/Toggle/Value patterns and modal isolation. Keep native bridge outputs ordered in Zig's build graph and compile Linux against matching distro headers with a baseline CPU.
- Add actual Rust motion exports, heading/alpha/bounds regressions, renderer timing workloads, and a tracked refinement audit. Native desktop, distribution, GUI, and complete release qualification remain in progress.

## [1.3.7] - 2026-07-23

### Fixed
- **Windows self-update lifecycle archive handoff (ADR 0040)** - generated Windows PowerShell now
  represents the archive-entry backslash as `[char]92` instead of transporting a quote-adjacent
  literal through `powershell.exe -Command`. This preserves the same traversal rejection and
  verified lease-extraction contract while preventing the expression from arriving as an empty
  `String.Replace` search value. The v1.3.6 updater failed safely before activation and left the
  v1.3.5 receipt/install untouched; v1.3.7 is the immutable fix-forward.

## [1.3.6] - 2026-07-23

### Changed
- **Calmer default mud cadence (ADR 0039)** - automatic puddle hops now occur every 180–300
  seconds instead of every 70–160 seconds, and their tracking window is 10–30 seconds instead of
  30–90 seconds. Fresh and direct mud defaults to 10 seconds instead of 15; existing explicit
  `mud.duration_to_track_seconds` values remain authoritative. The 8–15 second puddle trip,
  footprint lifetime/fade, rendering, sounds, manners, deterministic task flow, config schema,
  and TOML/TUI surface are unchanged.

## [1.3.5] - 2026-07-22

### Changed
- **A close cue that cannot blend into updater prose** - every retained tray-update result now
  separates the terminal status from a fixed-width ASCII panel. Successful and no-op results end
  with **HONK! ALL DONE** above the explicit close instruction; failures use **HONK! NEEDS
  ATTENTION** while preserving their diagnostics. The panel is structural rather than color-only,
  so it remains conspicuous in Windows Terminal, Apple Terminal, Linux terminals, redirected
  smoke output, and limited-color profiles.

## [1.3.4] - 2026-07-21

### Added
- **One-click native tray updates (ADR 0038)** - Windows notification-area, macOS status-menu,
  and compatible Linux StatusNotifier controls now include an always-visible **Update
  Honk300…** action. It launches a hidden private command from the exact installed executable in
  a visible terminal, runs the existing origin-preserving updater without manual typing, and
  performs no network check merely from opening the menu.
- **Verified app relaunch and visible completion** - the out-of-process helper serializes clicks
  with a crash-recoverable per-user lock, follows the protected receipt to the active executable,
  relaunches through the detached platform app path, and requires IPC readiness. Verified updates
  clear to one fixed completion screen; no-op checks choose from exactly 100 goose-themed lines
  while always stating the unchanged/current/running result; failures retain diagnostics and add
  receipt-owned recovery status. The helper deliberately stays alive until its terminal is closed.

### Changed
- **Typed internal updater outcome** - the tray helper consumes a typed updated/up-to-date result
  without parsing prose or changing `honk300 update [--json]`. Progress remains stderr-only,
  machine output remains exactly one stdout JSON object, and read-only no-op receipt discovery,
  origin/channel selection, exact-tag verification, rollback, cleanup-pending, autostart, and user
  content contracts remain unchanged.

## [1.3.3] - 2026-07-19

### Fixed
- **Windows app start from job-contained terminals (ADR 0037)** - the transient GUI launcher now
  breaks its hidden runtime out of an integrated terminal's breakaway-permitted Windows job, so
  command runners such as Codex return after readiness instead of tracking the goose for its full
  lifetime. Jobs that explicitly deny breakaway retain the original safe windowless launch; exact
  sibling identity, null handles, no-shell startup, singleton/readiness checks, and graceful or
  forced control semantics are unchanged.

## [1.3.2] - 2026-07-19

### Fixed
- **Windows bootstrap architecture detection in app-hosted terminals** - the versionless public
  PowerShell installer now falls back to Windows' machine-scoped processor architecture and then
  .NET's OS architecture when the launching app deliberately omits the inherited processor
  variables. The Codex app terminal can therefore run the documented command without an operator
  workaround, while x64/ARM64 selection and the exact-tag size/SHA-256 checks stay fail closed.

## [1.3.1] - 2026-07-19

### Fixed
- **Unelevated Windows update discovery** - owner-cleanup discovery no longer rewrites an
  already-current protected install receipt before downloading an update. Ordinary administrator
  terminals can reach the existing verified installer/UAC boundary, while real cleanup-state
  changes, conflicting-owner journals, slot identity, and the independently hashed
  `honk300-app.exe` remain fail closed.

## [1.3.0] - 2026-07-19

### Changed
- **Terminal-independent Windows start (ADR 0036)** - every public `start`, bare, and `plz`
  spelling through `honk300`, `honk`, or `goose` now forwards the complete start option vector to
  the exact sibling GUI-subsystem `honk300-app.exe`. The transient console controller and app wait
  for bounded IPC readiness and return, while the hidden `honk300.exe __windows-app-runtime`
  process owns the singleton, overlay, IPC, and notification-area item independently of the
  originating terminal. Missing launchers, spawn failure, early runtime exit, and timeout are
  nonzero; concurrent starts converge on the existing authoritative singleton without a shell or
  fallback launch path. Initial tray registration also retries without blocking IPC readiness so
  an immediate restart recovers its controls after Explorer finishes deleting the prior fixed-GUID
  item; a shell that stays unavailable reports explicit CLI-only degradation after three seconds.

### Added
- **Branded Windows app launcher** - `honk300-app.exe` now carries a multi-resolution icon derived
  from the canonical goose status artwork through a binary-specific Windows resource build. The
  build script and resource inputs ship in packaged source, while public aliases remain console
  subsystem 3 and the app launcher remains GUI subsystem 2 and user-pinnable.

## [1.2.6] - 2026-07-18

### Fixed
- **Windows PowerShell hashing compatibility** - the public bootstrap and CLI update coordinator
  now compute SHA-256 directly through the pinned .NET cryptography and file-stream APIs instead
  of depending on `Get-FileHash`. This retains regular-file, reparse-point, same-stream, size, and
  digest checks while supporting managed Windows PowerShell environments where that optional
  cmdlet is unavailable. Public v1.2.5 remains immutable; its six Mac/Linux/Debian public-byte
  lanes passed, while both Windows lanes exposed this bootstrap failure after the MSI completed.

## [1.2.5] - 2026-07-18

### Fixed
- **Semantic public Mac receipt verification** - the post-release shell/DMG takeover smoke now
  accepts the two exact autostart writer identities emitted by the managed shell bootstrap and
  the native app installer while still requiring autostart to remain disabled around a foreign
  LaunchAgent. v1.2.4's Mac installs, signatures, and release metadata were valid; only the
  verifier's transport-specific owner assertion failed after the successful DMG takeover.
- **PowerShell public-bootstrap qualification** - the post-release Windows takeover smoke now
  lets the verified public bootstrap own its required exclusive lifecycle lease instead of
  preoccupying that lease with a synthetic holder. Live-old-slot survival remains independently
  required on x64 and ARM64, and every installer subprocess now preserves stdout/stderr evidence
  so a future public-byte failure identifies the actual failing transaction.

## [1.2.4] - 2026-07-18

### Changed
- **Immutable v1.2.3 fix-forward publication** - the v1.2.3 tag remains unchanged and
  unpublished after its atomic release gate failed before draft creation. v1.2.4 carries the
  same command-first managed-installation implementation plus the qualification correction below.

### Fixed
- **Fail-closed hosted Windows tray qualification (ADR 0035)** - an explicitly opted-in
  GitHub-hosted Windows Server lane may now record fixed-GUID tray registration as unobservable
  only when Honk300's exact tray-owner window exists and an independent stock-icon
  registration/recovery probe also fails. The process-owned native menu Quit, force-menu,
  compositor, lifecycle, PE, and artifact-identity gates still run; ordinary x64 Windows CI keeps
  requiring observable initial registration and TaskbarCreated recovery without the waiver.

## [1.2.3] - 2026-07-18

### Added
- **Command-first managed installation (ADR 0034)** - the stable versionless PowerShell command
  is now the recommended Windows install and the stable versionless shell command is recommended
  on macOS/Linux. Each bootstrap still resolves immutable exact-tag payloads, verifies their
  recorded identity, writes a protected v2 receipt, and verifies the three public aliases.
- **Managed-channel takeover qualification** - candidate and public-byte gates now exercise
  Global MSI → PowerShell → Global MSI plus all Global/Corporate MSI/EXE transitions on both
  Windows architectures while an old process remains mapped. Signed Mac gates exercise shell →
  DMG → DMG-preserving update → shell; Debian packaging proves user-shell collision refusal.

### Changed
- **Fresh Mac intent is distinct from update transport** - an ordinary public shell invocation
  becomes the new shell owner, while `honk300 update` supplies an allowlisted internal hint that
  is revalidated against the protected receipt before preserving DMG or shell provenance. Both
  exact managed app receipts retain the same Accessibility onboarding eligibility. A narrow,
  receipt-validated temporary-filename bridge preserves the owner for already-released clients
  that predate the hint.
- **Native packages are supported alternatives** - the signed DMG, four Windows MSI/EXE
  families, and architecture-matched Debian packages remain full lifecycle owners. Raw Cargo,
  source-tree, and portable binaries remain unmanaged and cannot retire another installer owner.

### Fixed
- **Fresh Global MSI origin takeover** - a directly launched Global MSI no longer inherits a
  previous PowerShell origin from the registry. Only the verified PowerShell bootstrap passes
  `HONK300ORIGIN=powershell`, so later updates preserve the user's actual latest install choice.
- **Linux cross-scope collision safety** - the no-sudo shell bootstrap validates and refuses an
  existing Debian owner before staging. Debian `preinst` checks all three managed aliases for
  every account home in the password database and refuses before package commit, giving an exact
  owned-uninstall-and-retry command without deleting user files as root.

## [1.2.2] - 2026-07-18

### Fixed
- **Semantic post-release receipt verification** - the live macOS/Linux installer smoke now
  parses the v2 receipt as JSON instead of requiring the bootstrap's original single-line
  formatting. This preserves the complete install/start/X11/Wayland/rollback gate after runtime
  legitimately rewrites the same receipt as pretty-printed JSON; product behavior is unchanged
  from v1.2.1.

## [1.2.1] - 2026-07-18

### Fixed
- **Foreign login-start ownership during shell updates** - the macOS/Linux bootstrap now records
  login start as enabled only when the existing LaunchAgent or XDG desktop entry carries
  Honk300's ownership marker. A foreign regular file, symlink, or directory is preserved and
  treated as Honk300 autostart being off; disabling remains a no-op for foreign state, while
  enabling still refuses the ownership conflict. This fixes the v1.2.0 public Linux shell smoke
  regression without weakening mutation preflights.

## [1.2.0] - 2026-07-18

### Added
- **Provenance-preserving slot updater (ADR 0031)** - adds authoritative
  `honk300.install.v2` receipts and immutable version/target release slots. Windows installers
  activate through stable `current`/`bin` junctions; shell-managed Linux activates through an
  atomic `current` symlink. Receipts record installer family, edition, scope, stable track,
  target, owned root, active release, and exact artifact identity while retaining v1 migration
  reads.
- **Live-image and rollback qualification** - adds a Windows smoke that keeps the old executable
  mapped while switching installer origins, injects a failure after junction retargeting, proves
  the previous selector and receipt return intact, then verifies all three aliases and hashes.
- **Configurable login start (ADR 0032)** - adds default-off
  `[lifecycle].autostart_on_login` configuration and a General-TUI **Start on login** control.
  Saves reconcile through the active receipt's existing HKLM/HKCU Run value, macOS LaunchAgent,
  or per-user XDG autostart entry; foreign integrations and unknown provenance fail closed.
- **Explicit force-stop grammar** - all three command names accept `stop`, `quit`, and `exit
  --force` through a distinct `FORCE_STOP` IPC command for immediate termination. The same
  commands without `--force`, plus tray/menu Quit, retain the engine-owned offscreen walk-off.

### Changed
- **Synchronous same-origin updates** - `honk300 update [--json]` now waits for the matching MSI,
  EXE, PowerShell, DMG-origin app ZIP, Debian package, or shell bootstrap and verifies activation
  before exit zero. Progress is stderr-only and JSON mode emits exactly one final stdout object.
  Unknown or conflicting provenance stops with an assisted-reinstall command instead of guessing
  Global MSI.
- **Latest install is latest intent** - slot-aware MSI and EXE installers permit intentional
  downgrade, stage payloads before activation, preserve mapped old releases, and prevent an
  inactive owner's uninstall from deactivating a newer cross-channel selection. The graphical
  Mac install remains the signed DMG, Debian remains dpkg-owned, and package/bootstrap origins do
  not convert during update.
- **Conflicting Windows owner cleanup** - after activation, validated MSI/Inno conflicts are
  recorded in a protected cleanup journal. `honk300 update` retries only those exact registered
  uninstallers and verifies their removal; denied opposite-scope elevation remains nonzero
  `cleanup_pending` with the new slot preserved instead of falsely claiming public PATH takeover.
- **Monitor-relative collected props (ADR 0032)** - Windows and macOS use one engine-owned fit
  contract: notes target 32% of the receiving monitor with a hard 48% ceiling; images preserve
  aspect ratio, include the complete source without cropping, never upscale, and stay within 48%
  of either monitor dimension and 900x700 logical pixels.
- **Installer/config autostart precedence** - Windows slot receipts now record the actual MSI
  feature or Inno task selection. Package updates preserve it; a newer fresh-installer receipt is
  mirrored into config as latest intent, while a newer explicit config edit updates the one owned
  platform integration and receipt.

### Fixed
- **Windows self-update lock ordering** - removes the detached wait-for-parent and Restart Manager
  unlock dependency. Installer-owned activation now verifies staged payloads, retargets neutral
  selectors in place, commits the protected receipt, and lets the initiating CLI verify the final
  version, target, aliases, and artifact identity while still running from its old release.
- **Cross-scope Windows PATH retirement** - runs each validated old owner and its exact PATH/Run
  cleanup inside one hidden coordinator under the same required administrator grant, then verifies
  the active root's persisted PATH. Permanent MSI components can no longer leave an older
  machine-wide command ahead of a newly authoritative Corporate install. Registration validation
  accepts Windows Installer's protected-machine inventory for a per-user Corporate MSI without
  weakening the HKLM-only Global or HKCU-only Corporate EXE boundaries, and coordinator argument
  quoting normalizes Windows Installer's trailing root separator before elevation. Identical
  per-user registrations exposed through both shared registry views count as one logical owner;
  differing evidence remains conflicting.
- **Transactional receipt rollback** - preserves the prior protected receipt when activation
  fails before its backup is created, restores source markers with the selector, and retains an
  already-existing same-version Linux slot during injected post-activation rollback.
- **No local full-screen calibration (ADR 0033)** - product startup performs no screen
  calibration, and local Windows qualification now refuses to create the old full-desktop
  dark/near-white surfaces even with the former opt-in flag. Strict paired-color compositor proof
  runs only on disposable GitHub Actions desktops.
- **Owned Windows note presentation (ADR 0032)** - replaces external Notepad launch and synthetic
  input with a process-owned native editable note window. Windows session restoration can no
  longer expose a user's existing tabs, and Honk300 never opens or executes a script as note
  content.
- **Windowless Windows app startup (ADR 0033)** - keeps `honk300.exe` as the console-subsystem
  public CLI while shipping an independently hashed GUI-subsystem `honk300-app.exe` for Start
  Menu/desktop shortcuts and login startup. It launches the exact sibling runtime with
  `CREATE_NO_WINDOW`, null standard handles, and no PowerShell, shell association, visible helper,
  or focus-stealing console. Config restarts and background lifecycle helpers use the same
  no-console process boundary; only intentional CLI/TUI use and explicit **Configure Honk300…**
  may show a terminal.

## [1.1.0] - 2026-07-17

### Added
- **Windows and Linux tray control parity (ADR 0030)** - adds one accessible **Honk300 controls**
  surface while the goose is running. Windows uses a fixed-GUID native notification-area icon;
  compatible Linux desktops use a pure-Rust StatusNotifierItem with an embedded contrasting goose
  icon. Both expose only **Configure Honk300…** and **Quit Honk300**.
- **Native lifecycle and recovery qualification** - Windows restores the same icon after Explorer's
  `TaskbarCreated` broadcast and Linux recovers after its StatusNotifier watcher returns. Hosted
  Linux tests exercise watcher/host registration, accessible properties, menu events, recovery,
  and explicit no-host or no-session-bus behavior.

### Changed
- **One control-surface command path** - macOS, Windows, and Linux now emit the same finite
  `Configure`/`Quit` command type. Configure launches the exact running executable's existing
  terminal TUI without shell interpolation; Quit reaches only the engine's animated graceful
  walk-off and never terminates directly from a native callback.
- **Linux package integration** - Debian packages now own the shared icon in the hicolor theme and
  reference it from the existing desktop entry. Portable archives keep the icon embedded and add
  no GTK, AppIndicator, or compositor dependency.

### Fixed
- **Windows TUI launch isolation** - the tray launcher creates a normal new console without
  inheriting the goose process's redirected standard handles, preventing a blank configuration
  screen while leaving the running goose and singleton intact.
- **Explicit tray-unavailable boundaries** - a missing Windows shell owner or Linux session-bus/
  StatusNotifier host is non-fatal and visible while CLI, TUI, IPC, overlays, and supported
  mischief remain independent. Windows ARM64 hosted qualification may use this waiver only when
  an independent stock-icon `Shell_NotifyIconW` registration also fails.
- **Windows Server tray-recovery qualification** - keeps the `TaskbarCreated` recovery check
  strict while first requiring the simulated `NIM_DELETE` to remain settled for one second, then
  allowing the shell's rectangle API a bounded ten seconds to expose a successfully restored icon.
  This matches Explorer's destroy-then-broadcast ordering and records both poll counts plus runtime
  restoration diagnostics. A Server 2022 release runner may record recovery as unobservable only
  when a separate hidden owner with a stock icon and fresh GUID also fails the same bounded
  recovery sequence; Windows Latest, local/self-hosted Windows, and the physical Alienware remain
  strict.

## [1.0.3] - 2026-07-17

### Changed
- **Corporate MSI retry semantics (ADR 0029)** - schedules old-product removal after the new
  per-user installation commits. An injected mid-upgrade failure now leaves the old product and
  bytes untouched; retry and uninstall complete without the reproduced orphaned component client.
  The machine-wide Global MSI keeps its existing transactional schedule.
- **Conservative integrated-terminal protection** - Windows now protects Visual Studio Code,
  Codex, and the observed ChatGPT-titled Codex desktop surface from window mischief. Linux adds
  equivalent `code`/`codex` application-token coverage, matching the existing macOS policy while
  leaving ordinary applications eligible.

### Fixed
- **Native Windows update discovery and download** - keeps URI/output values outside PowerShell
  command source, uses non-interactive stop-on-error requests, decodes Windows PowerShell 5.1 byte
  responses as UTF-8, and preserves exact-tag size/SHA verification. Both installed v0.3.1 and
  exact portable v1.0.2 reproduced the old parser failure; rebuilt source passes live discovery
  and a quoted-path exact download. Because the immutable older executable cannot repair itself,
  Windows users on v1.0.2 or earlier must rerun the supported installer once to reach v1.0.3;
  ordinary installer upgrade preserves user content, and subsequent update checks use the fix.
- **Coexisting Global/Corporate lifecycle identity** - the marker beside the invoked executable
  now wins before registry fallback, and Corporate uninstall discovery searches current-user then
  machine registration while retaining exact product/publisher/ProductCode/root validation.
- **Deferred Windows parent completion** - a helper now treats an already-exited parent as a
  successful wait without suppressing genuine lookup/wait failures.
- **PowerShell-bootstrap receipt freshness** - after an exact-path/version-verified direct Global
  MSI update, only a complete regular non-reparse receipt proving the expected schema, channel,
  layout, and root is atomically refreshed to the new release identity. Missing, malformed,
  foreign, mismatched, and reparse receipts remain untouched.

## [1.0.2] - 2026-07-17

### Added
- **Shared goose control-surface icon (ADR 0028)** - added a Quiver-generated, transparent,
  two-path monochrome SVG as the canonical future macOS/Windows/Linux menu/tray source, with
  prompt/generation provenance, deterministic export guidance, an accessible-name contract, and
  an AppKit-safe 36×36 RGBA representation. v1.0.2 implements only the macOS item; future Windows
  notification-area and Linux desktop surfaces must reuse the same Configure/TUI and engine-owned
  graceful-Quit behavior rather than create another settings model or immediate exit.

### Changed
- **Accessible image-only macOS status item** - replaced the visible `Honk` title in production
  bundles with the 18-point goose template image, square item sizing, proportional scaling, and an
  independent `Honk300 controls` accessibility label/tooltip. The Rust owner retains both image
  and AppKit's weak action target. Missing or undecodable resources keep the variable-width
  `Honk` development fallback without preventing Configure or Quit from launching.
- **v1.0.2 atomic publication closure** - replacement candidate `29565557915`, exact-SHA main CI
  `29566294408`, atomic release `29566759574`, and post-release smoke `29567257622` passed at
  `964305869e9ec28768c789465db1b6317dfa3f6f`. The immutable latest release contains 22 payloads
  and 47 total assets. Fresh public Mac app/DMG hashes, G2 Developer ID identity, hardened runtime,
  notarization, staples, Gatekeeper, resources, universal slices, graphical lifecycle, the real
  v1.0.1→v1.0.2 three-alias managed update, and repeat no-ops passed. The deployed progressive-
  disclosure site reports that exact release and recommended immutable DMG; all published Windows,
  Linux, Debian, and Mac installer smokes passed.
- **v1.0.1 publication closure carried forward** - retained the exact candidate, same-SHA main CI,
  atomic publication, post-release native smoke, independent Developer ID/notarization/stapling/
  Gatekeeper, v0.3.2 upgrade, and DMG-first site evidence without mutating its immutable tag or
  assets. Physical-Mac follow-up additionally proved the published menu/TUI, same-process
  Accessibility grant, readable dark note, 5.415-second animated Quit/restart, and two passing
  process-specific 60-second CPU/RSS profiles; one-display, absent-Ghostty, and hardened-runtime
  leak-attach limitations remain explicit.

### Fixed
- **Old-macOS icon decoder dependency** - the runtime now loads a sealed PNG representation rather
  than depending on raw SVG decoding across the macOS 11+ support range. Native AppKit tests
  require a nonzero representation, 18-point size, and template state.
- **Bounded Windows named-pipe startup reads** - a successful zero-byte poll from a connected
  `PIPE_NOWAIT` command pipe is now treated as a transient scheduler state instead of an empty
  command frame. The existing deadline still fails closed when a peer never supplies data, and
  Windows-target regressions pin both the delayed-frame and bounded-timeout paths. This repairs
  the only failure in the first v1.0.2 candidate without changing the shared command protocol.
- **Release-shape icon enforcement** - macOS packaging copies the canonical SVG and runtime PNG
  before signing and fails closed unless both survive the staged app, extracted final app ZIP,
  initial DMG mount, and final notarized/stapled DMG remount. The updater regression now explicitly
  recognizes v1.0.1→v1.0.2 as a forward in-place update.

## [1.0.1] - 2026-07-15

### Added
- **macOS menu-bar Configure and graceful Quit (2026-07-14, ADR 0024)** - the running
  `LSUIElement` app owns one main-thread AppKit status item labeled `Honk`. Its accessible
  Configure action opens a signed-bundle `Configure Honk300.command` resource which executes the
  same app binary's existing ratatui `config` entry point; Quit sets the shared engine graceful-
  stop intent so simulation/presentation continue through the fully offscreen walk and final
  transparent acknowledgement. The bridge explicitly retains AppKit's weak menu target for the
  item lifetime, removes the item on runtime teardown, and adds no native settings schema/window,
  running Dock control surface, AppleScript API, global key, or Windows/Linux tray. Packaging
  contracts require the executable launcher before signing and in the final app ZIP and both DMG
  validation passes. A local packaged universal app exposed all items through macOS accessibility
  inspection, opened/restored the full TUI in Terminal, and completed animated Quit in four
  seconds; exact-final-SHA interaction repetition is tracked as post-release forward verification.
- **Managed macOS Accessibility first run (2026-07-13, ADR 0022)** - only the exact receipted app
  at `~/Applications/Honk300.app` can open automatic permission UI. Before calling the native
  consent request and opening Accessibility settings, the runtime atomically creates an
  owner-only `accessibility-prompt-v1/<version>` marker; mismatched metadata/receipts, symlinks,
  unsafe state, development binaries, and direct mounted-DMG launches fail closed without UI. A
  denied managed app enters a platform-neutral safe-edge wait which blocks automatic and direct
  pranks except honk while preserving status/reload/stop. One-second polling restores capability
  state and starts FirstUX after a live grant, or abandons permission-bound work and returns to
  the non-nagging wait after revocation. Focused engine, eligibility/marker, native bridge, and
  smoke-contract tests pass. One unchanged signed executable completed denied, non-nag relaunch,
  same-process grant, and same-process revoke on the physical M2; exact-final-SHA repetition is an
  explicit source-equivalent forward-verification waiver rather than a stronger claimed run.
- **Developer ID macOS distribution (v1.0.1, ADR 0020)** - the universal app now carries exact
  version/tag/commit metadata, and the DMG contains `Honk300.app`, a separately signed native
  `Install Honk300.app`, and concise per-user instructions without an `/Applications` symlink.
  The helper verifies the target bundle id, signature, and matching nonempty Developer ID team,
  requires the fixed `M9D5379H93` Developer ID Application certificate identity on both apps,
  delegates to the shared no-sudo install transaction, reports native failures, and opens the
  installed app on success.
- **Native Debian packages (2026-07-14, ADR 0023)** - every release now assembles stable
  `honk300-amd64.deb` and `honk300-arm64.deb` assets from the byte-exact, already-qualified GNU
  archive executables. Each package owns `/usr/lib/honk300/honk300`, stable `/usr/bin` aliases,
  its source marker, desktop entry, license, and exact release metadata while keeping mutable
  media in user XDG storage. Debian provenance adds target/kind/architecture-isolated CLI update,
  real `dpkg` removal, non-purge media preservation, purge backup, ownership proof, and native
  amd64/arm64 post-release compositor/lifecycle smokes.
- **Fail-closed signing and notarization pipeline** - candidate and release macOS jobs now import
  a password-protected Developer ID P12 into an ephemeral keychain, sign nested code inside-out
  with hardened runtime and secure timestamps, notarize/staple/validate the app and DMG, retain
  notarization JSON as internal evidence, and delete temporary credentials. All six certificate
  and App Store Connect secrets are mandatory; release mode has no ad-hoc fallback. The stapled
  final DMG is remounted and Gatekeeper-assessed for both contained apps, proving the user launch
  path rather than only the disk-image container; both the main app and graphical helper sign and
  verify their executable before their outer bundle rather than relying on recursive signing.
- **Native macOS pixel and screenshot contracts** - asymmetric RGBA round trips through AppKit
  and CoreGraphics pin channel/alpha semantics, while semantic captures require a visible body,
  outline, wing, beak, legs, and shadow on light and dark backgrounds.
- **Cross-platform presenter byte-order contracts** - asymmetric color/alpha tests also pin the
  intentional premultiplied RGBA-to-BGRA conversion used by Windows layered windows and both
  Linux X11 and Wayland presenters. This extends the macOS regression audit to every native
  pixel bridge without changing the platform-neutral renderer output.
- **Linux compositor-visible release evidence** - the native smoke can hold an exact
  `HONK300_BIN` unchanged instead of silently rebuilding it. Headless Sway now has two
  fractional-scale outputs and a deterministic background, and `grim` captures the actual
  composed desktop for semantic body/wing/asymmetric-orange assertions rather than checking only
  the renderer's internal PNG. X11 also fails closed unless the selected direct visual is the
  exact little-endian, 32-bpp ARGB8888 ZPixmap layout required by its hard-coded BGRA upload.
- **Windows compositor-visible release evidence** - native CI and the reusable release-candidate
  workflow now run exact x64 and native ARM64 executables through start/status/single-instance/
  reload/stop and immediate-restart lifecycle checks; the published-MSI smoke repeats the x64
  path. The native ARM job consumes the exact executable produced for ARM installer packaging,
  rather than accepting an independent rebuild as equivalent. Each smoke freezes a real
  `UpdateLayeredWindow` surface and captures that unchanged articulated pose over controlled dark
  and light desktops. A dependency-free analyzer assigns every pixel to one nearest palette color
  and fails closed unless the captures independently prove transparent margins, reconstructed
  semantic edge colors, body/shade/outline/wing, the asymmetric orange channel order, and one
  complete renderer view. Side view additionally requires spatially separated beak/two-tone legs
  plus shadow; top-down requires its compact beak, complete wing/body geometry, low shade share,
  and intentional lack of legs/shadow. Screenshots, analysis, logs, and the executable hash are
  retained even on failure.
- **Cross-platform exposed-edge traversal and animated lifecycle** - `DesktopLayout` now derives
  real exposed edges by subtracting touching monitor seams. Four wander entries plus one
  `EdgeWrapTask` keep hidden Pac-Man-style wrapping at 20% of baseline deck draws; relocation is
  legal only after the entire rendered pose is outside every monitor, and deliberate puddle/prank
  excursions return through their own edge without wrapping. Initial startup is staged fully
  offscreen. Stop/exit/quit uses an adaptive Run-to-Charge-bounded `GracefulExitTask`, continues
  ticking/presenting until the full pose is hidden, and gives singleton release a finite 30-second
  client wait for immediate-restart safety on every backend.
- **User-close annoyed reaction** - native macOS and Windows collect snapshots distinguish a user
  closing a spawned note/meme from Honk300 cleanup. An independent deterministic stream selects a
  visible annoyed reaction with 30% probability and may then chain the existing bounded nab only
  when mouse-steal configuration, manners, pointer state, permission, and capability allow it.
  Program cleanup never rolls; Linux remains explicitly collect-unsupported and has no trigger.

### Changed
- **First public stable major release (2026-07-15, ADRs 0025 and 0027)** - the prospective
  unpublished v0.3.3 milestone ships as v1.0.1. The immutable v1.0.0 tag failed closed before a
  draft or public asset existed and remains unchanged under the atomic-release fix-forward rule.
  Package, app-bundle, receipt, manifest, installer, updater, readiness, and website identities
  advance together; v0.3.2-to-v1.0.1 is an explicitly tested normal upgrade. Additional Alienware
  hands-on verification happens after publication and any findings ship in later forward patches
  without rewriting the immutable v1.0.1 tag or assets.
- **Complete rolling-latest release channel (2026-07-14, ADR 0023)** - every general tag now
  requires the complete cross-platform producer matrix, including a fresh GitHub-macOS-built,
  Developer ID-signed/notarized/stapled app and DMG plus both Debian packages regardless of the
  operator's trigger host. Stable unversioned `latest/download` installer names advance only
  after atomic publication; existing tags and their assets remain immutable. All three CLI names
  read the latest manifest only for discovery, then download the platform/provenance-matched
  payload from its exact tag and verify kind, target, size, and SHA-256 before mutation. Managed
  Mac apps update through the exact-tag universal app ZIP selected by the pinned bootstrap, not
  by replacing from the DMG.
- **Quicker shared walk recovery** - the platform-neutral planted-foot trigger now releases at
  four pixels and normal/moderate recovery keeps its weighted 70%-of-beat cadence. A speed-aware
  body-travel cap shortens only Run/Charge recovery: tests bound Walk lag at 16 pixels and
  Run/Charge at 26 pixels while guarding visible airtime and plant cadence against twitchy
  overcorrection. The shared fix applies on Windows, macOS, X11, and Wayland; three gait-dependent
  renderer goldens were deliberately refreshed before the tier cap, which requires no additional
  golden changes.
- **Lower-allocation renderer and macOS runtime** - renderer scratch surfaces grow in bounded
  increments, stippled shadows are cached, and opaque 1x frames can paint directly. macOS reuses
  bitmap/image storage, avoids swizzle buffers and redundant window/display work, drains native
  objects in autorelease pools, caches desktop geometry, retains 120 Hz simulation, and caps
  presentation at 60 Hz. The AppKit view now draws only the active RGBA rectangle and both the
  tiny-skia canvas and native bitmap shrink after an unusually large note, meme, or distant dirty
  region instead of color-converting that stale transparent capacity on every later frame. Its
  alpha-last bitmap remains Device RGB while the overlay window uses a stable standard-sRGB
  destination, avoiding a redundant per-frame Device-RGB-to-Display-P3 ICC/vImage conversion and
  leaving final per-display-profile composition to WindowServer. The active diagnostic measured
  5.55% median CPU, 29.52 MiB maximum RSS, negative 9.89 MiB growth, zero leaks, and 20 clean
  compositor captures. Exact-final-SHA repetition is retained as post-release hardware
  verification under the accepted source-equivalent waiver.
- **Mounted-bundle lifecycle transaction** - `honk300 install` can copy its enclosing mounted
  source app into `~/Applications/Honk300.app` after validating bundle id, stamped release
  identity, strict signature, and both architectures. It preserves the shared aliases/media/
  autostart/receipt/update/uninstall contract and performs an atomic bundle swap with rollback;
  a candidate-only fault hook proves the previous bundle and receipt return after activation.
  Late failures also restore/remove aliases, the LaunchAgent, receipt, and only migration-created
  media while preserving existing user content.
- **Exact-artifact native smoke** - both macOS smoke scripts accept `HONK300_APP` plus
  `HONK300_SKIP_BUILD=1`, reject missing bundles, and create their config only through `setup`,
  allowing denied and granted Accessibility evidence on one unchanged signed identity.

### Fixed
- **Pose-complete Windows release qualification (2026-07-15, ADR 0027)** - release run
  `29398343807` at exact candidate/default-branch SHA `9c5692b` failed atomically before draft
  creation because ten valid top-down entrance captures were judged by a side-only legs/shadow
  oracle. The paired-DWM and hosted-ARM64 raw-presenter analyzers now share strict side and
  top-down profiles plus reconstructed mid-alpha color proof. Red/blue swaps, straight alpha,
  double premultiplication, opaque/black surfaces, missing warm articulation, damaged side
  fallthrough, and partial/cropped top-down frames remain fatal. Retained replay accepts only
  complete attempts 6–12 and rejects partial attempts 3–5. No renderer, engine, presenter, gait,
  or user-visible behavior changed; v1.0.1 rebuilds and requalifies the entire matrix.
- **Hermetic Wayland compositor evidence (2026-07-15)** - exact candidate `29389882143` passed
  every release producer, native compositor/package job, and final assembly at commit `c44b89d`.
  Ordinary main CI then exposed a non-hermetic difference: installing Ubuntu's recommended Sway
  packages loaded `/etc/sway/config`, its wallpaper, and its bar on `HEADLESS-1`; Honk300's
  transparent layer correctly revealed that distro background while `HEADLESS-2` showed the
  requested solid. A first private config still contained its own wildcard background, and the
  exact setters still used `solid_color`. Ubuntu 24.04's `swaybg` represents that color as a
  one-pixel protocol buffer; linear filtering on the 1.5-scale pixman output sampled its outside
  edge and produced a deterministic gradient before Honk300 even launched. The smoke now starts
  with no background rule, creates constant opaque PNG tiles, addresses both discovered outputs
  only by exact name, and proves paired goose-free baselines before launching Honk300. Fractional
  filtering stays enabled. No product background, renderer behavior, or semantic threshold changed.
- **Candidate-native evidence calibration (2026-07-14, ADR 0026)** - candidate `29387569722`
  passed the trusted Mac producer, Windows x64 paired-DWM compositor/lifecycle proof, native
  Windows ARM64 PE/MSI lifecycle, and X11 plus dual-output Wayland on three of four Linux
  variants. Its remaining Linux x64 GNU capture was a valid top-down goose with 710 body, 1,615
  wing, and 13 warm articulation pixels, so the Linux warm floor is now 10 while all body, wing,
  background-transition, transparency, and opaque-surface checks remain unchanged. GitHub's
  hosted ARM64 runner separately returned one byte-identical wallpaper for two visible ordinary-
  window colors. Only that exact GitHub-hosted signature may now use the real process's cropped
  premultiplied-BGRA DIB, atomically recorded after successful `UpdateLayeredWindow` and bound to
  the frozen visible HWND/rectangle. Raw checks reject straight alpha, double premultiplication,
  swapped channels, mostly opaque-black surfaces, missing articulation, and stale metadata.
  Windows x64 plus local/self-hosted ARM64 still require paired live DWM captures; the hosted ARM64
  result is never described as desktop-composition proof. Candidate `29389046641` then passed the
  complete Mac producer, every Linux compositor job, both Debian package jobs, and Windows x64;
  both hosted ARM64 jobs also produced multiple surfaces that passed every raw semantic check but
  rejected them because the moving overlay advanced one or two pixels between the atomic record
  and controller suspension. The binding now keeps exact HWND identity, polls at five
  milliseconds, and permits only a three-physical-pixel origin/dimension delta covering the
  observed single-interval drift; larger movement retries. No pixel, alpha, articulation,
  runner-identity, or paired-DWM requirement changed.
- **Portable native-smoke runtime paths and Windows diagnostics (2026-07-14)** - the first
  v1.0.0 candidate proved all four repaired X11 compositor paths and matching Windows controller/
  background geometry, then failed closed before product capture. Sway's AF_UNIX sockets now use
  a short owner-only temporary runtime directory instead of an evidence path that can exceed
  Linux's 108-byte limit, and cleanup removes it while retaining logs/screenshots. Windows
  geometry validation now splits exact CRLF/LF diagnostic lines instead of applying a Unix-style
  end anchor to a CRLF document. No product background, renderer behavior, or semantic capture
  threshold changed.
- **Final-source native candidate diagnostics (2026-07-14)** - after native `BitBlt` repaired the
  first Windows capture failure, candidate `29384134561` proved the notarized Mac producer and
  Windows x64 path but failed closed because `xcompmgr` held its cached gray root tile on every
  Linux target and the ARM64 Windows controller captured neither its ordinary TopMost background
  nor the overlay before racing the shared color file. X11 qualification now uses a persistent,
  test-only opaque client behind the goose, changes its controlled color through an atomic
  command plus acknowledgement, and forces real compositor damage without adding any background
  to the product. Both Windows smoke processes enter per-monitor-v2 DPI awareness before WinForms
  or HWND creation, exchange tokenized atomic color requests, prove matching physical virtual-
  screen geometry and a real dark/light capture before launch, and retain per-overlay DPI/rect
  evidence. Cross-platform `status` output also treats only a downstream `BrokenPipe` as normal
  while preserving every other write failure. Semantic goose/color/alpha thresholds remain
  unchanged; the source differs from the proven candidate and must rerun exactly before release.
- **Collect pickup now aims locomotion at the beak interaction point** - live macOS qualification
  exposed a side-view deadlock where the body stopped at the prop center but completion measured
  distance from the neck-height beak. Locomotion now recomputes the body target from current beak
  geometry each tick, keeping the beak-distance arrival gate authoritative. A realistic 120 Hz
  world regression reaches passthrough/grab and note typing within 15 seconds without teleporting
  the beak, and the complete integrated local gate passes. A product-equivalent signed app spawned
  and typed a readable native note; exact visual beak-contact capture remains post-release
  verification and is not overstated as completed native evidence.
- **Lingering collect windows cannot starve a newer request** - the macOS and Windows native
  controllers now prefer the most recently spawned typed request over older notes or memes that
  remain open. Dead-window events are still drained first, matched by request id and kind, and
  removed exactly once, so delayed user-close reactions remain truthful without making a newer
  collect task time out behind arbitrary map iteration order.
- **Current macOS hands-on release runbook** - replaced the historical ad-hoc, unnotarized,
  terminal-first checklist with the exact v1.0.1 Developer ID/notarized/stapled DMG-first flow.
  The runbook now preserves one candidate identity across four Accessibility states, native
  terminal protection, renderer and performance evidence, lifecycle rollback, the one-display
  waiver, immutable publication, and fresh-download verification. The README now links the exact
  release DMG and distinguishes the stapled app inside the ZIP from the ZIP container itself.
- **Readable macOS notes in every appearance** - note text now uses AppKit's semantic label color
  instead of absolute black, so it follows light, dark, and increased-contrast appearances. A
  native regression pins the semantic-color contract. The equivalent platform paths were audited:
  Windows delegates note colors to system Notepad, while Linux explicitly reports collect windows
  unsupported rather than drawing a separate note surface.
- **macOS transparent-white/purple goose** - the AppKit bridge now declares tiny-skia's direct
  premultiplied RGBA bytes as alpha-last instead of interpreting them through the former
  BGRA/alpha-first contract. Native captures show the articulated goose rather than a translucent
  color blob; the seven shared engine goldens remain stable apart from the intentional gait poses.
- **macOS overlay capture and post-transient CPU** - replaced the custom child Core Animation
  layer with a reusable `NSImageView` in the ordinary AppKit backing store. WindowServer capture
  and screen sharing now receive the same transparent pixels as the physical display instead of
  omitting the goose or turning unused surface capacity into black rectangles. Capacity-shrink
  regressions prevent a large transient frame from making later normal walking redraw and color-
  convert an oversized image. Product-equivalent active-motion capture and profiling clear the
  budget; exact-final-SHA repetition remains explicit post-release verification.
- **macOS lint debt** - removed the obsolete world-bounds path and corrected the three remaining
  target-specific warning failures so workspace clippy can run with `-D warnings` on macOS.
- **Stop/start singleton race** - a successful `stop`, including the `exit` and `quit` aliases,
  now keeps the runtime alive while the goose walks completely beyond its nearest exposed edge,
  then waits for the shared singleton to be released before returning. Exit speed adapts within
  the existing Run/Charge envelope. An immediate restart can no longer lose the race between the
  old runtime acknowledging shutdown and actually exiting on Windows, macOS, or Linux; the wait
  is bounded and reports a stalled shutdown explicitly.
- **Install/update/uninstall lifecycle race** - managed mutations now stop the active runtime and
  retain its real cross-platform singleton as a `LifecycleLease` until every owned file and
  integration change is finished. The exact-tag Unix installer holds that lease through a staged
  binary and parent-owned FIFO; Windows bootstraps and updaters use the manifest-hashed portable
  executable as a redirected-stdin lease holder. Windows uninstall first performs ownership-only
  preflight, then hands off to a private temporary copy which owns the singleton before the
  installed CLI exits, preventing both running-EXE deletion failures and partial mutation before
  a helper is ready. Verified Windows payloads remain pinned by the same read-only stream through
  execution, generated updates explicitly release/reacquire rather than trusting ambient state,
  Restart Manager checks exact machine-wide paths across sessions, and reboot-deferred MSI results
  fail closed. Unix signals carry explicit rollback statuses, and every pre-READY deferred helper
  is killed and reaped on error. Missing, rejected, timed-out, or interrupted ownership fails or
  rolls back before an unowned partial state can be accepted.
- **Release-board runtime debt** - re-audited the shared `RuntimeCore` and the Windows per-monitor
  dirty-region presenter against their deferred cards. Focused sequencing, 4K bounded-damage, and
  Windows x64/ARM64 cross-target checks confirm the old loop-duplication and fullscreen-redraw
  follow-ups are resolved by the current architecture.
- **Native Wayland capability decision (ADR 0021)** - completed the upstream protocol/compositor
  audit and published a portable/wlroots/KDE/GNOME/portal/XWayland/privileged-helper matrix. The
  portable layer-shell mode remains honestly reduced; future near-parity work is split into
  explicit, permissioned compositor adapters rather than one false universal-support claim.

### Security
- **Local release credentials stay outside Git** - `/.private-release/` is ignored as the
  owner-only location for one-time local signing/notarization material. Credential contents are
  supplied to GitHub only through encrypted Actions secrets and never tracked in the repository.
- **Bound graphical installation** - mounted-source installation preflights foreign aliases,
  receipts, LaunchAgents, symlinks, bundle identity, release metadata, architecture, and code
  signature before mutation. DMG receipts bind updates to the exact tag and full commit, and the
  helper refuses adjacent apps signed by another team. Both graphical and terminal transactions
  also refuse to replace an existing app that lacks a matching Honk300 ownership receipt.
  Receipt checks use no-follow metadata so even a dangling foreign symlink is preserved rather
  than misclassified as an absent path.
- **Embedded-terminal protection on macOS** - the foreign-window classifier now excludes Codex
  and Visual Studio Code by bundle identity in addition to Terminal, Ghostty, iTerm, Warp, and
  other terminal apps. Because AppKit permission-safe filtering happens at application scope,
  conservatively excluding the full editor prevents ride/collect behavior from ever targeting an
  integrated terminal panel.

## [0.3.2] - 2026-07-11

### Added
- **Published installer lifecycle smoke** - a manually dispatched, read-only GitHub Actions gate
  now exercises the immutable stable release on x64/ARM64 Linux, Intel/Apple Silicon macOS, and
  Windows. It installs twice, verifies managed integrations, forces and proves transactional
  rollback, and covers Global MSI upgrade, repair, downgrade refusal, ARM64 extraction, and
  uninstall behavior.

### Fixed
- **Real MSI license plus the Great Honk Accord** - the x64/ARM64 Global and Corporate MSI
  agreement page now displays the authoritative PolyForm Noncommercial License instead of WiX's
  lorem-ipsum placeholder, followed by one shared, long-form, explicitly non-binding ceremonial
  agreement between the installer operator and the Goose. Packaging tests prove the legal wording
  stays aligned with `LICENSE`, the appendix stays within its copy contract, and every MSI variant
  consumes the same simple RTF while the Inno installers remain unchanged.

## [0.3.1] - 2026-07-10

> The immutable `v0.3.0` tag failed in artifact production before any draft or public release was
> created. In accordance with the atomic-release contract, the corrected build ships as the next
> patch version rather than moving or rebuilding that tag.

### Added
- **Stable release and install receipts** - `honk300.release.v1` and `honk300.install.v1`
  record the exact version, tag, commit, target, artifact hash/size, managed layout, aliases, and
  autostart ownership. Windows recommends the x64/ARM64 Global MSI; macOS and Linux use one
  version-stamped, exact-tag, hash-verifying shell bootstrap. macOS installs a real universal
  `~/Applications/Honk300.app`; Linux installs below the user's XDG data directory.
- **Config schema v2 and explicit recovery states** - configuration loads now distinguish
  missing, valid, malformed, and unsupported-newer files. `setup --reset` is the only reset path
  and creates a timestamped backup first; v1 mute controls migrate without losing intent.
- **Distribution notices** - the official PolyForm Noncommercial license, third-party media
  notice, and vendored-code provenance now travel with release payloads.

### Changed
- **Concept-C side renderer** - the goose now uses a single integrated white silhouette with
  distinct back-neck/throat curves, a broad neck base, restrained long-neck sweep, and oval head.
  Both facings, tucked/raised poses, the existing rig, palette, wings, beak, and legs remain.
- **Long-running and multi-display runtime core** - simulation clocks/deadlines use `f64`, visual
  phases wrap independently, display geometry uses real monitor regions/adjacency instead of a
  bounding rectangle, and shared `RuntimeCore` pins command/reload/tick/damage ordering across
  native event pumps. Renderer/platform scratch storage is reused and frame damage is bounded to
  the current and previous visual bounds.
- **Simpler distribution defaults** - Windows is machine-wide MSI-first; macOS/Linux are
  terminal-first and no-sudo. The DMG remains an unadvertised v0.2.1 updater compatibility asset,
  and the ad-hoc-signed, unnotarized macOS limitations are documented honestly.

### Fixed
- **Non-destructive config and responsive control UI** - malformed/future config is never
  overwritten, atomic saves preserve valid symlink targets and stable floats, reload rejects
  restart-only backend changes, stopped runtimes report capabilities as unprobed, and TUI IPC no
  longer blocks drawing/input. Start saves dirty settings first and surfaces actual readiness
  failures while small/error-heavy terminal layouts remain reachable.
- **Behavior and render continuity** - crossfades use complementary alpha, puddle returns stay
  vertically continuous, manners cancel delayed pranks, note/meme toggles cancel independently,
  on-hour/Hyper honks deduplicate, capability decks refresh, and missing renderer goldens fail
  rather than being created silently.
- **Cross-platform backend reliability** - Windows monitor removal no longer quits the process,
  topology/DPI changes reconcile in place, and note typing is target-scoped UI Automation only.
  macOS corrects display-coordinate conversion, drag classification, topology handling, and audio
  recovery. X11 fails closed without ARGB/compositor/input shaping; native Wayland now has a real
  event pump, per-output layer surfaces, hotplug, integer/fractional scale, empty input regions,
  and a released-buffer pool capped at three buffers per output.
- **Transactional lifecycle and publication** - archive traversal/absolute/duplicate/link entries
  are rejected, foreign integrations are preserved, payload swaps and owned integrations roll
  back together, Windows hands privileged replacement to hidden post-exit installer helpers, and
  one draft-only release orchestration now publishes the complete immutable asset set at once.
- **Deterministic hosted X11 readiness** - the Linux visible-overlay smoke gate now waits for the
  compositing manager to own the same X11 selection required by the runtime before starting it,
  and prints the runtime log when readiness fails instead of hiding the actionable cause.
- **X11 input-shape protocol negotiation** - the runtime now negotiates XFixes region support and
  the Shape extension before its required pre-map click-through check. Xorg otherwise rejects the
  first region request with `BadRequest` even when both extensions are installed.
- **Cross-platform release producers** - GNU portable jobs install their ALSA build prerequisite,
  macOS verifies cargo-dist with the native checksum tool and explicitly installs both Rust
  targets, the flat Windows cargo-dist archive is extracted correctly, and WiX legal notices use
  permanent component GUIDs plus an explicit ARM64-compatible installer schema version accepted
  by both x64 and ARM64 MSI builds. A pre-tag candidate mode now builds and verifies the complete
  artifact set without executing publication steps or consuming an immutable tag; hosted MSI
  smoke also exercises an actual v0.2.1 upgrade, repair, downgrade refusal, and uninstall.

### Security
- **Hardened local boundaries and supply chain** - Windows named pipes are limited to the current
  user and SYSTEM, reject remote clients, and use bounded I/O; Unix IPC uses owner-only directories
  and sockets with peer-credential validation, including a short UID-scoped fallback when a macOS
  temporary path exceeds the Unix-socket limit. Elevated Windows lifecycle helpers and note
  pranks resolve only validated system executables rather than the caller's search path. `cargo
  audit` is required, and the compatible vendored `wayland-scanner` security backport carries upstream revision
  `d07c4f91f28b42e5a485823ffd9d8d5a210b1053`'s `quick-xml` fix.

### Removed
- **Obsolete source/config material** - the current `DESKTOP-GOOSE/` reference tree, duplicate
  `behavior.silence_sounds`, nonfunctional `stop_radius`, and old donation/developer material are
  no longer part of the active repository or product surface. Historical transcripts and
  superseded planning records remain untouched.

## [0.2.1] - 2026-07-08

### Changed
- **Side-view neck refinement (renderer polish, per live review)** - the side-profile goose's neck
  no longer bows backward toward the body, and no longer shows a seam where it meets the shoulders.
  The neck spine is now a near-straight, gently forward-leaning curve matching the reference art's
  back edge (`rig.rs` drops the backward bow at `neck_c1`), and its base is buried in the shoulder
  mass so the ribbon outline is covered by the body fill (`neck_base` lowered to reference y92,
  base ribbon width 30→32 in `render/side.rs`). Goldens re-blessed (four side frames plus the
  crossfade-band `top_down_diag`); the website's walk-cycle and pose frames were regenerated to
  match.

## [0.2.0] - 2026-07-08

### Added
- **Renderer V2: flat-illustration dual-view goose (R2, ADR 0014)** - the goose is now drawn in
  the flat-illustration style of the project's own reference art (`docs/art-reference/`): a
  dual-view rig with a mirrored side profile for shallow headings and a freely-rotating top-down
  view for steep ones (125 ms crossfade with 55°/45° hysteresis), stateful plant-and-swing feet
  (planted feet never slide; footmarks stamp at real plant events), a seamless S-curve neck
  ribbon, layered slate wing with scalloped feathers, two-tone beak and webbed-foot legs, eased
  neck posture, blink/breath/honk-tail-flick secondary motion, and 2x-supersampled per-view layer
  compositing. Goldens re-blessed and expanded to six frames; `examples/preview.rs` renders
  contact-sheet/zoom/walk strips for visual tuning.
- **Six-tone configurable palette** - `[colors]` gains optional `goose_shade`, `goose_wing`, and
  `goose_orange_dark` keys (derived coherently from the legacy three when absent, so old config
  files keep working); all six tones edit as R/G/B rows in the TUI, which materializes explicit
  keys on first edit. New defaults follow the reference art.
- **Idle-life behaviors (ADR 0016)** - wander paths meander (rng-driven lateral wobble that fades
  near targets); mud now only comes home from quick off-screen puddle hops (away 8-15 s, tracking
  mud 30-90 s afterward); every 4-7 minutes the goose takes a longer off-screen errand (away
  90-120 s, horizontal-edge preference) and 40% of errands chain a collect-window prank on
  return. All excursion behavior respects manners, never interrupts mischief-in-progress, and
  stays deterministic per seed.
- **`exit` / `quit` stop synonyms** - the goose-speak grammar now accepts `<name> exit` and
  `<name> quit` alongside `bad` / `no` / `no honk`.
- **TUI mouse rows** - `mouse.grab_distance` and `mouse.drop_distance` are now editable.
- **`behavior.attack_randomly` is wired** (was persisted but dead): when enabled, spontaneous
  cursor nabs join the roaming deck (and the mischievous-mood bias), matching the original's
  `AttackRandomly`. Default stays off — without it, nabs come only from clicks and `do nab`.
  New "Attack randomly" TUI toggle.
- **macOS packaging: universal2 `.app` + DMG (R3, ADR 0017)** - `x86_64-apple-darwin` and
  `aarch64-apple-darwin` join `[workspace.metadata.dist].targets`, so cargo-dist emits
  `honk300-<arch>-apple-darwin.tar.xz` (+ sha256) and a Darwin-capable shell installer. A
  hand-authored `.github/workflows/macos-packaging.yml` (chained on the Release workflow, like the
  Windows one) builds a `lipo`-fused universal2 `Honk300.app`, `hdiutil`-packs a compressed
  `honk300-universal2.dmg` (with an `/Applications` symlink), writes a sha256 sidecar, and uploads
  to the tag. `script/package_macos_app.sh` now stamps the real bundle version from
  `HONK300_VERSION` instead of `0.0.0`. Artifacts are **unsigned personal-use** (ad-hoc codesign
  only) — first launch needs a one-time right-click → Open. Supersedes ADR 0013's macOS deferral.
- **macOS `install` / `uninstall` / `update` (R3, ADR 0017)** - `honk300 install` stages
  `~/Applications/Honk300.app` (copying the bundle when run from one, else synthesizing the layout
  from a bare binary), symlinks `honk300`/`honk`/`goose` into `~/.local/bin`, writes a `mac-app`
  install marker, and with `--autostart` writes a `RunAtLoad` LaunchAgent
  (`~/Library/LaunchAgents/dev.emmetts.honk300.plist`). `uninstall` removes those and preserves
  user memes/notes (ADR 0015 §7 semantics); `--purge` also clears
  `~/Library/Application Support/honk300`. `update` replaces the `.app` from the verified DMG for
  bundle installs, and re-runs the cargo-dist shell installer for bare/symlink installs (Linux
  parity, no `cargo install`).

### Changed
- **Wandering no longer starts mud tracking** (previously a 50% roll at every waypoint) — mud is
  an occasional narrative event, not a constant state. `do mud` still forces it.
- **Default goose look** - the default palette moves from pure white/orange to the softer
  reference tones (off-white body, slate wing); the previous look remains reachable via the
  palette config.

### Fixed
- **Windows Per-Monitor-V2 DPI awareness (R1, ADR 0015)** - the process now declares
  PMv2 DPI awareness before creating any window (`init_dpi_awareness()`), and handles
  `WM_DPICHANGED`/`WM_DISPLAYCHANGE` by re-enumerating monitors, rebuilding the per-monitor
  overlay windows, refreshing world bounds, and forcing a repaint. Mixed-DPI overlays are no
  longer blurred or mispositioned, and display/DPI changes no longer need a restart.
- **Non-blocking collect-window + Notepad lifecycle (R1, ADR 0015)** - Notepad spawning is a
  polled state machine instead of a blocking up-to-3s wait, and typing is deferred until the
  spawned window is foreground (best-effort; a focus steal skips the note instead of disabling
  collect-window). The spawned Notepad child is tracked and closed (`WM_CLOSE`, then terminate)
  on close/stop, so `honk300 stop` leaves no notepad.exe zombie and IPC `status`/`do`/`reload`
  stay responsive mid-collect.
- **Unix stale singleton lock (R1, ADR 0015)** - the single-instance guard on macOS/Linux is now
  an advisory `flock` held for the process lifetime (released by the kernel on any death) instead
  of a marker file; a crash can no longer cause a false "already running" refusal. Adds unix-only
  `rustix` to honk-control.
- **macOS event pump + present safety (R1, ADR 0015)** - the overlay pump now drains the
  NSApplication event queue (collect-window close buttons work), and frame presents copy pixels
  into AppKit-owned `NSBitmapImageRep` storage instead of aliasing a reused buffer.
- **Linux degraded-overlay honesty (R1, ADR 0015)** - overlay-creation failure now fails start
  loudly (exit nonzero, clear message) unless `HONK300_ALLOW_HEADLESS=1` explicitly opts into the
  invisible headless mode; a new `overlay` capability rides the status protocol and shows in
  `honk300 status` and the TUI Status tab. X11 also stops re-creating the XFixes input region and
  re-setting the event mask every frame.
- **Non-purge uninstall preserves user content (R1, ADR 0015)** - plain `honk300 uninstall` now
  relocates user-supplied memes/notes to a timestamped `preserved-` folder and prints the
  location instead of deleting them; `--purge` keeps its backup-then-remove behavior.


## [0.1.0] - 2026-07-07

### Added
- **M19 lifecycle commands and update safety** — replaced the placeholder lifecycle commands with
  real `honk300 install [--autostart]`, `honk300 uninstall [--purge]`, and `honk300 update`
  behavior. `install` copies the current executable into the user install location, installs the
  `honk300`/`honk`/`goose` aliases, copies `Assets/` next to the binary, writes install-source
  markers, creates Windows shortcuts or Linux `.desktop` entries, and enables login autostart
  only when requested. `uninstall --purge` backs up user memes/notes before removing config/state.
  `update` reads install-source markers, chooses the matching arch-specific release installer,
  downloads and verifies the `.sha256` sidecar, runs the installer, and verifies post-install
  `--version`; tests pin that no update path uses `cargo install`.
- **M19 cargo-dist and Windows installer release artifacts** — added cargo-dist 0.31 metadata for the
  Windows x64/ARM64 and Linux x64/ARM GNU/musl matrix, a tag-triggered `Release` workflow, a
  chained `Windows Installers` workflow, WiX Global/Corporate MSI manifests, Inno Global/Corporate
  EXE manifests, fresh WiX/Inno GUIDs, per-arch artifact names, install-source marker files,
  optional default-off autostart, alias binaries, asset harvesting, and `.sha256` sidecar upload.
  Release run <https://github.com/RealEmmettS/goose/actions/runs/28842068256> produced the
  cargo-dist shell/PowerShell installers plus Windows and Linux archives; Windows installer run
  <https://github.com/RealEmmettS/goose/actions/runs/28842489497> attached x64/ARM64
  Global/Corporate MSI and EXE artifacts plus sidecars to
  the first release artifacts. macOS DMG/signing/notarization
  remain intentionally deferred behind ADR 0013 and `#m16r`; later macOS packaging defaults to
  unsigned personal-use artifacts unless signing credentials are intentionally added.
- **M16 macOS backend, status, and `.app` staging (implementation in-tree; Accessibility-granted
  smoke pending)** — added `crates/honk-platform-macos` with AppKit/CoreGraphics/ApplicationServices
  dependencies, macOS `start` runtime wiring through the existing Unix IPC transport, one
  AppKit overlay surface per display, CoreGraphics pointer polling/warp, local-time sampling,
  Accessibility-gated focused-window polling for foreign-window ride snapshots, AppKit-owned
  note/meme collect windows, macOS terminal-app classification tests, Accessibility-denied
  capability degradation, and dependency-free macOS audio through `/usr/bin/afplay`. The macOS
  target checks pass for `x86_64-apple-darwin` and `aarch64-apple-darwin`; hosted macOS CI run
  <https://github.com/RealEmmettS/goose/actions/runs/28569332035> now proves the universal2
  bundle/status/IPC gate and uploads `honk300-macos-macos-15` plus
  `honk300-macos-macos-15-intel`. Accessibility-granted cursor/window/collect smoke remains the
  M16.1 readiness blocker.
- **Runtime status protocol and TUI Status tab** — `honk-control` now supports `STATUS` and a
  compact `ControlResponse::Status` payload reporting running state, platform, bundle mode,
  Accessibility, cursor/window/collect/presence/audio capability states, and asset counts.
  `honk300 status` prints the same data, and the config TUI has a Status category plus refresh
  command. `honk-config::BackendState` now preserves supported/unsupported/denied/failed state
  while still collapsing to simple engine options.
- **macOS agent bundle staging** — added `script/package_macos_app.sh` to build x86_64 and arm64
  release slices, `lipo` them into `Honk300.app`, copy `Assets/`, write `Info.plist` with
  bundle id `dev.emmetts.honk300` and `LSUIElement=true`, ad-hoc sign, and validate with
  `plutil`, `codesign`, and `lipo`. Bundle-aware asset discovery now prefers
  `Contents/Resources/Assets`, and TUI Start launches bundled macOS runs through
  `/usr/bin/open -n <Honk300.app> --args start --config <path>`.
- **M17/M18 Linux visible backend and reduced Wayland runtime (CI proven)** — extended
  `crates/honk-platform-linux` beyond session/control plumbing with an X11 visible overlay
  using `x11rb` (`shape`, `xfixes`, `xinerama`, `randr`, `render`), XShape/XFixes input-region
  shaping, Xinerama/root display bounds, pointer sampling, cursor warp, and terminal-filtered
  foreign-window drag snapshots. Native Wayland now has a reduced layer-shell overlay using
  `smithay-client-toolkit` and `wayland-protocols-wlr`; it renders and remains IPC-controllable
  while cursor/window/collect/synthetic-input mischief reports unsupported. Linux collect-window
  support remains unsupported and is visible in status rather than silently attempted.
- **CI-proven M16.1-M18.1 readiness gates** — added `.github/workflows/ci.yml`,
  `script/smoke_m16_macos_accessibility.sh`, and stronger macOS/Linux smoke coverage. Hosted CI
  now covers Windows fmt/test/clippy/release plus Windows x64/ARM64 checks, macOS Intel/arm64
  universal2 bundle smoke with app artifacts, and Linux x64/ARM X11 + Wayland smoke under
  Xvfb/openbox/xcompmgr and headless sway. The X11 smoke verifies both internal frame pixels and
  actual root-window screenshot pixels. The optional self-hosted macOS Accessibility job is gated by
  `HONK300_RUN_A11Y_SMOKE=true` and labels `[self-hosted, macOS, ARM64, honk300-a11y]`.
  `docs/readiness/m16-m18-readiness.md` now records run
  <https://github.com/RealEmmettS/goose/actions/runs/28569332035>. `#m17r` and `#m18r` are Done
  from Linux x64/ARM hosted evidence; `#m16r` remains open because the optional self-hosted
  Accessibility smoke job was skipped.
- **Multi-monitor chase and appearance controls (milestone M15, complete)** — Windows now creates
  one layered overlay HWND per monitor, enumerates signed monitor bounds, chooses the engine world
  bounds from `[behaviors].multi_monitor_chase`, and clips/crops dirty render regions per monitor
  before calling `UpdateLayeredWindow`. With multi-monitor chase off, startup uses the primary
  monitor bounds; with it on, startup uses the full signed virtual desktop. Reloads hot-apply
  normal world options but report multi-monitor chase changes as restart-required.
- **M15 engine/config appearance contract** — `WorldOptions` now carries
  `multi_monitor_chase` and `AppearanceOptions { calm_goose }`. Calm Goose uses the existing Calm
  Suppression/manners path to suppress spontaneous honks, on-hour honks, autonomous
  cursor/window/collect mischief, and Autumn pile chase while leaving direct clicks and CLI/TUI
  pokes under their normal gates. `World::render_bounds(previous)` centralizes dirty-region
  coverage for the goose, previous frame, footmarks, hearts, sleepy particles, and Autumn piles.
- **M15 TUI recolor controls and ADR 0009** — the config TUI now makes Calm Goose live, marks
  multi-monitor chase as restart-required, and edits goose white/orange/outline through separate
  RGB channel rows so hue changes are possible without free-form text input. ADR 0009 records the
  accepted multi-monitor, dirty-render, Calm Goose, and original three-color palette scope.
- **Schedule manners and built-in Autumn (milestone M14, complete)** — added
  `honk-engine::schedule` with `ScheduleOptions`, `LocalMinute`, `PresenceSnapshot`, and
  `PresenceState`, plus `World::set_presence`, `World::manners_active`, and the schedule field on
  `WorldOptions`. Quiet hours are start-inclusive/end-exclusive, support overnight windows, and
  treat `start == end` as no quiet window. Quiet hours, Windows DND, and fullscreen use Calm
  Suppression: spontaneous honks, on-hour honks, autonomous cursor/window/collect mischief, and
  Autumn pile chase are suppressed while direct clicks and CLI/TUI pokes still pass through normal
  config/capability gates. Windows maps `SHQueryUserNotificationState` into platform-neutral
  presence snapshots and polls periodically, warning once and degrading to unsupported if the API
  fails.
- **Procedural Autumn leaf piles** — added platform-free `AutumnState`, piles, leaves, kicked-leaf
  physics, `AutumnLeafPileTask`, render-layer splitting, and Windows render ordering
  (footmarks → Autumn below-goose leaves → goose → Autumn above-goose leaves → hearts → sleepy
  particles). Autumn is active September 1 through November 30 by local runtime-injected date,
  uses recovered reference constants for pile timing/count/physics, and does not copy or load the
  original `Autumn.dll`. The Windows runtime adds `HONK300_SMOKE_LOCAL_DATE=YYYYMMDD` so Autumn can
  be visually smoke-tested outside the season.
- **M14 config and TUI plumbing** — existing version-1 TOML schedule fields now map into
  `WorldOptions.schedule`, `[safety].pause_on_fullscreen` controls fullscreen manners, and the TUI
  removes `(planned)` from live schedule/season rows while adding a separate fullscreen-respect row.
- **Dynamic moods and on-hour double honk (milestone M13, complete)** — added
  `honk-engine::mood` with `MoodKind::{Content,Hyper,Sad,Sleepy,Mischievous}`,
  `MoodIntensity::{Calm,Normal,Spicy}`, seeded weighted transitions, and platform-free
  `LocalTime` injection for schedule-like inputs. Mood effects post-modulate task output:
  sad/sleepy slow movement and lower neck posture, sleepy emits procedural Z particles, hyper
  can request the existing `HyperTask`, and mischievous duplicates only already-enabled
  nab/collect factories in the pickable list. The Windows runtime samples local time outside the
  engine and feeds `World::set_local_time`; the engine emits exactly two high honks at the top
  of a local hour, once per hour. `Sound::Honk` now carries `HonkTone::{Normal,High,Low}` and
  the audio backend maps tones to bundled honk clips while respecting audio toggles.
- **Config TUI and durable configuration (milestone M12, complete)** — added the `honk-config`
  crate for versioned TOML defaults, path resolution, validation, tolerant loading, conversion
  into runtime/world options, and atomic save with practical preservation of unknown keys. The
  default path is `%LOCALAPPDATA%\honk300\config.toml`, `~/Library/Application Support/honk300/config.toml`,
  or `$XDG_DATA_HOME` / `~/.local/share/honk300/config.toml`, with `--config <path>` override.
  Startup falls back to defaults on missing or rejected config and warns without corrupting the
  running state. Reload parses and validates before applying, then hot-applies current M0-M15
  settings for audio, mouse steal/tuning, perch-and-ride, collect-window kinds, pat behavior,
  timing, movement speed, mud/footmark timing, palette, mood intensity, on-hour honking, schedule,
  Autumn, and Calm Goose. Future settings for Wayland/backend and spicy behavior are persisted and
  shown as planned or restart-required.
- **Ratatui reducer UI (milestone M12, complete)** — added the `honk-config-tui` crate with
  reducer-owned state, pure render modules, categories for General, Behaviors, Mischief,
  Schedule, Appearance, Audio, Commands, and About, plus a Poke panel that sends M10 IPC commands.
  Terminal-window protection is shown as always on rather than configurable. Reducer tests cover
  navigation, toggles, numeric edits, dirty/save state, and poke command generation.
- **Shared control crate** — extracted the M10 protocol/client/server code from the binary into
  `honk-control`, reused by the root binary and TUI without changing the wire protocol or adding
  IPC concerns to `honk-engine`.
- **CLI grammar (milestone M11, complete)** — added deterministic pre-clap normalization for
  executable stems `honk300`, `honk`, and `goose`. The binary accepts default start, `start`,
  `plz`, `stop`, `reload`, `do <honk|wander|mud|meme|note|nab>`, `config`, `help`, `--help`,
  `--version`, `--config <path>`, and `--wayland`. `honk plz`, `goose plz`, and `honk300 plz`
  all start; `bad`, `no`, and `no honk` stop; pokes stay explicit through `do <action>`,
  including `do honk`. `install`, `uninstall`, `update`, and `setup` now parse for
  discoverability and M19 implements the lifecycle behavior.
- **CLI/TUI control plane (milestone M10, complete)** — the root binary is now split into
  `src/cli.rs`, `src/control/`, and `src/runtime/windows.rs`. `honk300` defaults to `start`;
  `honk300 start` refuses to create a second goose; and `honk300 stop`, `honk300 reload`, and
  `honk300 do <honk|wander|mud|meme|note|nab>` send finite local IPC commands to the running
  instance. Windows uses a per-user named mutex plus a per-user named pipe. Unix-family readiness
  uses the same protocol over a UID-scoped lock file and Unix domain socket shape for later macOS
  and Linux overlay backends. `honk-engine` gained `PokeAction`, `PokeOutcome`, `World::poke`,
  and `World::apply_options` so stop/reload/poke plumbing stays structured and platform-neutral.
  The protocol rejects malformed, unknown, and oversized payloads. ADR 0004 records the
  CLI/TUI-only control model: no system tray, no global quit key, and no non-IPC stop path.
- **Terminal-window protection** — Windows foreign-window discovery now classifies common terminal
  hosts and excludes them before the goose can ride, collect, move, focus, type into, drag, or
  otherwise manipulate them. The protection rule is documented as permanent and applies to future
  spicy/default-off behavior too; visual overlay over terminal windows remains allowed.
- **Collect-window dispatcher (milestone M9, complete)** — the goose can now drag in Notepad and
  meme windows on Windows. `honk-engine` gained a platform-neutral collect-window contract
  (`CollectWindowId`, `CollectWindowRequestId`, `CollectWindowKind::{Note,Meme}`,
  `CollectWindowCapabilities`, `CollectWindowOptions`, ordered `CollectWindowCommand`s, and
  `CollectWindowSnapshot`) plus `CollectWindowTask` and `World` drain/feed APIs. The task chooses
  note/meme content only when both content and backend capabilities exist, emits ordered spawn /
  move / focus / type / close commands, uses the rig beak tip for drag offset, suppresses
  overlapping pat/click/perch/cursor interrupts while active, leaves Notepad open after typing,
  and closes owned meme windows after a visible dwell. The Windows runtime loads assets from
  provenance-separated `Assets/` directories, spawns and tracks Notepad by PID/HWND, verifies
  foreground focus before Unicode `SendInput`, creates non-topmost owned image windows for memes,
  moves controlled windows with Win32 APIs, toggles pass-through while dragging, feeds snapshots
  back into the engine, and adds `HONK300_SMOKE_COLLECT=note|meme` for visual smoke before M10/M11
  public pokes.
- **M9 assets and ADR 0003** — screened original meme/note assets that pass provenance checks are
  copied 1:1 for personal-use builds, one complete custom in-house counterpart is added per copied
  original, and user-supplied `Meme8.png` is included as an approved meme prop. One original meme
  candidate with a baked-in social handle watermark is excluded rather than redacted. Donate is
  intentionally removed: old donate pages, Patreon links, social handles, and old-project branding
  do not ship. ADR 0003 records the collect-window command/snapshot boundary, asset provenance,
  no-donate decision, cross-platform degradation model, and target expectations.
- **Foreign-window perch & ride (milestone M8, complete)** — the goose now reacts when the
  user drags another application's window on Windows. `honk-engine` gained a platform-neutral
  foreign-window contract (`ForeignWindowId`, `ForeignWindowSnapshot`,
  `ForeignWindowCapabilities`, and `ForeignWindowOptions`) and a transient `PerchRideTask`
  that interrupts the current task, runs to the dragged window's ride anchor, pins to the
  moving anchor if it arrives before release, and resumes the interrupted task on release or
  capability loss. The Windows backend now watches move/size drags with an out-of-context
  `SetWinEventHook`, queues hook events only, polls live geometry via `GetWindowRect`, filters
  the app overlay and invalid/non-root/invisible/minimized windows, unhooks on drop, and exposes
  a temporary `--no-window-ride` opt-out until M12 config exists. `move_window` is reported as
  future capability data only; M8 does not autonomously move windows or start M9
  collect-window/notepad/meme behavior. Added ADR 0002 to pin the engine/backend
  contract and cross-platform guardrails.
- **Cursor mischief: warp + nab sub-states (milestone M7, complete)** — the goose can now steal
  the real cursor on Windows in a bounded, recoverable way. `honk-engine` remains platform-free:
  it owns `CursorCommand::WarpTo(Vec2)`, `MouseStealOptions`, `WorldOptions`, and the
  `NabMouseTask` state machine; platform backends drain cursor commands after each fixed-tick
  update. `TaskCtx` now carries the current platform-neutral pointer plus a cursor-command
  queue, so tasks can request cursor motion without importing Win32/CoreGraphics/X11/Wayland
  APIs. `NabMouseTask` is randomly pickable only when mouse stealing is enabled and the backend
  reports cursor-warp support. A click on the goose also starts `NabMouseTask` when supported;
  when mouse stealing is disabled or unsupported, the older M6 click-to-hyper burst remains the
  fallback. The nab lifecycle seeks the live pointer at charge speed, bites once when the beak
  reaches `grab_distance`, captures the beak-to-cursor offset, then runs a bounded HYPR-style
  retargeting burst while keeping the cursor anchored to the beak until `succ_time` or a
  pull-away/drop threshold ends the grab. While nab owns the cursor, M6 pat/click handling is
  suppressed so synthetic cursor movement does not spawn hearts or interrupt into `HyperTask`.
  The Windows backend now exposes a cursor-warp wrapper, applies only the newest warp command
  after ticking, warns once if warping fails, marks cursor warp unavailable on failure, and the
  binary adds `--no-mouse-steal` as an opt-out. M7 added regression coverage for disabled and
  unsupported paths, click-to-nab, fallback click-to-hyper, seek/grab/drag/drop/timeout, one bite
  sound, drag-offset preservation, HYPR-style retargeting, deterministic command draining, and
  M6 interaction suppression during nab. The full local gate and release build passed before M7
  was moved to Done.
- **M7.0/M7.1/M7.2 completion work** — M7 now includes the completed-milestone audit, the
  mandatory cross-platform `honk-engine` readiness pass, and the renderer/runtime architecture
  spike. The M7.0 audit rechecked M0-M6 against `honk300_plan.md`, fixed stale status docs, and
  created follow-up `#p4d` for fullscreen overlay present-cost measurement. The M7.1 readiness
  pass confirmed the engine stayed platform-free and that current target coverage still respects
  Windows x64/ARM64, macOS Intel/Apple Silicon, Linux x64/ARM GNU, and Linux x64/ARM musl
  expectations. The M7.2 spike selected a future custom CPU sprite/atlas renderer and split that
  implementation into backlog task `#r2v`.
- **Architecture decision records** — added `docs/adr/` with ADR 0001, recording the accepted M7
  cursor-mischief contract, Windows runtime behavior, cross-platform guardrails, renderer
  direction, consequences, verification, and follow-up tasks. `AGENTS.md` and `CLAUDE.md` now
  include ADR maintenance rules so future architecture changes update ADRs, task memory, docs,
  and both changelogs together.
- **Hit-testing: pat (hover-streak + hearts) + click→hyper (milestone M6)** — the goose
  reacts to the cursor. Two distinct interactions (plan §5.9 / §6), built on a new per-frame
  pointer feed (`World::set_pointer` taking a platform-free `interaction::Pointer`; the
  Windows backend polls `GetCursorPos` + `GetAsyncKeyState`). **Pat** = repeated cursor
  *hover-sweeps* over the goose (no buttons): a `PatTracker` accumulates hover-movement into
  a happy streak, each registered pat spawns a rising/fading **heart particle** (new
  `honk-engine::hearts` module + `render::render_hearts`, a clean-room procedural heart) and
  keeps the goose briefly **calm** (a content goose suppresses its spontaneous honks). **Click**
  = a left-press on the goose → a charge-speed **hyper** burst (`task::HyperTask`) that bolts
  around erratically and honks, installed as a transient interrupt that **saves and restores
  the task it suspended** (the resume mechanism perch-and-ride will reuse in M8). Hit-testing
  uses the rig bounding box (`Rect::contains`), naturally click-through everywhere else.
  Engine-side logic is fully unit-tested; the on-screen result was verified visually.
- **Audio (milestone M5)** — the goose honks. A `rodio` backend in the binary plays the
  bundled original sounds (Honk1–4, BITE, MudSquith, Pat1–3) mapped from platform-free
  `Sound` requests the engine emits (`honk-engine::sound::Sound` + a `World` queue drained
  each frame). The goose honks on wander-retarget and squelches while tracking mud.
  `--no-sound` / `--silent` mutes it (the original `SilenceSounds`); a missing audio device
  degrades to a silent no-op. Sounds are embedded via `include_bytes!` from `Assets/Sounds/`.
  Audio is Windows-scoped this round (the macOS/Linux backends wire it in M16/M17).
- **Task state machine + wander + FirstUX intro (milestone M4)** — the M2 roam stand-in is
  replaced by the real AI. A `Task` trait (the documented internal extension seam, plan §18 —
  no external mod ABI), a `TaskCtx`, a registry of randomly-pickable tasks chosen via the
  biased `Deck`, and a `World` task runner. Tasks set targets/params only; the engine
  auto-locomotes. Ships `WanderTask` (roam to random points for a verified 20–40 s dwell, with
  occasional mud-tracking folded in) and a scripted `FirstUxTask` (the goose walks on-stage
  from off the bottom edge and pauses to introduce itself for the verified 20 s
  `FirstWanderTime`, then hands off to roaming). Timings are the verified `config.ini` values
  (20 / 20 / 40); config-driven values arrive with the TOML loader in a later round.
- **Footmarks + mud trail (milestone M3)** — the goose leaves a trail of fading muddy
  footprints while it's "tracking mud," at the verified lifetimes (8.5 s life / 1 s
  shrink-out). To render world-space trails the overlay moved from the small per-goose
  window to a **fullscreen primary-monitor layered overlay** (the plan's intended
  per-monitor architecture; multi-monitor traversal is M15). The engine drops an
  alternating-foot print at each gait half-step while tracking mud; the M2 roam driver
  triggers mud-tracking periodically (M4's `Task_TrackMud` will formalize the trigger).
  Present is capped a touch lower (~40 Hz) since a fullscreen layered present is heavier;
  a dirty-rect optimization (`UpdateLayeredWindowIndirect` + `prcDirty`) is a future perf task.

### Improved
- **M12R config/TUI polish** — `[speeds]`, `[mud]`, `[colors]`, `[moods]`, and on-hour settings
  now validate and map into `WorldOptions` instead of staying write-only. Unknown top-level TOML
  keys and unknown section keys emit a one-shot load warning while still being preserved on save.
  The TUI now uses a row model with scroll support; surfaces movement, mud, color, mood,
  on-hour, and quiet-time rows; edits quiet start/end in 15-minute increments; cycles mood
  intensity through `calm -> normal -> spicy`; confirms dirty quits; routes command outcomes
  through reducer actions; and starts the goose with null stdio plus Windows detached flags.
- **Goose look reworked toward the real original — from direct observation and review.** The
  published modding API documents the rig *model* but not the `updateRig`/`Render` maths (closed
  binary; not decompiled, per the clean-room rule), so the goose was re-grounded by running the
  original Desktop Goose and capturing a local reference screenshot, then iterating against
  golden-frame previews and visual-smoke captures. A generated-sprite-style wing-panel/tall-neck
  pass was saved only as a local visual backup and rejected because it drifted from the original's
  charm. The accepted M7 renderer now uses a deliberate single Bezier body silhouette instead of
  stacked capsules, a flatter/thinner oval body closer to the original side-profile mass, the
  neck drawn under the body/head to hide construction seams, a small plain eye instead of a
  ringed cartoon eye, a short rounded orange beak, fuller tiny orange feet, a subtle dotted
  ground shadow, and updated golden frames for rest/reaching/mid-stride. This remains a
  clean-room procedural renderer; Renderer V2 owns the future atlas-based art pipeline.
- **Windows overlay + walking goose (milestones M1 + M2)** — `honk300` now renders the
  procedural goose on a transparent, always-on-top, click-through-where-transparent overlay
  and walks it around the desktop.
  - **Engine (platform-free, tested):** a fixed-120 Hz `Accumulator` (catch-up clamped to
    avoid the spiral of death); clean-room `locomotion` (accelerate toward `target_pos`,
    cap at the speed tier, face the travel direction, stop cleanly on arrival); a `World`
    with a minimal **roam driver** (a temporary stand-in for the M4 task/AI system); and a
    distance-driven **procedural-feet gait** with a subtle body bob.
  - **Windows backend (`honk-platform-windows`):** a layered popup window presented via
    `UpdateLayeredWindow` with premultiplied BGRA (softbuffer can't do per-pixel alpha on a
    Windows layered window). The small window is repositioned every frame, so it *is* the
    dirty rect — present cost stays proportional to the goose, not the screen. `WS_EX_LAYERED`
    **without** `WS_EX_TRANSPARENT` gives natural per-pixel-alpha click-through (opaque goose
    clickable, transparent margins fall through). This presenter shape was superseded by the
    M3 fullscreen primary-monitor overlay so mud/heart/world-space props can render in-place;
    the M7.0 audit tracks dirty-rect/per-monitor optimization as follow-up work.
  - **Renderer reworked to the original's technique:** capsule body / neck / two-segment
    head, an orange beak and webbed feet, a grey outline, and a ground shadow — tuned to
    resemble the real side-profile goose, animated by the neck-lerp + gait + body bob.
  - **Root `honk300` binary:** the three-clock loop (sim 120 Hz, present ~60 Hz on the
    goose's bounding box). Golden frames re-blessed (rest / reaching / mid-stride).
  - **Design note (deviation from plan §4):** the overlay uses raw Win32 (the `windows`
    crate) rather than winit — a small moving layered window via `UpdateLayeredWindow` is the
    canonical low-CPU desktop-pet pattern, and per-backend windowing fits the capability-trait
    design. winit can be revisited at M15 (multi-monitor) / M16 (cross-platform loop). The
    workspace root is now also the `honk300` binary package; added the `honk-platform-windows`
    crate.
- **Cargo workspace + `honk-engine` crate (milestone M0)** — the platform-free
  simulation core: `#![forbid(unsafe_code)]`, no windowing/OS/audio/input dependency,
  fully headless-testable. Ported 1:1 from the verified modding-API source
  (`…/GooseModdingAPI/{Exports.cs, SamEngine.cs}`): `Vec2` + `SamMath`; the 120 Hz
  fixed-timestep constants (`DT = 1/120`); the **faithful biased** `Deck` shuffle-bag
  (decision C8 — a seedable SplitMix64 drives it for deterministic tests; RNG internals
  are clean-room); `GooseEntity` + `ParametersTable` at the verified values (Walk/Run/
  Charge 80/200/400, accel 1300/2300, step 0.2/0.1, mud 15); the rig geometry constants
  with a clean-room `update_rig`; `ProceduralFeet`; the 64-slot `FootMarks` ring buffer
  (lifetime 8.5 s / shrink 1 s); and a clean-room tiny-skia renderer (`Rig → Pixmap`
  with a dirty-rect bounding box). Pinned by 26 unit tests (constants, rig vertices, the
  exact `Deck` sequence + its documented bias, footmark lifetimes) and 3 committed
  golden-frame PNGs. The renderer's proportions are a first clean-room approximation —
  the goldens are a regression baseline, not a fidelity reference (visual tuning is M1+).
- **Workspace scaffold** — root `Cargo.toml` (workspace, edition 2021 / Rust 1.95 via
  `[workspace.package]`, `[profile.dist]`), `rust-toolchain.toml` pinned to 1.95, and
  `crates/honk-engine/Cargo.toml`. The `[workspace.metadata.dist]` / WiX / CI blocks are
  intentionally deferred to the M19 packaging round. Local gate is green
  (`fmt --check`, `clippy -D warnings`, `test --workspace`, `build --release`).
- `honk300_plan.md` — **the canonical, authoritative plan.** A claim-tested *hybrid* that
  synthesizes `claude_plan.md` (structural spine) and `codex_plan.md` (grafts), then folds in an
  approved round of new scope. Each draft's load-bearing claims were verified against ground
  truth: engine constants checked against `…/GooseModdingAPI/Exports.cs` (claude exact; codex's
  Appendix-B speeds wrong), the biased `Deck` against `SamEngine.cs`, and the QubeTX conventions
  (editions, the 6 base targets, `cargo-dist 0.31.0`, ICE flags) across TR300/ND300/WB300. Adds:
  the new autonomous behaviors, a ratatui `<name> config` TUI, a three-name goose-speak CLI, and
  a full all-OS/all-arch build matrix. Build milestones now **M0–M19**.
- `claude_plan.md` — comprehensive, adversarially-reviewed plan for **honk300**, a
  cross-platform (Windows/macOS/Linux) Rust reimplementation of Desktop Goose. Derived
  from analysis of `DESKTOP-GOOSE/` (the original v0.31 Windows + v0.22 macOS builds) and
  the `*300` sibling repos (TR300/ND300/WB300). Captures the reverse-engineered engine
  (rig geometry + physics constants, 120 Hz fixed tick, the biased `Deck` shuffle-bag, the
  Task/`InjectionPoints` model from `…/GooseModdingAPI/{SamEngine,Exports}.cs`), a
  Cargo-workspace architecture (`honk-engine` + capability-trait platform backends), build
  milestones M0–M17, the packaging pipeline (cargo-dist + hand-authored
  `windows-installers.yml`), a per-platform capability matrix, and a ranked risk table.
- `codex_plan.md` — a parallel planning document produced by Codex.
- `CHANGELOG.md` / `HUMAN_CHANGELOG.md` — dual changelogs, mirroring the `*300` family
  convention.
- `CLAUDE.md` — repository guidance for future Claude Code sessions.

### Changed
- M15 is now Done, M16 implementation has moved into the active backend-readiness track, and
  Renderer V2 remains tracked separately as backlog task
  `#r2v`. The task records now preserve M7's audit/readiness/renderer work, M8's foreign-window
  readiness pass, M9's collect-window asset/ADR/target-readiness work, and M10's IPC/control
  readiness work, plus M11 CLI grammar, M12 config/TUI readiness work, M13 moods/hourly-honk
  closure, M14 schedule/Autumn closure, and M15 multi-monitor/appearance closure.
- `README.md`, `AGENTS.md`, and `CLAUDE.md` were updated to reflect M0-M19 implementation in
  tree, the M16.1 macOS Accessibility evidence gate, Linux X11/Wayland presentation support,
  M19 release evidence, and the ADR
  0001/0002/0003/0004/0007/0008/0009/0010/0011/0012/0013 location and maintenance rules.
- Added **ADR 0005** (M11 three-name CLI, goose-speak, and the poke-outcome round-trip) and
  **ADR 0006** (M12 config TUI, durable TOML, and the capability/preference boundary), recording
  the previously-undocumented M11/M12 decisions and the four contract corrections from the
  adversarial review.
- Added **ADR 0007** (M13 dynamic moods and local-time injection), recording the platform-free
  mood state machine, honk-tone contract, and runtime-owned local-clock sampling boundary.
- Added **ADR 0008** (M14 schedule, presence, and Autumn), recording Calm Suppression, the
  schedule/presence engine boundary, Windows presence polling, and the built-in Autumn constants.
- Added **ADR 0009** (M15 multi-monitor and appearance), recording the per-monitor Windows overlay
  boundary, dirty-region presentation, Calm Goose valve, restart-required multi-monitor reload
  rule, and original three-color recolor scope.
- Added **ADR 0011** (M17/M18 Linux control runtime and degraded Wayland foundation), recording
  the X11-first session rule, forced/native Wayland degradation, Linux Unix IPC runtime, terminal
  classifier, command-player audio, and the remaining Linux-host readiness gates that existed
  before the visible backends landed.
- Added **ADR 0012** (M16.1-M18.1 CI-proven backend readiness), recording the hosted CI matrix,
  optional self-hosted macOS Accessibility gate, Linux X11/Wayland smoke contract, and the rule
  that readiness tasks do not move to Done until CI evidence is recorded.
- `claude_plan.md` and `codex_plan.md` are now **superseded reference drafts**; `honk300_plan.md`
  is canonical. The "Read these first" pointers in **both** `CLAUDE.md` and its Codex twin
  `AGENTS.md` were updated in lockstep (canonical plan, milestone range M0–M19, workspace
  cross-reference → `honk300_plan.md` §7).
- `README.md` gained a **"Status — the decided plan"** section recording `honk300_plan.md` as
  canonical and summarizing the decided direction (three-name goose-speak CLI, ratatui config
  TUI, new autonomous behaviors, no external mods / no tray, all-OS/all-arch builds).

### Fixed
- **Control responses now report the real outcome (M11 round-trip).** `honk300 do <action>` and
  `reload` previously always answered `OK` because the server thread responded at command-enqueue
  time, before the simulation ran. The transport now completes a request/response round-trip:
  `honk-control` gained `ControlRequest`, a bounded (2 s) wait for the sim's answer, and a
  `PokeOutcome`→`ControlResponse` mapping (`Busy` → `ERR BUSY`, `Unsupported` → `ERR UNSUPPORTED`,
  reload failure → `ERR RELOAD_REJECTED`, timeout → `ERR TIMEOUT`). The CLI/TUI "rejected: {code}"
  paths now actually fire. (ADR 0005.)
- **Cursor-warp capability is no longer seeded from the mouse-steal preference (M12 reload).** The
  Windows runtime initialized `cursor_warp_supported` from `!no_mouse_steal`, latching warp off so
  a config edit that re-enabled mouse steal never took effect until restart. It is now a pure
  platform capability (`true` on Windows, via `initial_cursor_warp_supported`) that degrades only
  on a real warp failure; the preference is applied solely through `MouseStealOptions::enabled`.
  (ADR 0006.)
- **Collect-window capability loss now survives reload (M12).** A backend collect-window failure
  was recorded only in engine state and was overwritten by the next reload, so the goose kept
  retrying a dead capability. `BackendState` gained `collect_window_supported`, threaded through
  `Config::effective_options`, so the loss is durable across reloads. (ADR 0006.)
- **Disabling the pat streak no longer disables clicking (M12 interaction).** `interaction.pat_streak`
  gated the click reaction as well as pats. It now scopes to the hover-pat hearts/calm only;
  clicking the goose still triggers a hyper burst (or a cursor nab when mouse steal is supported).
  (ADR 0006.)

### Decided
- **Renderer V2 direction:** use a custom CPU sprite/atlas blitter that outputs premultiplied
  pixels for each platform backend. Keep `tiny-skia`/`resvg` for vector effects or
  asset-rasterization helpers, but do not make Vello/wgpu, Skia, Bevy, Macroquad, or ggez the
  main runtime renderer for the desktop-pet overlay. Future atlas metadata should include stable
  anchors, beak/cursor attach points, hit masks, frame bounds, and animation tags.
- **Three invocation names** (`honk300` / `honk` / `goose`) with a finite, deterministic
  "goose-speak" grammar (e.g. `goose plz` to start, `honk bad` / `goose no honk` to stop,
  `goose do honk` to poke, `<name> config`, `<name> help`) — a fixed phrase map, **not** runtime
  NL parsing.
- **TOML config** (`config.toml`) replacing the original `.ini`, original keys preserved at the
  verified values, versioned + tolerant loader.
- **No external mod system** (no DLL/WASM/data mods). Autumn becomes a **built-in** season/task;
  extensibility is via documented internal seams (`ARCHITECTURE.md` + rustdoc).
- **No system tray and no global quit key.** Start, stop, reload, pokes, and future configuration
  are CLI/TUI-only over the **single-instance + IPC command channel** (`start` / `stop` / `do` /
  `reload`) that is also the Wayland-safe control path and the TUI's hot-apply transport.
- **Terminal windows are protected.** The goose may visually overlay terminals, but terminal
  windows are never valid ride, collect, movement, focus, typing, drag, or spicy-behavior targets.
- A **ratatui** config TUI at `<name> config` (QubeTX-family architecture: reducer + crossterm +
  `tokio::select!`) toggling every behavior incl. Autumn; **hot-apply where cheap, restart-note
  otherwise**.
- **New autonomous behaviors** (each a toggle, scoped to parameter-modulation of the procedural
  rig — no new art): dynamic moods, seasonal moods, multi-monitor chase, on-the-hour double honk,
  perch-&-ride windows, hover-sweep pat streak + hearts, quiet-hours/DND-fullscreen respect, a
  Calm-goose valve, and manual poke commands. Default = full prank, always-on.
- **Build for every advertised OS and architecture:** Windows x64 **and ARM64**, macOS Intel
  **and Apple Silicon** (universal2 `.app`/`.dmg`), Linux x64 **and ARM** (gnu + musl) — arch is a
  build/packaging axis, capability is an OS/display-server axis (`Cap<T>`).
- App name **honk300** (binary `honk300`, optional `honk` alias); fresh permanent WiX/Inno
  GUIDs (never reuse the sibling repos').
- Clean-room **procedural** goose renderer — no sprite extraction. Original sound effects,
  screened original memes, and screened original notes are bundled 1:1 for personal-use builds;
  every copied meme/note original gets one complete custom in-house counterpart. Old donate pages
  and old developer references do not ship.
- Linux: **X11-first** (runs under XWayland on Wayland sessions); native Wayland behind an
  opt-in `--wayland` flag with reduced mischief.
- Distribution: Windows-first installer matrix (Global/Corporate × MSI/EXE) + shell/
  PowerShell installers + macOS `.app`/`.dmg` + Linux `.desktop`. **No crates.io** —
  `crates-publish.yml` intentionally dropped from the family pipeline.

### Notes
- A personal-use `v0.1.0` GitHub release now exists with Windows/Linux archives, shell/
  PowerShell installers, Windows x64/ARM64 MSI/EXE installers, and checksum sidecars.
  `DESKTOP-GOOSE/` remains the reference copy of the original app and contains third-party
  copyrighted assets; do not redistribute those bundled assets publicly.
