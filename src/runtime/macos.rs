use crate::assets;
use crate::audio;
use crate::runtime::control_surface;
use crate::runtime::core::RuntimeCore;
use crate::runtime::macos_accessibility::{
    safe_anchor, transition, AccessibilityOnboarding, PermissionTransition,
};
use crate::runtime::{audio_probe_capability, RuntimeOptions};
use honk_config::{BackendCapability, BackendState, Config, EffectiveOptions};
use honk_control::{
    BundleStatus, CapabilityStatus, CommandServer, ControlCommand, ControlResponse, PlatformStatus,
    RuntimeStatus,
};
use honk_engine::render::DamageCanvas;
use honk_engine::render::{
    render_autumn_leaves, render_footmarks_with_timing, render_hearts,
    render_pose_with_palette_at_scale, render_sleepies, AutumnRenderLayer,
};
use honk_engine::{
    CollectWindowCommand, CollectWindowPayload, CursorCommand, DesktopLayout, Pointer,
    PresenceSnapshot, Rect, Sound, Vec2, World,
};
use honk_platform_macos::{
    accessibility_state, local_time, main_bundle_release_metadata, open_accessibility_settings,
    open_update_helper, request_accessibility_prompt, warp_cursor, AccessibilityState,
    CollectWindowController, ForeignWindowWatcher, Overlay, PresenceObserver,
};

