use crate::assets;
use crate::audio;
use crate::runtime::control_surface;
use crate::runtime::core::RuntimeCore;
use crate::runtime::remote::RemoteSlot;
use crate::runtime::{audio_probe_capability, RuntimeOptions};
use honk_config::{BackendCapability, BackendState, Config, EffectiveOptions};
use honk_control::{
    BundleStatus, CapabilityStatus, CommandServer, ControlCommand, ControlResponse,
    ControlSurfaceCommand, PlatformStatus, RuntimeStatus,
};
use honk_engine::render::DamageCanvas;
use honk_engine::render::{
    render_autumn_leaves, render_footmarks_with_timing, render_hearts, render_pose_with_palette,
    render_sleepies, render_toys, AutumnRenderLayer,
};
use honk_engine::{
    collect_window::fitted_collect_image_size, CollectWindowCommand, CollectWindowId,
    CollectWindowPayload, CursorCommand, DesktopLayout, LocalTime, Pointer, PresenceSnapshot, Rect,
    Sound, Vec2, World,
};
use honk_platform_windows::{
    init_dpi_awareness, local_time, pointer_state, presence_state, warp_cursor,
    CollectWindowController, ForeignWindowWatcher, Overlay, StatusTray,
};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub fn run(
    options: RuntimeOptions,
    server: &CommandServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = options.config.clone();
    let assets = assets::AssetCatalog::load();
    let smoke_note = smoke_note_text();
    let smoke_note_evidence = std::env::var_os("HONK300_SMOKE_NOTE_EVIDENCE").map(PathBuf::from);
    let smoke_image_evidence = std::env::var_os("HONK300_SMOKE_IMAGE_EVIDENCE").map(PathBuf::from);
    println!("honk300: loaded {}", assets.summary());

    // Opt into Per-Monitor-V2 DPI awareness before creating any window or enumerating monitors, so
    // every Win32 coordinate the backend and engine share is in one physical-pixel space.
    init_dpi_awareness();

    let mut overlay = Overlay::new()?;
    let mut status_tray = match StatusTray::new() {
        Ok(tray) => Some(tray),
        Err(error) => {
            eprintln!(
                "honk300: Windows notification-area controls are unavailable; CLI controls remain active ({error})"
            );
            None
        }
    };
    let primary_bounds = overlay.primary_monitor_bounds();

    // Backend capability only: Windows can always warp the cursor. The user's mouse-steal
    // preference is applied separately via MouseStealOptions::enabled in effective_options, so
    // it must NOT be folded in here — doing so would keep warp latched off across a reload that
    // re-enables stealing. The flag only flips to false later if a real warp call fails.
    let mut cursor_warp = initial_cursor_warp_capability();
    let mut window_watch = BackendCapability::Supported;
    let mut collect_window = BackendCapability::Supported;
    let mut presence = BackendCapability::Supported;
    let mut presence_status = honk_control::PresenceStatus::unprobed();
    let mut audio_capability = BackendCapability::Supported;
    let initial_effective = effective_options(
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
    if initial_effective.wayland {
        eprintln!("honk300: --wayland is recorded for M18; Windows uses the native overlay.");
    }
    let mut audio = if initial_effective.no_sound {
        None
    } else {
        audio::Audio::new()
    };
    if !initial_effective.no_sound {
        audio_capability = audio_probe_capability(audio.is_some());
    }

    let mut warned_window_ride = false;
    let mut window_watcher = if !initial_effective.world.foreign_window.enabled {
        None
    } else {
        match ForeignWindowWatcher::new(&overlay) {
            Ok(watcher) => Some(watcher),
            Err(err) => {
                window_watch = BackendCapability::Failed;
                warned_window_ride = true;
                eprintln!("honk300: window ride unavailable; disabling perch-and-ride ({err})");
                None
            }
        }
    };
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

    let layout = desktop_layout_for(
        effective.world.multi_monitor_chase,
        primary_bounds,
        overlay.monitor_bounds(),
    )?;
    let mut world = World::with_layout_and_options(layout, seed_from_clock(), effective.world);
    if let Some(kind) = smoke_collect_kind() {
        world.force_collect_window(kind);
    }
    let mut collect_controller = CollectWindowController::new(primary_bounds);
    let mut core = RuntimeCore::new();
    let mut damage_canvas = DamageCanvas::default();
    let mut frame_profile = super::windows_profile::FrameProfile::from_env();
    const PRESENCE_POLL_INTERVAL: f64 = 0.5;
    const AUDIO_RETRY_INTERVAL: f64 = 5.0;
    let mut warned_cursor_warp = false;
    let mut warned_collect_window = false;
    let mut warned_presence = false;
    let mut last_presence_poll = f64::NEG_INFINITY;
    let mut next_audio_probe = 0.0;
    let mut smoke_note_id: Option<CollectWindowId> = None;
    let mut smoke_note_origin: Option<Vec2> = None;
    let mut smoke_note_recorded = false;
    let mut smoke_image_recorded = false;

    println!("honk300: a goose is loose on your desktop. Use `honk300 stop` to send it home.");

    // Opt-in remote control (ADR 0059). It only starts after the tray exists so a connection is
    // never invisible; without a tray it stays off.
    let mut remote = RemoteSlot::default();
    if status_tray.is_some() {
        remote.sync(&config.remote);
    } else if config.remote.enabled {
        eprintln!("honk300: remote control stays off because the tray indicator is unavailable.");
    }

    loop {
        if !overlay.pump() {
            break;
        }
        world.set_collect_window_capacity(collect_controller.has_capacity());

        if status_tray
            .as_mut()
            .is_some_and(|tray| tray.maintain().is_err())
        {
            eprintln!(
                "honk300: Windows notification-area controls could not be restored; CLI controls remain active."
            );
            status_tray = None;
        }
        if let Some(tray) = status_tray.as_mut() {
            tray.set_remote_indicator(remote.indicator(), remote.relay_host());
        }
        while let Some(command) = status_tray.as_ref().and_then(StatusTray::take_command) {
            if command == ControlSurfaceCommand::DisconnectRemote {
                remote.disconnect();
                continue;
            }
            let action = control_surface::command_name(command);
            if let Err(error) = control_surface::handle_command(
                command,
                &mut world,
                || control_surface::open_configuration(&options.config_path),
                control_surface::open_update_helper,
            ) {
                eprintln!("honk300: {action} action could not start ({error})");
                if let Some(tray) = status_tray.as_ref() {
                    tray.show_action_error(action, &error);
                }
            }
        }

        // A monitor's DPI, resolution, or the display topology changed (WM_DPICHANGED /
        // WM_DISPLAYCHANGE). Re-enumerate monitors, rebuild the per-monitor overlay windows, and
        // apply the new world bounds so the goose keeps roaming the whole (possibly resized)
        // desktop. Under PMv2 all coordinates stay in one physical-pixel space, so this is a pure
        // bounds/window refresh with no scaling math.
        if overlay.take_monitors_changed() {
            match overlay.rebuild_monitors() {
                Ok(_) => {
                    let new_layout = desktop_layout_for(
                        effective.world.multi_monitor_chase,
                        overlay.primary_monitor_bounds(),
                        overlay.monitor_bounds(),
                    )?;
                    world.apply_layout(new_layout);
                    // Overlay HWNDs may have been added/removed; rebuild the foreign-window
                    // watcher so its own-overlay filter matches the new window set.
                    if window_watcher.is_some() {
                        match ForeignWindowWatcher::new(&overlay) {
                            Ok(watcher) => window_watcher = Some(watcher),
                            Err(err) => {
                                window_watch = BackendCapability::Failed;
                                window_watcher = None;
                                world.set_foreign_window_watch_supported(false);
                                if !warned_window_ride {
                                    warned_window_ride = true;
                                    eprintln!(
                                        "honk300: window ride unavailable after monitor rebuild ({err})"
                                    );
                                }
                            }
                        }
                    }
                    // Force a full repaint on the next present against the rebuilt windows.
                    eprintln!(
                        "honk300: monitor topology changed; rebuilt overlay and world bounds."
                    );
                }
                Err(err) => {
                    eprintln!("honk300: monitor rebuild failed; keeping prior overlay ({err})");
                }
            }
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
                            if !effective.world.foreign_window.enabled {
                                window_watcher = None;
                            } else if window_watcher.is_none() && window_watch.active() {
                                match ForeignWindowWatcher::new(&overlay) {
                                    Ok(watcher) => window_watcher = Some(watcher),
                                    Err(err) => {
                                        window_watch = BackendCapability::Failed;
                                        if !warned_window_ride {
                                            warned_window_ride = true;
                                            eprintln!(
                                                "honk300: window ride unavailable after reload ({err})"
                                            );
                                        }
                                    }
                                }
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
                            }
                            if effective.no_sound {
                                audio = None;
                            } else if audio.is_none() {
                                audio = audio::Audio::new();
                                audio_capability = audio_probe_capability(audio.is_some());
                            }
                            world.apply_options(effective.world);
                            if status_tray.is_some() {
                                remote.sync(&config.remote);
                            }
                            println!("honk300: reload command applied.");
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
                | ControlCommand::ToyPlace(..)
                | ControlCommand::ToyMove(..)
                | ControlCommand::Snapshot
                | ControlCommand::Toys => {
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

        let now = frame.now();
        world.set_local_time(runtime_local_time());
        if now - last_presence_poll >= PRESENCE_POLL_INTERVAL {
            last_presence_poll = now;
            match presence_state() {
                Ok(snapshot) => {
                    presence = BackendCapability::from(snapshot.supported);
                    let state = capability_status(presence);
                    presence_status = honk_control::PresenceStatus {
                        fullscreen: state,
                        dnd: state,
                    };
                    world.set_presence(snapshot);
                }
                Err(err) => {
                    presence = BackendCapability::Failed;
                    presence_status = honk_control::PresenceStatus {
                        fullscreen: CapabilityStatus::Failed,
                        dnd: CapabilityStatus::Failed,
                    };
                    world.set_presence(PresenceSnapshot::unsupported());
                    if !warned_presence {
                        warned_presence = true;
                        eprintln!(
                            "honk300: Windows presence unavailable; DND/fullscreen respect disabled ({err})"
                        );
                    }
                }
            }
        }

        // Feed the cursor before ticking: tasks such as nab_mouse chase the newest pointer
        // sample, then emit platform-free cursor commands for the backend to apply below.
        let (mx, my, left_down) = pointer_state();
        world.set_pointer(Pointer {
            pos: Vec2::new(mx, my),
            present: true,
            left_down,
        });

        let mut disable_window_watcher = false;
        let dragged_window = match window_watcher.as_mut() {
            Some(watcher) => match watcher.active_drag() {
                Ok(snapshot) => snapshot,
                Err(err) => {
                    disable_window_watcher = true;
                    if !warned_window_ride {
                        warned_window_ride = true;
                        eprintln!(
                            "honk300: window ride polling failed; disabling perch-and-ride ({err})"
                        );
                    }
                    None
                }
            },
            None => None,
        };
        if disable_window_watcher {
            window_watch = BackendCapability::Failed;
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
        collect_controller.update_display_bounds(collect_display);

        for command in world.take_collect_window_commands() {
            let result = match command {
                CollectWindowCommand::Spawn { request, payload } => match payload {
                    CollectWindowPayload::Note { .. } => {
                        collect_controller.spawn_note(request).and_then(|id| {
                            let Some(id) = id else { return Ok(()) };
                            if let Some(text) = smoke_note.as_deref() {
                                collect_controller.type_text(id, text)?;
                                smoke_note_id = Some(id);
                                println!(
                                    "honk300: owned Windows calibration note created and prefilled."
                                );
                                Ok(())
                            } else {
                                Ok(())
                            }
                        })
                    }
                    CollectWindowPayload::Meme { index } => {
                        if let Some(meme) = assets.meme(index) {
                            let result =
                                collect_controller.spawn_image(request, &meme.title, &meme.pixmap);
                            if matches!(result, Ok(Some(_))) && !smoke_image_recorded {
                                if let Some(path) = smoke_image_evidence.as_deref() {
                                    write_smoke_image_evidence(
                                        path,
                                        index,
                                        meme.pixmap.width(),
                                        meme.pixmap.height(),
                                        collect_display,
                                    )?;
                                    smoke_image_recorded = true;
                                }
                            }
                            result.map(|_| ())
                        } else {
                            Ok(())
                        }
                    }
                },
                CollectWindowCommand::Move { id, top_left } => {
                    let result = collect_controller.move_window(id, top_left);
                    if result.is_ok() && smoke_note_id == Some(id) && !smoke_note_recorded {
                        if let Some(origin) = smoke_note_origin {
                            if Vec2::distance(origin, top_left) >= 20.0 {
                                if let Some(path) = smoke_note_evidence.as_deref() {
                                    let text = smoke_note
                                        .as_deref()
                                        .expect("smoke note id requires smoke note text");
                                    write_smoke_note_evidence(path, origin, top_left, text)?;
                                }
                                smoke_note_recorded = true;
                            }
                        } else {
                            smoke_note_origin = Some(top_left);
                        }
                    }
                    result
                }
                CollectWindowCommand::SetPassthrough { id, passthrough } => {
                    collect_controller.set_passthrough(id, passthrough)
                }
                CollectWindowCommand::Focus { id } => {
                    collect_controller.focus(id);
                    Ok(())
                }
                CollectWindowCommand::TypeNote { id, note_index } => {
                    if let Some(text) = smoke_note
                        .as_deref()
                        .or_else(|| assets.note_text(note_index))
                    {
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
                // Latch the loss in the runtime capability flag too, so a later reload rebuilds
                // the world with collect still disabled instead of resurrecting it.
                collect_window = BackendCapability::Failed;
                world.set_collect_window_supported(false);
                if !warned_collect_window {
                    warned_collect_window = true;
                    eprintln!("honk300: collect-window unavailable; disabling it ({err})");
                }
            }
        }

        // Apply at most the newest warp request. If the OS/session rejects cursor warping,
        // degrade honestly and stop registering further mouse-steal behavior.
        if let Some(CursorCommand::WarpTo(pos)) = world.take_cursor_commands().last().copied() {
            if let Err(err) = warp_cursor(pos) {
                cursor_warp = BackendCapability::Failed;
                world.set_cursor_warp_supported(false);
                if !warned_cursor_warp {
                    warned_cursor_warp = true;
                    eprintln!("honk300: cursor warp unavailable; disabling mouse stealing ({err})");
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

        // Drain and play any sounds the sim requested this frame (silently dropped if muted).
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
            let profile_start = frame_profile.as_ref().map(|_| std::time::Instant::now());
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
            render_toys(
                canvas,
                world.toys(),
                world.now(),
                origin,
                AutumnRenderLayer::BelowGoose,
            );
            render_pose_with_palette(canvas, world.pose(), origin, world.render_palette());
            render_autumn_leaves(
                canvas,
                world.autumn(),
                world.now(),
                origin,
                world.goose.position,
                AutumnRenderLayer::AboveGoose,
            );
            render_toys(
                canvas,
                world.toys(),
                world.now(),
                origin,
                AutumnRenderLayer::AboveGoose,
            );
            render_hearts(canvas, world.hearts(), world.now(), origin);
            render_sleepies(canvas, world.sleepies(), world.now(), origin);
            if let Some(profile) = frame_profile.as_mut() {
                let start = profile_start.expect("profile clock");
                let render_us = start.elapsed().as_secs_f64() * 1e6;
                let native = overlay.present_profiled(dirty, canvas)?;
                profile.record(start, render_us, native);
            } else {
                overlay.present(dirty, canvas)?;
            }
            core.acknowledge_present();
        }

        if core.graceful_stop_complete(&world) {
            println!("honk300: goose walked home; stopping.");
            return Ok(());
        }

        // Yield so the loop doesn't busy-spin; the accumulator keeps the sim at 120 Hz.
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

#[cfg(test)]
fn world_bounds_for(multi_monitor_chase: bool, primary_bounds: Rect, virtual_bounds: Rect) -> Rect {
    if multi_monitor_chase {
        virtual_bounds
    } else {
        primary_bounds
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

fn smoke_collect_kind() -> Option<honk_engine::CollectWindowKind> {
    match std::env::var("HONK300_SMOKE_COLLECT")
        .ok()?
        .to_ascii_lowercase()
        .as_str()
    {
        "note" => Some(honk_engine::CollectWindowKind::Note),
        "meme" => Some(honk_engine::CollectWindowKind::Meme),
        _ => None,
    }
}

/// Exact text used only by the interactive Windows calibration preflight. The ordinary runtime
/// never sets this variable; keeping the override next to the existing smoke hooks lets the
/// qualification prove the real goose-driven owned-note path without changing user media.
fn smoke_note_text() -> Option<String> {
    std::env::var("HONK300_SMOKE_NOTE_TEXT")
        .ok()
        .filter(|text| !text.trim().is_empty())
}

fn write_smoke_note_evidence(
    path: &Path,
    origin: Vec2,
    moved: Vec2,
    text: &str,
) -> std::io::Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let text_sha256 = format!("{:x}", Sha256::digest(text.as_bytes()));
    std::fs::write(
        &temporary,
        format!(
            "schema=honk300.windows.calibration-note.v1\npid={}\ntext_sha256={}\nfrom={:.1},{:.1}\nto={:.1},{:.1}\ndistance={:.1}\nresult=created-prefilled-and-moved\n",
            std::process::id(),
            text_sha256,
            origin.x,
            origin.y,
            moved.x,
            moved.y,
            Vec2::distance(origin, moved),
        ),
    )?;
    std::fs::rename(temporary, path)
}

fn write_smoke_image_evidence(
    path: &Path,
    index: u32,
    source_width: u32,
    source_height: u32,
    display: Rect,
) -> std::io::Result<()> {
    let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
    let (fitted_width, fitted_height) =
        fitted_collect_image_size(source_width, source_height, display);
    std::fs::write(
        &temporary,
        format!(
            "schema=honk300.windows.collect-image.v1\npid={}\nindex={}\nsource_width={}\nsource_height={}\nfitted_width={}\nfitted_height={}\ndisplay={:.0},{:.0},{:.0},{:.0}\nresult=created-aspect-fitted\n",
            std::process::id(),
            index,
            source_width,
            source_height,
            fitted_width,
            fitted_height,
            display.min.x,
            display.min.y,
            display.width(),
            display.height(),
        ),
    )?;
    std::fs::rename(temporary, path)
}

fn runtime_local_time() -> LocalTime {
    let mut time = local_time();
    if let Some(day) = smoke_local_date() {
        time.day = day;
    }
    time
}

fn smoke_local_date() -> Option<i32> {
    parse_smoke_local_date(&std::env::var("HONK300_SMOKE_LOCAL_DATE").ok()?)
}

fn parse_smoke_local_date(value: &str) -> Option<i32> {
    if value.len() != 8 || !value.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let day = value.parse::<i32>().ok()?;
    let year = day / 10_000;
    let month = (day / 100) % 100;
    let date = day % 100;
    let max_day = days_in_month(year, month)?;
    (year >= 1900 && (1..=max_day).contains(&date)).then_some(day)
}

fn days_in_month(year: i32, month: i32) -> Option<i32> {
    Some(match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => return None,
    })
}

fn is_leap_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

/// The platform's initial cursor-warp capability, before any runtime warp attempt.
///
/// On Windows this is unconditionally `true` (`SetCursorPos` always exists); it is a backend
/// capability, deliberately independent of the user's mouse-steal preference, which is applied
/// separately via `MouseStealOptions::enabled`.
fn initial_cursor_warp_capability() -> BackendCapability {
    BackendCapability::Supported
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
        platform: PlatformStatus::Windows,
        bundle: BundleStatus::Bare,
        // Reached only after `Overlay::new()?` succeeded, so the layered overlay is live.
        overlay: CapabilityStatus::Supported,
        accessibility: CapabilityStatus::Unsupported,
        cursor: capability_status(cursor),
        window: capability_status(window),
        collect: capability_status(collect),
        presence: capability_status(presence),
        audio: capability_status(audio),
        notes,
        memes,
    }
}

fn capability_status(capability: BackendCapability) -> CapabilityStatus {
    match capability {
        BackendCapability::Supported => CapabilityStatus::Supported,
        BackendCapability::Unsupported => CapabilityStatus::Unsupported,
        BackendCapability::Denied => CapabilityStatus::Denied,
        BackendCapability::Failed => CapabilityStatus::Failed,
    }
}

/// A non-deterministic seed for the roam driver, derived from the wall clock.
fn seed_from_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

#[cfg(test)]
mod tests {
    use super::{initial_cursor_warp_capability, parse_smoke_local_date, world_bounds_for};
    use honk_config::BackendCapability;
    use honk_engine::{Rect, Vec2};

    #[test]
    fn initial_cursor_warp_capability_ignores_mouse_steal_preference() {
        // The cursor-warp capability is a backend trait — Windows can always warp the cursor —
        // not a user choice. Seeding it from the no-mouse-steal preference would wrongly latch
        // warp off and keep it off across a reload that re-enables stealing. The preference is
        // applied separately through MouseStealOptions::enabled, so this stays a pure capability.
        assert_eq!(
            initial_cursor_warp_capability(),
            BackendCapability::Supported
        );
    }

    #[test]
    fn smoke_local_date_accepts_yyyymmdd_only() {
        assert_eq!(parse_smoke_local_date("20261015"), Some(20261015));
        assert_eq!(parse_smoke_local_date("20240229"), Some(20240229));
        assert_eq!(parse_smoke_local_date("20260231"), None);
        assert_eq!(parse_smoke_local_date("20250229"), None);
        assert_eq!(parse_smoke_local_date("2026-10-15"), None);
        assert_eq!(parse_smoke_local_date("20261301"), None);
        assert_eq!(parse_smoke_local_date("18991231"), None);
    }

    #[test]
    fn world_bounds_follow_multi_monitor_chase_setting() {
        let primary = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(1920.0, 1080.0));
        let virtual_desktop = Rect::new(Vec2::new(-1280.0, 0.0), Vec2::new(1920.0, 1080.0));

        assert_eq!(world_bounds_for(false, primary, virtual_desktop), primary);
        assert_eq!(
            world_bounds_for(true, primary, virtual_desktop),
            virtual_desktop
        );
    }
}
