use crate::assets;
use crate::audio;
use crate::runtime::control_surface;
use crate::runtime::core::RuntimeCore;
use crate::runtime::owned_props::Controller as PropController;
use crate::runtime::{audio_probe_capability, RuntimeOptions};
use honk_config::{BackendCapability, BackendState, Config, EffectiveOptions};
use honk_control::{
    BundleStatus, CapabilityStatus, CommandServer, ControlCommand, ControlResponse, DesktopBackend,
    PlatformStatus, RuntimeStatus, SessionStatus,
};
use honk_engine::render::DamageCanvas;
use honk_engine::render::{
    render_autumn_leaves, render_footmarks_with_timing, render_hearts, render_pose_with_palette,
    render_sleepies, AutumnRenderLayer,
};
use honk_engine::{
    CollectWindowCommand, CursorCommand, DesktopLayout, PresenceSnapshot, Rect, Sound, World,
};
use honk_platform_linux::{
    display_cursor_mischief_supported, display_foreign_window_watch_supported, local_time,
    presence_supported, DisplayServer, Overlay, OverlayMode, SessionInfo, StatusTray,
};

pub fn run(
    options: RuntimeOptions,
    server: &CommandServer,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut config = options.config.clone();
    let assets = assets::AssetCatalog::load();
    println!("honk300: loaded {}", assets.summary());

    let session = SessionInfo::detect(options.cli_overrides.wayland || config.platform.wayland);
    if session.display_server == DisplayServer::Wayland && !session.forced_wayland {
        return Err("No X11 display is available. Start with --wayland or enable Use native Wayland in settings to opt into reduced native Wayland mode.".into());
    }
    let mut overlay = Overlay::new(session.display_server)?;
    let mut status_tray = match StatusTray::new() {
        Ok(tray) => Some(tray),
        Err(error) => {
            eprintln!(
                "honk300: Linux StatusNotifier controls are unavailable; CLI controls remain active ({error})"
            );
            None
        }
    };
    let overlay_mode = overlay.mode();
    let display_server = overlay.display_server();
    // GNOME Wayland exposes native windows through its explicit companion.
    // Its XWayland overlay alone cannot inspect all protected desktop surfaces.
    let gnome_wayland = session.desktop == honk_control::DesktopEnvironment::Gnome
        && session.xdg_session_type.as_deref() == Some("wayland");
    let capability_session = if gnome_wayland {
        DisplayServer::Wayland
    } else {
        display_server
    };
    let mut gnome = if overlay_mode == OverlayMode::X11 {
        crate::integrations::GnomeRuntime::start()
    } else {
        crate::integrations::GnomeRuntime::default()
    };
    let mut sway = if overlay_mode == OverlayMode::Wayland {
        crate::integrations::SwayRuntime::start()
    } else {
        crate::integrations::SwayRuntime::default()
    };
    let mut hyprland = if overlay_mode == OverlayMode::Wayland {
        crate::integrations::HyprlandRuntime::start()
    } else {
        crate::integrations::HyprlandRuntime::default()
    };
    let mut kwin = if overlay_mode == OverlayMode::Wayland {
        crate::integrations::KwinRuntime::start()
    } else {
        crate::integrations::KwinRuntime::default()
    };
    eprintln!(
        "honk300: Linux {} runtime active; overlay mode is {:?}.",
        display_server.label(),
        overlay_mode
    );

    let mut cursor_warp = cursor_capability(overlay_mode, capability_session);
    let mut window_watch = window_capability(overlay_mode, capability_session);
    let mut collect_window = BackendCapability::Unsupported;
    let mut props = if overlay_mode == OverlayMode::Headless {
        None
    } else {
        match PropController::start(overlay_mode == OverlayMode::X11) {
            Ok(controller) => Some(controller),
            Err(error) => {
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_none_or(|error| error.kind() != std::io::ErrorKind::NotFound)
                {
                    collect_window = BackendCapability::Failed;
                }
                eprintln!("honk300: native Linux notes and pictures unavailable ({error})");
                None
            }
        }
    };
    let presence = presence_capability(display_server);
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
    if !effective.no_sound && audio.is_none() {
        audio_capability = audio_probe_capability(false);
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

    let layout = desktop_layout_for(
        effective.world.multi_monitor_chase,
        overlay.monitor_bounds(),
        overlay.bounds(),
    )?;
    let seed = seed_from_clock();
    let mut world = World::with_layout_and_options(layout, seed, effective.world);
    let mut core = RuntimeCore::new();
    let mut damage_canvas = DamageCanvas::default();
    const AUDIO_RETRY_INTERVAL: f64 = 5.0;
    let mut next_audio_probe = 0.0;
    let mut warned_cursor = false;
    // Opt-in, bounded state evidence for the isolated native CI fixture. No
    // window titles, user content, or foreign application identities are logged.
    let trace_collection = std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("HONK300_TRACE_COLLECTION").as_deref() == Ok("1");
    let mut next_collection_trace = 0.0;
    let mut collection_trace_count = 0;
    let trace_gnome = std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("HONK300_TRACE_GNOME").as_deref() == Ok("1");
    let mut next_gnome_trace = 0.0;
    let mut gnome_trace_count = 0;

    let trace_presence = std::env::var("GITHUB_ACTIONS").as_deref() == Ok("true")
        && std::env::var("HONK300_TRACE_PRESENCE").as_deref() == Ok("1");
    let mut last_presence_trace = None;
    let mut presence_trace_count = 0;

    println!("honk300: Linux goose control is live. Use `honk300 stop` to send it home.");

    loop {
        if !overlay.pump() {
            eprintln!("honk300: Linux overlay closed.");
            return Ok(());
        }

        if status_tray.as_ref().is_some_and(StatusTray::is_closed) {
            eprintln!(
                "honk300: Linux StatusNotifier controls stopped; CLI controls remain active."
            );
            status_tray = None;
        }
        while let Some(command) = status_tray.as_ref().and_then(StatusTray::take_command) {
            let action = control_surface::command_name(command);
            if let Err(error) = control_surface::handle_command(
                command,
                &mut world,
                || control_surface::open_configuration(&options.config_path),
                control_surface::open_update_helper,
            ) {
                eprintln!("honk300: {action} action could not start ({error})");
            }
        }

        if overlay.take_topology_changed() {
            eprintln!("honk300: desktop layout or scale changed; optional actions are cancelled");
            kwin.cancel_pointer();
            if overlay_mode == OverlayMode::Wayland {
                world.set_cursor_warp_supported(false);
            }
            let layout = desktop_layout_for(
                effective.world.multi_monitor_chase,
                overlay.monitor_bounds(),
                overlay.bounds(),
            )?;
            world.apply_layout(layout);
        }

        let frame = core.begin_frame();

        if let Some(controller) = props.as_mut() {
            match controller.poll() {
                Ok(()) if controller.ready() && collect_window != BackendCapability::Supported => {
                    collect_window = BackendCapability::Supported;
                    world.set_collect_window_supported(true);
                    world.set_collect_window_positioning(overlay_mode == OverlayMode::X11);
                    eprintln!(
                        "honk300: native Linux notes and pictures ready; animated placement: {}",
                        overlay_mode == OverlayMode::X11
                    );
                }
                Ok(()) => {}
                Err(error) => {
                    eprintln!("honk300: native Linux prop connection failed; disabling delivery ({error})");
                    props = None;
                    collect_window = BackendCapability::Failed;
                    world.set_collect_window_supported(false);
                }
            }
        }
        world.set_collect_window_capacity(props.as_ref().is_some_and(PropController::has_capacity));

        while let Some(request) = server.try_recv() {
            match request.command() {
                ControlCommand::SwayStatus => {
                    request.respond(ControlResponse::Wayland(sway.status()));
                }
                ControlCommand::SwayEnable => {
                    let response = if overlay_mode != OverlayMode::Wayland
                        || world.graceful_exit_requested()
                    {
                        ControlResponse::Err("UNSUPPORTED".into())
                    } else {
                        match sway.enable() {
                            Ok(()) => ControlResponse::Ok,
                            Err(error) => {
                                eprintln!("honk300: Sway observation activation failed ({error})");
                                ControlResponse::Err("ADAPTER_FAILED".into())
                            }
                        }
                    };
                    request.respond(response);
                }
                ControlCommand::SwayDisable => {
                    sway.disable();
                    request.respond(ControlResponse::Ok);
                }
                ControlCommand::GnomeStatus => {
                    let mut status = gnome.status();
                    if overlay_mode == OverlayMode::X11
                        && collect_window == BackendCapability::Supported
                    {
                        status.prop_positioning = CapabilityStatus::Supported;
                    }
                    request.respond(ControlResponse::Wayland(status));
                }
                ControlCommand::GnomeEnable => {
                    let response = if overlay_mode != OverlayMode::X11
                        || world.graceful_exit_requested()
                    {
                        ControlResponse::Err("UNSUPPORTED".into())
                    } else {
                        match gnome.enable() {
                            Ok(()) => ControlResponse::Ok,
                            Err(error) => {
                                eprintln!("honk300: Gnome observation activation failed ({error})");
                                ControlResponse::Err("ADAPTER_FAILED".into())
                            }
                        }
                    };
                    request.respond(response);
                }
                ControlCommand::GnomeDisable => {
                    gnome.disable();
                    request.respond(ControlResponse::Ok);
                }
                ControlCommand::HyprlandStatus => {
                    request.respond(ControlResponse::Wayland(hyprland.status()));
                }
                ControlCommand::HyprlandEnable => {
                    let response = if overlay_mode != OverlayMode::Wayland
                        || world.graceful_exit_requested()
                    {
                        ControlResponse::Err("UNSUPPORTED".into())
                    } else {
                        match hyprland.enable() {
                            Ok(()) => ControlResponse::Ok,
                            Err(error) => {
                                eprintln!(
                                    "honk300: Hyprland observation activation failed ({error})"
                                );
                                ControlResponse::Err("ADAPTER_FAILED".into())
                            }
                        }
                    };
                    request.respond(response);
                }
                ControlCommand::HyprlandDisable => {
                    hyprland.disable();
                    request.respond(ControlResponse::Ok);
                }
                ControlCommand::WaylandStatus => {
                    let mut status = kwin.status();
                    if collect_window == BackendCapability::Supported {
                        status.prop_positioning = status.movement;
                    }
                    request.respond(ControlResponse::Wayland(status));
                }
                ControlCommand::PresenceStatus => {
                    let fullscreen = honk_control::combine_capabilities([
                        kwin.status().fullscreen,
                        sway.status().fullscreen,
                        hyprland.status().fullscreen,
                        gnome.status().fullscreen,
                    ]);
                    request.respond(ControlResponse::Presence(honk_control::PresenceStatus {
                        fullscreen,
                        dnd: CapabilityStatus::Unsupported,
                    }));
                }
                ControlCommand::KwinEnable => {
                    let response = if overlay_mode != OverlayMode::Wayland {
                        ControlResponse::Err("UNSUPPORTED".into())
                    } else {
                        match kwin.enable() {
                            Ok(()) => ControlResponse::Ok,
                            Err(error) => {
                                eprintln!("honk300: KDE activation failed ({error})");
                                ControlResponse::Err("ADAPTER_FAILED".into())
                            }
                        }
                    };
                    request.respond(response);
                }
                ControlCommand::KwinDisable => {
                    kwin.disable();
                    if overlay_mode == OverlayMode::Wayland {
                        world.set_cursor_warp_supported(false);
                    }
                    world.set_foreign_window_watch_supported(false);
                    world.set_foreign_window_drag(None);
                    request.respond(ControlResponse::Ok);
                }
                ControlCommand::PointerRequest => {
                    let response = if overlay_mode != OverlayMode::Wayland
                        || world.graceful_exit_requested()
                    {
                        ControlResponse::Err("UNSUPPORTED".into())
                    } else {
                        match kwin.request_pointer() {
                            Ok(()) => ControlResponse::Ok,
                            Err(error) => {
                                eprintln!("honk300: pointer permission request failed ({error})");
                                ControlResponse::Err("POINTER_UNAVAILABLE".into())
                            }
                        }
                    };
                    request.respond(response);
                }
                ControlCommand::PointerCancel => {
                    kwin.cancel_pointer();
                    if overlay_mode == OverlayMode::Wayland {
                        world.set_cursor_warp_supported(false);
                    }
                    request.respond(ControlResponse::Ok);
                }
                ControlCommand::Stop => {
                    kwin.cancel_pointer();
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
                            cursor_warp = cursor_capability(overlay_mode, capability_session);
                            window_watch = window_capability(overlay_mode, capability_session);
                            effective = effective_options(
                                &config,
                                &options,
                                backend_state(
                                    cursor_warp,
                                    window_watch,
                                    collect_window,
                                    presence_capability(display_server),
                                    audio_capability,
                                    assets.note_count(),
                                    assets.meme_count(),
                                ),
                            );
                            if effective.no_sound {
                                audio = None;
                            } else if audio.is_none() {
                                audio = audio::Audio::new();
                                audio_capability = audio_probe_capability(audio.is_some());
                            }
                            world.apply_options(effective.world);
                            world.set_collect_window_positioning(
                                collect_window == BackendCapability::Supported
                                    && (overlay_mode == OverlayMode::X11
                                        || kwin.status().movement == CapabilityStatus::Supported),
                            );
                            world.set_collect_window_capacity(
                                props.as_ref().is_some_and(PropController::has_capacity),
                            );
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
                | ControlCommand::LeafPlace(..)
                | ControlCommand::LeafMove(..)
                | ControlCommand::Snapshot
                | ControlCommand::Leaves => {
                    let response = RuntimeCore::remote_response(&mut world, request.command());
                    request.respond(response);
                }
                ControlCommand::Status => {
                    let fullscreen = honk_control::combine_capabilities([
                        kwin.status().fullscreen,
                        sway.status().fullscreen,
                        hyprland.status().fullscreen,
                        gnome.status().fullscreen,
                    ]);
                    request.respond(ControlResponse::Status(runtime_status(
                        overlay_capability(overlay_mode, display_server),
                        backend_state(
                            cursor_warp,
                            window_watch,
                            collect_window,
                            super::backend_capability(fullscreen),
                            audio_capability,
                            assets.note_count(),
                            assets.meme_count(),
                        ),
                    )));
                }
                ControlCommand::Session => {
                    let backend = match overlay_mode {
                        OverlayMode::X11
                            if session
                                .xdg_session_type
                                .as_deref()
                                .is_some_and(|value| value.eq_ignore_ascii_case("wayland")) =>
                        {
                            DesktopBackend::X11OnWayland
                        }
                        OverlayMode::X11 => DesktopBackend::X11,
                        OverlayMode::Wayland => DesktopBackend::Wayland,
                        OverlayMode::Headless => DesktopBackend::Headless,
                    };
                    let prop_positioning = if collect_window == BackendCapability::Supported
                        && overlay_mode != OverlayMode::X11
                    {
                        kwin.status().movement
                    } else {
                        capability_status(collect_window)
                    };
                    request.respond(ControlResponse::Session(SessionStatus {
                        backend,
                        desktop: session.desktop,
                        prop_positioning,
                    }));
                }
            }
        }

        world.set_local_time(local_time());
        let kwin_frame = kwin.poll();
        let sway_frame = sway.poll();
        let hyprland_frame = hyprland.poll();
        let gnome_frame = gnome.poll();
        if world.graceful_exit_requested() {
            kwin.cancel_pointer();
        }
        if overlay_mode == OverlayMode::Wayland {
            cursor_warp = match kwin.pointer_status() {
                CapabilityStatus::Supported => BackendCapability::Supported,
                CapabilityStatus::Unprobed => BackendCapability::Denied,
                CapabilityStatus::Denied => BackendCapability::Denied,
                CapabilityStatus::Failed => BackendCapability::Failed,
                CapabilityStatus::Unsupported => BackendCapability::Unsupported,
            };
            // A granted device does not authorize a prank over a protected or
            // unknown window. Recheck the complete path again at actual warp.
            world.set_cursor_warp_supported(
                cursor_warp.active()
                    && kwin_frame
                        .as_ref()
                        .is_some_and(|frame| frame.permits_pointer_motion(frame.pointer)),
            );
        }
        world.set_collect_window_positioning(
            collect_window == BackendCapability::Supported
                && (overlay_mode == OverlayMode::X11 || kwin_frame.is_some()),
        );
        window_watch = if kwin_frame.is_some() || gnome_frame.is_some() {
            BackendCapability::Supported
        } else {
            window_capability(overlay_mode, capability_session)
        };
        world.set_foreign_window_watch_supported(window_watch.active());
        let observed = kwin_frame.is_some()
            || sway_frame.is_some()
            || hyprland_frame.is_some()
            || gnome_frame.is_some();
        let fullscreen = gnome_frame.as_ref().is_some_and(|frame| frame.fullscreen())
            || kwin_frame.as_ref().is_some_and(|frame| frame.fullscreen())
            || sway_frame.as_ref().is_some_and(|frame| frame.fullscreen())
            || hyprland_frame
                .as_ref()
                .is_some_and(|frame| frame.fullscreen());
        world.set_presence(if !observed {
            PresenceSnapshot::unsupported()
        } else if fullscreen {
            PresenceSnapshot::fullscreen()
        } else {
            PresenceSnapshot::available()
        });
        if trace_presence && presence_trace_count < 64 {
            let state = (observed, fullscreen, world.manners_active());
            if last_presence_trace != Some(state) {
                last_presence_trace = Some(state);
                presence_trace_count += 1;
                eprintln!(
                    "honk300 presence trace: observed={} fullscreen={} manners={}",
                    state.0, state.1, state.2
                );
            }
        }
        // XWayland cannot observe the global pointer on GNOME Wayland. Do not
        // feed its partial/stale X11 sample into hover, pat or click reactions.
        let mut pointer = if gnome_wayland {
            honk_engine::Pointer::default()
        } else {
            overlay.pointer_state()
        };
        if let Some(frame) = &kwin_frame {
            let pos = honk_engine::Vec2::new(frame.pointer[0] as f32, frame.pointer[1] as f32);
            // KWin supplies position only. Retain a native button observation only
            // when the overlay itself observed that same point.
            pointer.left_down =
                pointer.left_down && pointer.present && (pointer.pos - pos).magnitude() < 1.0;
            pointer.pos = pos;
            pointer.present = world.layout().region_at(pos).is_some();
        }
        world.set_pointer(pointer);
        let gnome_drag = gnome_frame.as_ref().and_then(|frame| gnome.dragged(frame));
        world.set_foreign_window_drag(
            kwin_frame
                .as_ref()
                .and_then(|frame| kwin.dragged(frame))
                .or(gnome_drag)
                .or_else(|| {
                    (!gnome_wayland)
                        .then(|| overlay.foreign_window_drag())
                        .flatten()
                }),
        );
        let collect_snapshot = props
            .as_mut()
            .and_then(|controller| controller.snapshot(kwin_frame.as_ref()));
        world.set_collect_window_snapshot(collect_snapshot);
        let _ = overlay.set_input_region(Some(world.rig().bounding_box()));

        let now = frame.now();
        let task_before_tick = world.current_task();
        core.tick(&mut world, frame);
        if trace_gnome && now >= next_gnome_trace && gnome_trace_count < 1200 {
            next_gnome_trace = now + 0.1;
            gnome_trace_count += 1;
            let native_drag = gnome_frame
                .as_ref()
                .and_then(|frame| frame.dragged_window());
            eprintln!(
                "honk300 gnome trace: {}",
                serde_json::json!({
                    "observed": gnome_frame.is_some(),
                    "fullscreen": gnome_frame.as_ref().is_some_and(|frame| frame.fullscreen()),
                    "manners": world.manners_active(),
                    "drag_id": native_drag.map(|window| window.id),
                    "drag_pid": native_drag.and_then(|window| window.pid),
                    "task": world.current_task(),
                    "position": [world.goose.position.x, world.goose.position.y],
                    "pointer_present": pointer.present,
                    "pointer_left_down": pointer.left_down,
                    "anchor": gnome_drag.map(|window| [window.ride_anchor.x, window.ride_anchor.y]),
                })
            );
        }

        if trace_collection
            && world.is_collect_window_active()
            && now >= next_collection_trace
            && collection_trace_count < 7200
        {
            next_collection_trace = now + 1.0 / 120.0;
            collection_trace_count += 1;
            eprintln!(
                "honk300 collection trace: time={now:.6} sim={:.6} seed={seed} task={task_before_tick}->{} position={:?} target={:?} velocity={:?} forward={:?} feet={:?} beak={:?} snapshot={collect_snapshot:?} kwin_sequence={:?}",
                world.now(), world.current_task(), world.goose.position, world.goose.target_pos,
                world.goose.velocity, world.goose.rig.forward, world.goose.rig.feet_pose,
                world.goose.rig.beak_tip, kwin_frame.as_ref().map(|frame| frame.sequence)
            );
        }

        let collect_display = world
            .layout()
            .region_at(world.goose.position)
            .and_then(|index| world.layout().regions().get(index).copied())
            .unwrap_or_else(|| overlay.bounds());
        for command in world.take_collect_window_commands() {
            if let Some(controller) = props.as_mut() {
                if overlay_mode == OverlayMode::Wayland {
                    if let CollectWindowCommand::Move { id, top_left } = command {
                        controller.move_with_kwin(id, top_left, &mut kwin);
                        continue;
                    }
                }
                if let Err(error) = controller.apply(command, &assets, collect_display) {
                    eprintln!(
                        "honk300: native Linux prop command failed; disabling delivery ({error})"
                    );
                    props = None;
                    collect_window = BackendCapability::Failed;
                    world.set_collect_window_supported(false);
                }
            }
        }

        let cursor_commands = world.take_cursor_commands();
        if let Some(CursorCommand::WarpTo(pos)) = cursor_commands.last().copied() {
            let result = if overlay_mode == OverlayMode::Wayland {
                kwin.warp_pointer(pos)
            } else {
                overlay.warp_cursor(pos)
            };
            if let Err(err) = result {
                cursor_warp = if err.kind() == std::io::ErrorKind::Unsupported {
                    BackendCapability::Unsupported
                } else {
                    BackendCapability::Failed
                };
                world.set_cursor_warp_supported(false);
                if !warned_cursor {
                    warned_cursor = true;
                    eprintln!("honk300: Linux cursor warp unavailable; disabling it ({err})");
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
            for sound in sounds {
                if sound_enabled(effective.audio, sound)
                    && a.play(sound) == audio::PlayOutcome::Failed
                {
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
            render_pose_with_palette(canvas, world.pose(), origin, world.render_palette());
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
            if overlay.present(dirty, canvas)? {
                core.acknowledge_present();
            } else {
                core.defer_present();
            }
        }

        if core.graceful_stop_complete(&world) {
            println!("honk300: goose walked home; stopping.");
            return Ok(());
        }

        std::thread::sleep(core.next_tick_delay());
    }
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

fn cursor_capability(mode: OverlayMode, session: DisplayServer) -> BackendCapability {
    if display_cursor_mischief_supported(session) {
        return if mode == OverlayMode::X11 {
            BackendCapability::Supported
        } else {
            BackendCapability::Failed
        };
    }
    capability_for(session, display_cursor_mischief_supported)
}

fn window_capability(mode: OverlayMode, session: DisplayServer) -> BackendCapability {
    if display_foreign_window_watch_supported(session) {
        return if mode == OverlayMode::X11 {
            BackendCapability::Supported
        } else {
            BackendCapability::Failed
        };
    }
    capability_for(session, display_foreign_window_watch_supported)
}

fn presence_capability(session: DisplayServer) -> BackendCapability {
    capability_for(session, presence_supported)
}

fn capability_for(
    session: DisplayServer,
    supported: impl FnOnce(DisplayServer) -> bool,
) -> BackendCapability {
    if supported(session) {
        BackendCapability::Supported
    } else if session == DisplayServer::Unknown {
        BackendCapability::Failed
    } else {
        BackendCapability::Unsupported
    }
}

/// Map the live overlay to a status capability so `honk300 status` can tell a visible overlay
/// apart from the invisible headless fallback (which only runs when `HONK300_ALLOW_HEADLESS=1`).
fn overlay_capability(mode: OverlayMode, session: DisplayServer) -> CapabilityStatus {
    match mode {
        OverlayMode::X11 | OverlayMode::Wayland => CapabilityStatus::Supported,
        OverlayMode::Headless => {
            if session == DisplayServer::Unknown {
                // No display server was ever detected: nothing to bring up, not a failure.
                CapabilityStatus::Unsupported
            } else {
                // An X11/Wayland overlay was attempted and fell back headless.
                CapabilityStatus::Failed
            }
        }
    }
}

fn runtime_status(overlay: CapabilityStatus, backend: BackendState) -> RuntimeStatus {
    RuntimeStatus {
        running: true,
        platform: PlatformStatus::Linux,
        bundle: BundleStatus::Bare,
        overlay,
        accessibility: CapabilityStatus::Unsupported,
        cursor: capability_status(backend.cursor_warp),
        window: capability_status(backend.window_watch),
        collect: capability_status(backend.collect_window),
        presence: capability_status(backend.presence),
        audio: capability_status(backend.audio),
        notes: backend.note_count,
        memes: backend.meme_count,
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

fn seed_from_clock() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
}

fn desktop_layout_for(
    multi_monitor_chase: bool,
    monitor_bounds: Vec<Rect>,
    fallback_bounds: Rect,
) -> Result<DesktopLayout, honk_engine::DesktopLayoutError> {
    if multi_monitor_chase {
        DesktopLayout::new(monitor_bounds)
    } else {
        Ok(DesktopLayout::single(
            monitor_bounds.into_iter().next().unwrap_or(fallback_bounds),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_display_maps_core_capabilities_to_failed() {
        assert_eq!(
            cursor_capability(OverlayMode::Headless, DisplayServer::Unknown),
            BackendCapability::Failed
        );
        assert_eq!(
            window_capability(OverlayMode::Headless, DisplayServer::Unknown),
            BackendCapability::Failed
        );
    }

    #[test]
    fn x11_session_with_headless_fallback_reports_failed_desktop_capabilities() {
        assert_eq!(
            cursor_capability(OverlayMode::Headless, DisplayServer::X11),
            BackendCapability::Failed
        );
        assert_eq!(
            window_capability(OverlayMode::Headless, DisplayServer::X11),
            BackendCapability::Failed
        );
    }

    #[test]
    fn wayland_reports_core_mischief_unsupported_not_denied() {
        assert_eq!(
            cursor_capability(OverlayMode::Wayland, DisplayServer::Wayland),
            BackendCapability::Unsupported
        );
        assert_eq!(
            window_capability(OverlayMode::Wayland, DisplayServer::Wayland),
            BackendCapability::Unsupported
        );
    }

    #[test]
    fn x11_reports_supported_cursor_and_window() {
        assert_eq!(
            cursor_capability(OverlayMode::X11, DisplayServer::X11),
            BackendCapability::Supported
        );
        assert_eq!(
            window_capability(OverlayMode::X11, DisplayServer::X11),
            BackendCapability::Supported
        );
    }

    #[test]
    fn linux_runtime_status_keeps_platform_and_bundle_stable() {
        let status = runtime_status(
            CapabilityStatus::Supported,
            backend_state(
                BackendCapability::Unsupported,
                BackendCapability::Unsupported,
                BackendCapability::Unsupported,
                BackendCapability::Unsupported,
                BackendCapability::Supported,
                2,
                3,
            ),
        );
        assert_eq!(status.platform, PlatformStatus::Linux);
        assert_eq!(status.bundle, BundleStatus::Bare);
        assert_eq!(status.overlay, CapabilityStatus::Supported);
        assert_eq!(status.audio, CapabilityStatus::Supported);
        assert_eq!(status.notes, 2);
        assert_eq!(status.memes, 3);
    }

    #[test]
    fn overlay_capability_distinguishes_visible_from_headless_fallback() {
        // A visible X11/Wayland overlay reports supported.
        assert_eq!(
            overlay_capability(OverlayMode::X11, DisplayServer::X11),
            CapabilityStatus::Supported
        );
        assert_eq!(
            overlay_capability(OverlayMode::Wayland, DisplayServer::Wayland),
            CapabilityStatus::Supported
        );
        // A headless fallback after a real X11/Wayland attempt reports failed, not supported.
        assert_eq!(
            overlay_capability(OverlayMode::Headless, DisplayServer::X11),
            CapabilityStatus::Failed
        );
        assert_eq!(
            overlay_capability(OverlayMode::Headless, DisplayServer::Wayland),
            CapabilityStatus::Failed
        );
        // No display server detected at all is unsupported, not a failure.
        assert_eq!(
            overlay_capability(OverlayMode::Headless, DisplayServer::Unknown),
            CapabilityStatus::Unsupported
        );
    }
}