pub fn run(
    options: RuntimeOptions,
    server: &CommandServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = options.config.clone();
    let assets = assets::AssetCatalog::load();
    println!("honk300: loaded {}", assets.summary());

    let mut overlay = Overlay::new()?;
    let primary_bounds = overlay.primary_monitor_bounds();

    let accessibility = accessibility_state();
    let mut onboarding = detect_accessibility_onboarding();
    let mut accessibility = run_accessibility_onboarding(
        &mut onboarding,
        accessibility,
        request_accessibility_prompt,
        open_accessibility_settings,
    );

    let mut cursor_warp = accessibility_capability_from_state(accessibility);
    let mut window_watch = accessibility_capability_from_state(accessibility);
    let mut collect_window = BackendCapability::Supported;
    let mut presence_observer = PresenceObserver::new().ok();
    let mut presence = BackendCapability::Unsupported;
    let mut audio_capability = BackendCapability::Supported;

    let mut effective = effective_options(
        &config,
        &options,
        backend_state(
            cursor_warp,
            window_watch,
            collect_window,
            presence,
            audio_capability,
            assets.note_count(),
            assets.meme_count(),
        ),
    );
    let mut audio = if effective.no_sound {
        None
    } else {
        audio::Audio::new()
    };
    if !effective.no_sound {
        audio_capability = audio_probe_capability(audio.is_some());
    }

    let mut warned_window_ride = false;
    let (mut window_watcher, initial_window_watch) = attempt_window_watcher(
        window_watcher_requested(effective.world.foreign_window),
        window_watch,
        &mut warned_window_ride,
        "honk300: macOS window ride unavailable; disabling it",
        || ForeignWindowWatcher::new(&overlay),
    );
    window_watch = initial_window_watch;
    effective = effective_options(
        &config,
        &options,
        backend_state(
            cursor_warp,
            window_watch,
            collect_window,
            presence,
            audio_capability,
            assets.note_count(),
            assets.meme_count(),
        ),
    );

    let layout = desktop_layout_for(
        effective.world.multi_monitor_chase,
        primary_bounds,
        overlay.monitor_bounds(),
    )?;
    let mut world = World::with_layout_and_options(layout, seed_from_clock(), effective.world);
    if onboarding.waiting_for(accessibility) {
        world.enter_permission_wait(safe_anchor(primary_bounds));
        println!(
            "honk300: waiting calmly for macOS Accessibility permission; status, reload, honk, and stop remain available."
        );
    }
    let mut collect_controller =
        CollectWindowController::new(primary_bounds, overlay.virtual_desktop_bounds());
    let mut core = RuntimeCore::new();
    let mut damage_canvas = DamageCanvas::default();
    const AUDIO_RETRY_INTERVAL: f64 = 5.0;
    const ACCESSIBILITY_POLL_INTERVAL: f64 = 1.0;
    let mut next_audio_probe = 0.0;
    let mut next_accessibility_probe = ACCESSIBILITY_POLL_INTERVAL;
    let mut warned_cursor_warp = false;
    let mut warned_collect_window = false;

    println!(
        "honk300: a macOS goose is loose. Use the Honk menu or `honk300 stop` to send it home."
    );

    let trace_presence = std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("HONK300_TRACE_PRESENCE").as_deref() == Ok("1");
    let mut last_presence_trace = None;
    let mut presence_trace_count = 0;
    loop {
        if !overlay.pump() {
            break;
        }
        let (presence_snapshot, presence_status) = presence_observer
            .as_mut()
            .map(PresenceObserver::poll)
            .unwrap_or((
                PresenceSnapshot::unsupported(),
                honk_control::PresenceStatus {
                    fullscreen: CapabilityStatus::Failed,
                    dnd: CapabilityStatus::Unsupported,
                },
            ));
        presence = super::backend_capability(presence_status.aggregate());
        world.set_presence(presence_snapshot);
        if trace_presence && presence_trace_count < 64 {
            let state = (
                presence_status.fullscreen,
                presence_snapshot.state == honk_engine::PresenceState::Fullscreen,
                world.manners_active(),
            );
            if last_presence_trace != Some(state) {
                last_presence_trace = Some(state);
                presence_trace_count += 1;
                eprintln!(
                    "honk300 macos presence trace: {}",
                    serde_json::json!({
                        "fullscreen_status": state.0.label(), "fullscreen": state.1, "manners": state.2,
                    })
                );
            }
        }
        world.set_collect_window_capacity(collect_controller.has_capacity());

        if let Some(command) = overlay.take_status_menu_command() {
            let action = control_surface::command_name(command);
            if let Err(err) = control_surface::handle_command(
                command,
                &mut world,
                || control_surface::open_configuration(&options.config_path),
                open_update_helper,
            ) {
                eprintln!("honk300: {action} action could not start ({err})");
            }
        }

        if overlay.take_topology_changed() {
            let primary = overlay.primary_monitor_bounds();
            let layout = desktop_layout_for(
                effective.world.multi_monitor_chase,
                primary,
                overlay.monitor_bounds(),
            )?;
            world.apply_layout(layout);
            if world.permission_waiting() {
                world.update_permission_wait_anchor(safe_anchor(primary));
            }
            collect_controller.update_display_bounds(
                primary,
                primary,
                overlay.virtual_desktop_bounds(),
            );
        }

        let frame = core.begin_frame();

        while let Some(request) = server.try_recv() {
            match request.command() {
                ControlCommand::Stop => {
                    println!("honk300: stop command received.");
                    request.respond(ControlResponse::Ok);
                    RuntimeCore::begin_graceful_stop(&mut world);
                }
                ControlCommand::ForceStop => {
                    println!("honk300: forced stop command received; stopping immediately.");
                    request.respond(ControlResponse::Ok);
                    return Ok(());
                }
                ControlCommand::Reload | ControlCommand::ReloadIf(_) => {
                    let response = match RuntimeCore::load_reload_config(
                        request.command(),
                        &options.config_path,
                    ) {
                        Ok(next_config)
                            if RuntimeCore::restart_required_reason(&config, &next_config)
                                .is_some() =>
                        {
                            let reason =
                                RuntimeCore::restart_required_reason(&config, &next_config)
                                    .expect("guard established restart-required changes");
                            eprintln!("honk300: reload rejected; restart required for {reason}");
                            ControlResponse::Err("RESTART_REQUIRED".into())
                        }
                        Ok(next_config) => {
                            config = next_config;
                            cursor_warp = refresh_accessibility_capability(cursor_warp);
                            window_watch = refresh_accessibility_capability(window_watch);
                            collect_window = refresh_supported_capability(collect_window);
                            effective = effective_options(
                                &config,
                                &options,
                                backend_state(
                                    cursor_warp,
                                    window_watch,
                                    collect_window,
                                    presence,
                                    audio_capability,
                                    assets.note_count(),
                                    assets.meme_count(),
                                ),
                            );
                            if !window_watcher_requested(effective.world.foreign_window) {
                                window_watcher = None;
                            } else if window_watcher.is_none() {
                                match ForeignWindowWatcher::new(&overlay) {
                                    Ok(watcher) => window_watcher = Some(watcher),
                                    Err(err) => {
                                        window_watch = permission_or_failed(&err);
                                        if !warned_window_ride {
                                            warned_window_ride = true;
                                            eprintln!(
                                                "honk300: macOS window ride unavailable after reload ({err})"
                                            );
                                        }
                                    }
                                }
                            }
                            if effective.no_sound {
                                audio = None;
                            } else if audio.is_none() {
                                audio = audio::Audio::new();
                                audio_capability = audio_probe_capability(audio.is_some());
                            }
                            world.apply_options(effective.world);
                            ControlResponse::Ok
                        }
                        Err(err) => {
                            eprintln!("honk300: reload rejected; keeping prior config ({err})");
                            ControlResponse::Err("RELOAD_REJECTED".into())
                        }
                    };
                    request.respond(response);
                }
                ControlCommand::Do(action) => {
                    let outcome = world.poke(action);
                    println!("honk300: do {action:?} -> {outcome:?}");
                    request.respond(outcome.into());
                }
                ControlCommand::RemoteDo(_)
                | ControlCommand::WalkTo(..)
                | ControlCommand::LeafPlace(..)
                | ControlCommand::LeafMove(..)
                | ControlCommand::Snapshot
                | ControlCommand::Leaves => {
                    let response = RuntimeCore::remote_response(&mut world, request.command());
                    request.respond(response);
                }
                ControlCommand::Status => {
                    request.respond(ControlResponse::Status(runtime_status(
                        cursor_warp,
                        window_watch,
                        collect_window,
                        presence,
                        audio_capability,
                        assets.note_count(),
                        assets.meme_count(),
                    )));
                }
                ControlCommand::PresenceStatus => {
                    request.respond(ControlResponse::Presence(presence_status));
                }
                ControlCommand::Session
                | ControlCommand::WaylandStatus
                | ControlCommand::KwinEnable
                | ControlCommand::KwinDisable
                | ControlCommand::SwayStatus
                | ControlCommand::SwayEnable
                | ControlCommand::SwayDisable
                | ControlCommand::GnomeStatus
                | ControlCommand::GnomeEnable
                | ControlCommand::GnomeDisable
                | ControlCommand::HyprlandStatus
                | ControlCommand::HyprlandEnable
                | ControlCommand::HyprlandDisable
                | ControlCommand::PointerRequest
                | ControlCommand::PointerCancel => {
                    request.respond(ControlResponse::Err("UNSUPPORTED".into()))
                }
            }
        }

        world.set_local_time(local_time());

        let now = frame.now();
        if onboarding.managed() && now >= next_accessibility_probe {
            let current_accessibility = accessibility_state();
            match transition(accessibility, current_accessibility, true) {
                PermissionTransition::ResumeFirstUx => {
                    cursor_warp = BackendCapability::Supported;
                    window_watch = BackendCapability::Supported;
                    effective = effective_options(
                        &config,
                        &options,
                        backend_state(
                            cursor_warp,
                            window_watch,
                            collect_window,
                            presence,
                            audio_capability,
                            assets.note_count(),
                            assets.meme_count(),
                        ),
                    );
                    let watcher_requested =
                        window_watcher_requested(effective.world.foreign_window);
                    let (next_watcher, next_window_watch) = complete_accessibility_grant(
                        &mut world,
                        watcher_requested,
                        &mut warned_window_ride,
                        || ForeignWindowWatcher::new(&overlay),
                        |world, capability| {
                            effective = effective_options(
                                &config,
                                &options,
                                backend_state(
                                    cursor_warp,
                                    capability,
                                    collect_window,
                                    presence,
                                    audio_capability,
                                    assets.note_count(),
                                    assets.meme_count(),
                                ),
                            );
                            world.apply_options(effective.world);
                        },
                    );
                    window_watcher = next_watcher;
                    window_watch = next_window_watch;
                    println!(
                        "honk300: macOS Accessibility granted; resuming the FirstUX introduction."
                    );
                }
                PermissionTransition::EnterWait => {
                    window_watcher = None;
                    cursor_warp = BackendCapability::Denied;
                    window_watch = BackendCapability::Denied;
                    effective = effective_options(
                        &config,
                        &options,
                        backend_state(
                            cursor_warp,
                            window_watch,
                            collect_window,
                            presence,
                            audio_capability,
                            assets.note_count(),
                            assets.meme_count(),
                        ),
                    );
                    world.apply_options(effective.world);
                    world.enter_permission_wait(safe_anchor(overlay.primary_monitor_bounds()));
                    println!(
                        "honk300: macOS Accessibility was revoked; returning to the calm permission wait."
                    );
                }
                PermissionTransition::Stable => {}
            }
            accessibility = current_accessibility;
            next_accessibility_probe = now + ACCESSIBILITY_POLL_INTERVAL;
        }

        let (mx, my, left_down) = overlay.pointer_state();
        let pointer = Vec2::new(mx, my);
        world.set_pointer(Pointer {
            pos: pointer,
            present: true,
            left_down,
        });
        overlay.set_interactive(world.goose_hit(pointer));

        let mut disable_window_watcher = false;
        let dragged_window = match window_watcher.as_mut() {
            Some(watcher) => match watcher.active_drag() {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    disable_window_watcher = true;
                    window_watch = permission_or_failed(&err);
                    if !warned_window_ride {
                        warned_window_ride = true;
                        eprintln!("honk300: macOS window ride polling failed ({err})");
                    }
                    None
                }
            },
            None => None,
        };
        if disable_window_watcher {
            window_watcher = None;
            world.set_foreign_window_watch_supported(false);
        }
        world.set_foreign_window_drag(dragged_window);
        world.set_collect_window_snapshot(collect_controller.snapshot());
        world.set_collect_window_capacity(collect_controller.has_capacity());

        core.tick(&mut world, frame);

        let collect_display = world
            .layout()
            .region_at(world.goose.position)
            .and_then(|index| world.layout().regions().get(index).copied())
            .unwrap_or(primary_bounds);
        collect_controller.update_display_bounds(
            collect_display,
            primary_bounds,
            overlay.virtual_desktop_bounds(),
        );

        for command in world.take_collect_window_commands() {
            let result = match command {
                CollectWindowCommand::Spawn { request, payload } => match payload {
                    CollectWindowPayload::Note { .. } => {
                        collect_controller.spawn_note(request).map(|_| ())
                    }
                    CollectWindowPayload::Meme { index } => {
                        if let Some(meme) = assets.meme(index) {
                            collect_controller
                                .spawn_image(request, &meme.title, &meme.pixmap)
                                .map(|_| ())
                        } else {
                            Ok(())
                        }
                    }
                },
                CollectWindowCommand::Move { id, top_left } => {
                    collect_controller.move_window(id, top_left)
                }
                CollectWindowCommand::SetPassthrough { id, passthrough } => {
                    collect_controller.set_passthrough(id, passthrough)
                }
                CollectWindowCommand::Focus { id } => collect_controller.focus(id),
                CollectWindowCommand::TypeNote { id, note_index } => {
                    if let Some(text) = assets.note_text(note_index) {
                        collect_controller.type_text(id, text)
                    } else {
                        Ok(())
                    }
                }
                CollectWindowCommand::Close { id } => {
                    collect_controller.close(id);
                    Ok(())
                }
            };
            if let Err(err) = result {
                collect_window = permission_or_failed(&err);
                world.set_collect_window_supported(false);
                if !warned_collect_window {
                    warned_collect_window = true;
                    eprintln!("honk300: macOS collect-window unavailable; disabling it ({err})");
                }
            }
        }

        if let Some(CursorCommand::WarpTo(pos)) = world.take_cursor_commands().last().copied() {
            if let Err(err) = warp_cursor(pos) {
                cursor_warp = permission_or_failed(&err);
                world.set_cursor_warp_supported(false);
                if !warned_cursor_warp {
                    warned_cursor_warp = true;
                    eprintln!("honk300: macOS cursor warp unavailable; disabling it ({err})");
                }
            }
        }

        if let Some(audio) = audio.as_mut() {
            audio.poll();
        }
        if !effective.no_sound && audio.is_none() && now >= next_audio_probe {
            audio = audio::Audio::new();
            audio_capability = audio_probe_capability(audio.is_some());
            next_audio_probe = now + AUDIO_RETRY_INTERVAL;
        }

        let sounds = world.take_sounds();
        let mut audio_failed = false;
        if let Some(a) = audio.as_mut() {
            for s in sounds {
                if sound_enabled(effective.audio, s) && a.play(s) == audio::PlayOutcome::Failed {
                    audio_failed = true;
                    break;
                }
            }
        }
        if audio_failed {
            audio = None;
            audio_capability = BackendCapability::Failed;
            next_audio_probe = now + AUDIO_RETRY_INTERVAL;
        }

        if let Some(dirty) = core.damage(&world, frame) {
            let width = dirty.width().ceil().max(1.0) as u32;
            let height = dirty.height().ceil().max(1.0) as u32;
            let origin = dirty.min;
            let canvas = damage_canvas.prepare(width, height)?;
            render_footmarks_with_timing(
                canvas,
                &world.goose.foot_marks,
                world.now(),
                origin,
                world.footmark_timing(),
            );
            render_autumn_leaves(
                canvas,
                world.autumn(),
                world.now(),
                origin,
                world.goose.position,
                AutumnRenderLayer::BelowGoose,
            );
            render_pose_with_palette_at_scale(
                canvas,
                world.pose(),
                origin,
                world.render_palette(),
                1.0,
            );
            render_autumn_leaves(
                canvas,
                world.autumn(),
                world.now(),
                origin,
                world.goose.position,
                AutumnRenderLayer::AboveGoose,
            );
            render_hearts(canvas, world.hearts(), world.now(), origin);
            render_sleepies(canvas, world.sleepies(), world.now(), origin);
            overlay.present(dirty, canvas)?;
            core.acknowledge_present();
        }

        if core.graceful_stop_complete(&world) {
            println!("honk300: goose walked home; stopping.");
            return Ok(());
        }

        std::thread::sleep(core.next_tick_delay());
    }

    Ok(())
}

fn effective_options(
    config: &Config,
    options: &RuntimeOptions,
    backend: BackendState,
) -> EffectiveOptions {
    config.effective_options(backend, options.cli_overrides)
}

fn backend_state(
    cursor_warp: BackendCapability,
    window_watch: BackendCapability,
    collect_window: BackendCapability,
    presence: BackendCapability,
    audio: BackendCapability,
    note_count: u32,
    meme_count: u32,
) -> BackendState {
    BackendState {
        cursor_warp,
        window_watch,
        collect_window,
        presence,
        audio,
        note_count,
        meme_count,
    }
}

fn window_watcher_requested(options: honk_engine::ForeignWindowOptions) -> bool {
    options.watch_active()
}

fn sound_enabled(config: honk_config::AudioConfig, sound: Sound) -> bool {
    if !config.enabled {
        return false;
    }
    match sound {
        Sound::Honk(_) => config.honk,
        Sound::Bite => config.bite,
        Sound::MudSquish => config.mud,
        Sound::Pat => config.pat,
    }
}

fn desktop_layout_for(
    multi_monitor_chase: bool,
    primary_bounds: Rect,
    monitor_bounds: Vec<Rect>,
) -> Result<DesktopLayout, honk_engine::DesktopLayoutError> {
    if multi_monitor_chase {
        DesktopLayout::new(monitor_bounds)
    } else {
        Ok(DesktopLayout::single(primary_bounds))
    }
}

fn run_accessibility_onboarding(
    onboarding: &mut AccessibilityOnboarding,
    mut accessibility: AccessibilityState,
    request_prompt: impl FnOnce() -> std::io::Result<AccessibilityState>,
    open_settings: impl FnOnce() -> std::io::Result<()>,
) -> AccessibilityState {
    if !onboarding.should_prompt(accessibility) {
        return accessibility;
    }
    match onboarding.mark_prompted() {
        Ok(()) => {
            match request_prompt() {
                Ok(state) => accessibility = state,
                Err(err) => eprintln!(
                    "honk300: macOS rejected the native Accessibility prompt request ({err})"
                ),
            }
            if accessibility == AccessibilityState::Denied {
                if let Err(err) = open_settings() {
                    eprintln!(
                        "honk300: could not open macOS Accessibility settings; open Privacy & Security > Accessibility manually ({err})"
                    );
                }
            }
        }
        Err(err) => {
            eprintln!(
                "honk300: skipped the first-run Accessibility prompt because secure prompt state could not be recorded ({err})"
            );
        }
    }
    accessibility
}

fn attempt_window_watcher<T>(
    requested: bool,
    current_capability: BackendCapability,
    warned: &mut bool,
    failure_context: &str,
    create: impl FnOnce() -> std::io::Result<T>,
) -> (Option<T>, BackendCapability) {
    if !requested {
        return (None, current_capability);
    }
    match create() {
        Ok(watcher) => (Some(watcher), BackendCapability::Supported),
        Err(err) => {
            if !*warned {
                *warned = true;
                eprintln!("{failure_context} ({err})");
            }
            (None, permission_or_failed(&err))
        }
    }
}

fn complete_accessibility_grant<T>(
    world: &mut World,
    watcher_requested: bool,
    warned: &mut bool,
    create_watcher: impl FnOnce() -> std::io::Result<T>,
    apply_window_capability: impl FnOnce(&mut World, BackendCapability),
) -> (Option<T>, BackendCapability) {
    let (watcher, capability) = attempt_window_watcher(
        watcher_requested,
        BackendCapability::Supported,
        warned,
        "honk300: macOS window ride unavailable after Accessibility grant",
        create_watcher,
    );
    apply_window_capability(world, capability);
    world.leave_permission_wait();
    (watcher, capability)
}

fn accessibility_capability() -> BackendCapability {
    accessibility_capability_with(|| Ok(accessibility_state()))
}

fn accessibility_capability_from_state(state: AccessibilityState) -> BackendCapability {
    match state {
        AccessibilityState::Trusted => BackendCapability::Supported,
        AccessibilityState::Denied => BackendCapability::Denied,
    }
}

fn detect_accessibility_onboarding() -> AccessibilityOnboarding {
    let Some(home) = std::env::var_os("HOME").map(std::path::PathBuf::from) else {
        return AccessibilityOnboarding::unmanaged();
    };
    let Ok(executable) = std::env::current_exe() else {
        return AccessibilityOnboarding::unmanaged();
    };
    let Some(metadata) = main_bundle_release_metadata() else {
        return AccessibilityOnboarding::unmanaged();
    };
    match AccessibilityOnboarding::detect(&home, &executable, &metadata) {
        Ok(onboarding) => onboarding,
        Err(err) => {
            eprintln!(
                "honk300: automatic Accessibility onboarding is unavailable; continuing without opening settings ({err})"
            );
            AccessibilityOnboarding::unmanaged()
        }
    }
}

fn accessibility_capability_with(
    probe: impl FnOnce() -> std::io::Result<AccessibilityState>,
) -> BackendCapability {
    match probe() {
        Ok(AccessibilityState::Trusted) => BackendCapability::Supported,
        Ok(AccessibilityState::Denied) => BackendCapability::Denied,
        Err(err) => permission_or_failed(&err),
    }
}

fn permission_or_failed(err: &std::io::Error) -> BackendCapability {
    if err.kind() == std::io::ErrorKind::PermissionDenied {
        BackendCapability::Denied
    } else if err.kind() == std::io::ErrorKind::Unsupported {
        BackendCapability::Unsupported
    } else {
        BackendCapability::Failed
    }
}

fn refresh_accessibility_capability(current: BackendCapability) -> BackendCapability {
    refresh_accessibility_capability_with(current, || Ok(accessibility_state()))
}

fn refresh_accessibility_capability_with(
    current: BackendCapability,
    probe: impl FnOnce() -> std::io::Result<AccessibilityState>,
) -> BackendCapability {
    match current {
        BackendCapability::Unsupported => BackendCapability::Unsupported,
        BackendCapability::Supported | BackendCapability::Denied | BackendCapability::Failed => {
            accessibility_capability_with(probe)
        }
    }
}

fn refresh_supported_capability(current: BackendCapability) -> BackendCapability {
    match current {
        BackendCapability::Failed | BackendCapability::Unsupported => current,
        BackendCapability::Supported | BackendCapability::Denied => BackendCapability::Supported,
    }
}

fn runtime_status(
    cursor: BackendCapability,
    window: BackendCapability,
    collect: BackendCapability,
    presence: BackendCapability,
    audio: BackendCapability,
    notes: u32,
    memes: u32,
) -> RuntimeStatus {
    RuntimeStatus {
        running: true,
        platform: PlatformStatus::Macos,
        bundle: macos_bundle_status(),
        // Reached only after `Overlay::new()?` succeeded, so the AppKit overlay is live.
        overlay: CapabilityStatus::Supported,
        accessibility: capability_status(accessibility_capability()),
        cursor: capability_status(cursor),
        window: capability_status(window),
        collect: capability_status(collect),
        presence: capability_status(presence),
        audio: capability_status(audio),
        notes,
        memes,
    }
}

fn macos_bundle_status() -> BundleStatus {
    std::env::current_exe()
        .ok()
        .and_then(|exe| {
            exe.ancestors()
                .find(|path| path.extension().and_then(|ext| ext.to_str()) == Some("app"))
                .map(|_| BundleStatus::App)
        })
        .unwrap_or(BundleStatus::Bare)
}

fn capability_status(capability: BackendCapability) -> CapabilityStatus {
    match capability {
        BackendCapability::Supported => CapabilityStatus::Supported,
        BackendCapability::Unsupported => CapabilityStatus::Unsupported,
        BackendCapability::Denied => CapabilityStatus::Denied,
        BackendCapability::Failed => CapabilityStatus::Failed,
    }
}

fn seed_from_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::*;
    use honk_platform_macos::MacBundleReleaseMetadata;
    use serde_json::json;
    use std::fs;
    use std::path::PathBuf;

    const TEST_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

    fn managed_onboarding_fixture() -> (tempfile::TempDir, AccessibilityOnboarding, PathBuf) {
        let home = tempfile::tempdir().expect("temporary managed home");
        let app = home.path().join("Applications/Honk300.app");
        let executable = app.join("Contents/MacOS/honk300");
        fs::create_dir_all(executable.parent().expect("MacOS directory")).expect("app tree");
        fs::write(&executable, b"fixture").expect("fixture executable");
        let state_root = home.path().join("Library/Application Support/honk300");
        fs::create_dir_all(&state_root).expect("state root");
        fs::write(
            state_root.join("install-receipt.json"),
            serde_json::to_vec(&json!({
                "schema": "honk300.install.v1",
                "version": "1.0.1",
                "tag": "v1.0.1",
                "commit": TEST_SHA,
                "install_root": app.to_string_lossy(),
            }))
            .expect("receipt json"),
        )
        .expect("receipt");
        let metadata = MacBundleReleaseMetadata {
            bundle_id: "dev.emmetts.honk300".into(),
            version: "1.0.1".into(),
            tag: "v1.0.1".into(),
            commit: TEST_SHA.into(),
        };
        let onboarding = AccessibilityOnboarding::detect(home.path(), &executable, &metadata)
            .expect("managed onboarding");
        let marker = home
            .path()
            .join("Library/Application Support/honk300/state/accessibility-prompt-v1/1.0.1");
        (home, onboarding, marker)
    }

    #[test]
    fn settings_open_failure_keeps_secure_marker_and_permission_wait_eligible() {
        let (_home, mut onboarding, marker) = managed_onboarding_fixture();
        let accessibility = run_accessibility_onboarding(
            &mut onboarding,
            AccessibilityState::Denied,
            || Ok(AccessibilityState::Denied),
            || Err(std::io::Error::other("settings fixture rejected URL")),
        );

        assert_eq!(accessibility, AccessibilityState::Denied);
        assert!(
            marker.is_file(),
            "prompt marker must survive settings failure"
        );
        assert!(onboarding.waiting_for(accessibility));
        assert!(!onboarding.should_prompt(accessibility));
        let mut world = World::new(Rect::new(Vec2::ZERO, Vec2::new(1200.0, 800.0)), 7);
        if onboarding.waiting_for(accessibility) {
            world.enter_permission_wait(Vec2::new(1080.0, 690.0));
        }
        assert!(world.permission_waiting());
    }

    #[test]
    fn grant_watcher_failure_reports_capability_and_still_resumes_first_ux() {
        for (error, expected) in [
            (
                std::io::Error::other("watcher fixture failed"),
                BackendCapability::Failed,
            ),
            (
                std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "watcher permission fixture",
                ),
                BackendCapability::Denied,
            ),
        ] {
            let mut world = World::new(Rect::new(Vec2::ZERO, Vec2::new(1200.0, 800.0)), 7);
            world.enter_permission_wait(Vec2::new(1080.0, 690.0));
            let mut warned = false;
            let mut applied = None;

            let (watcher, capability) = complete_accessibility_grant(
                &mut world,
                true,
                &mut warned,
                || Err::<(), _>(error),
                |world, capability| {
                    applied = Some(capability);
                    world.set_foreign_window_watch_supported(
                        capability == BackendCapability::Supported,
                    );
                },
            );

            assert!(watcher.is_none());
            assert_eq!(capability, expected);
            assert_eq!(applied, Some(expected));
            assert!(warned);
            assert!(!world.permission_waiting());
            assert_eq!(world.current_task(), "first_ux");
        }
    }

    #[test]
    fn denied_startup_skips_watcher_without_consuming_future_warning() {
        let mut warned = false;
        let (watcher, capability) = attempt_window_watcher(
            false,
            BackendCapability::Denied,
            &mut warned,
            "denied startup fixture",
            || -> std::io::Result<()> { panic!("denied startup must not create a watcher") },
        );

        assert!(watcher.is_none());
        assert_eq!(capability, BackendCapability::Denied);
        assert!(!warned);
    }

    #[test]
    fn hosted_accessibility_adapter_maps_granted_without_querying_host_permission() {
        assert_eq!(
            accessibility_capability_with(|| Ok(AccessibilityState::Trusted)),
            BackendCapability::Supported
        );
    }

    #[test]
    fn hosted_accessibility_adapter_maps_denied_without_querying_host_permission() {
        assert_eq!(
            accessibility_capability_with(|| Ok(AccessibilityState::Denied)),
            BackendCapability::Denied
        );
    }

    #[test]
    fn hosted_accessibility_adapter_maps_probe_errors() {
        assert_eq!(
            accessibility_capability_with(|| Err(std::io::Error::other("probe failed"))),
            BackendCapability::Failed
        );
        assert_eq!(
            accessibility_capability_with(|| Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "permission denied",
            ))),
            BackendCapability::Denied
        );
    }

    #[test]
    fn hosted_accessibility_adapter_recovers_after_denial_or_probe_error() {
        for current in [BackendCapability::Denied, BackendCapability::Failed] {
            assert_eq!(
                refresh_accessibility_capability_with(current, || {
                    Ok(AccessibilityState::Trusted)
                }),
                BackendCapability::Supported
            );
        }
    }

    #[test]
    fn hosted_accessibility_adapter_keeps_unsupported_capability_stable() {
        assert_eq!(
            refresh_accessibility_capability_with(BackendCapability::Unsupported, || {
                panic!("unsupported capability must not be re-probed")
            }),
            BackendCapability::Unsupported
        );
    }

    #[test]
    fn window_watcher_creation_requires_enabled_live_capability() {
        let denied = honk_engine::ForeignWindowOptions::with_backend_support(false, true);
        assert!(!window_watcher_requested(denied));

        let mut supported = honk_engine::ForeignWindowOptions::with_backend_support(true, true);
        assert!(window_watcher_requested(supported));
        supported.enabled = false;
        assert!(!window_watcher_requested(supported));
    }
}
