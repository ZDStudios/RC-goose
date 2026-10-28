use honk_config::{Config, ConfigError, ConfigSnapshot};
use honk_control::{
    ControlCommand, ControlResponse, RemoteSnapshot, ToyPosition, MAX_REPORTED_TOYS,
};
use honk_engine::{Accumulator, Clock, PokeOutcome, Rect, ToyId, Vec2, World};
use std::time::Duration;

const PRESENT_INTERVAL: f64 = 1.0 / 60.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct RuntimeFrame {
    id: u64,
    now: f64,
    dt: f64,
}

impl RuntimeFrame {
    pub(crate) fn now(self) -> f64 {
        self.now
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Commands,
    Ticked,
}

/// Shared platform-neutral runtime sequencing.
///
/// Native event pumps and capability I/O remain in each platform module. This core pins the
/// common order: sample the clock, process config/control commands, fixed-step the world, then
/// compute current-vs-previous visual damage at the presentation cadence.
pub(crate) struct RuntimeCore {
    clock: Clock,
    accumulator: Accumulator,
    last_frame_time: f64,
    last_present_time: f64,
    /// Bounds from the last overlay present that returned success.
    last_visual_bounds: Option<Rect>,
    /// Current bounds associated with damage handed to the platform but not yet acknowledged.
    /// The outer option distinguishes "no pending present" from a pending transparent clear.
    pending_visual_bounds: Option<Option<Rect>>,
    pending_present_time: Option<f64>,
    phase: Phase,
    frame_id: u64,
}

impl RuntimeCore {
    pub(crate) fn new() -> Self {
        let clock = Clock::start();
        let now = clock.elapsed_secs();
        Self {
            clock,
            accumulator: Accumulator::new(),
            last_frame_time: now,
            last_present_time: f64::NEG_INFINITY,
            last_visual_bounds: None,
            pending_visual_bounds: None,
            pending_present_time: None,
            phase: Phase::Idle,
            frame_id: 0,
        }
    }

    /// Begin a frame after the platform event pump. Config/control requests must be handled after
    /// this call and before [`Self::tick`].
    pub(crate) fn begin_frame(&mut self) -> RuntimeFrame {
        self.begin_at(self.clock.elapsed_secs())
    }

    pub(crate) fn next_tick_delay(&self) -> Duration {
        self.next_tick_delay_at(self.clock.elapsed_secs())
    }

    fn next_tick_delay_at(&self, now: f64) -> Duration {
        let elapsed = (now - self.last_frame_time).max(0.0);
        Duration::from_secs_f64((self.accumulator.until_next_tick() - elapsed).max(0.0))
    }

    fn begin_at(&mut self, now: f64) -> RuntimeFrame {
        assert_eq!(
            self.phase,
            Phase::Idle,
            "previous runtime frame was not finished"
        );
        let dt = (now - self.last_frame_time).max(0.0);
        self.last_frame_time = now;
        self.frame_id = self.frame_id.wrapping_add(1);
        self.phase = Phase::Commands;
        RuntimeFrame {
            id: self.frame_id,
            now,
            dt,
        }
    }

    pub(crate) fn tick(&mut self, world: &mut World, frame: RuntimeFrame) {
        self.assert_frame(frame, Phase::Commands);
        for _ in 0..self.accumulator.pump(frame.dt) {
            world.tick();
        }
        self.phase = Phase::Ticked;
    }

    /// Begin the shared graceful-stop contract. Platform loops keep pumping, ticking, and
    /// presenting until [`Self::graceful_stop_complete`] becomes true.
    pub(crate) fn begin_graceful_stop(world: &mut World) {
        world.request_graceful_exit();
    }

    pub(crate) fn graceful_stop_complete(&self, world: &World) -> bool {
        world.graceful_exit_complete()
            && world.visual_bounds().is_none()
            && self.last_visual_bounds.is_none()
            && self.pending_visual_bounds.is_none()
    }

    /// Finish the frame and return the region that must be repainted, if the present cadence is
    /// due and any current/previous visual pixels need drawing or clearing.
    pub(crate) fn damage(&mut self, world: &World, frame: RuntimeFrame) -> Option<Rect> {
        self.assert_frame(frame, Phase::Ticked);
        assert!(
            self.pending_visual_bounds.is_none(),
            "platform did not acknowledge the previous overlay present"
        );
        self.phase = Phase::Idle;
        if frame.now - self.last_present_time < PRESENT_INTERVAL {
            return None;
        }
        let current = world.visual_bounds();
        let damage = World::damage_bounds(self.last_visual_bounds, current);
        if damage.is_some() {
            self.pending_visual_bounds = Some(current);
            self.pending_present_time = Some(frame.now);
        } else {
            // No prior or current pixels exist, so no platform present is required.
            self.last_present_time = frame.now;
        }
        damage
    }

    /// Commit damage bookkeeping only after the platform overlay accepted the rendered frame.
    /// A transparent final frame is represented by `Some(None)` and is therefore acknowledged
    /// just like a visible frame before terminal shutdown may complete.
    pub(crate) fn acknowledge_present(&mut self) {
        self.last_visual_bounds = self
            .pending_visual_bounds
            .take()
            .expect("overlay present acknowledged without pending damage");
        self.last_present_time = self
            .pending_present_time
            .take()
            .expect("pending overlay damage did not retain its presentation time");
    }

    /// A backpressured presenter has not accepted these pixels. Retry against the last
    /// acknowledged bounds, including any transparent final clear, on the next frame.
    #[cfg(any(test, target_os = "linux"))]
    pub(crate) fn defer_present(&mut self) {
        assert!(self.pending_visual_bounds.take().is_some());
        self.pending_present_time = None;
    }

    /// Answer the remote-control IPC commands (ADR 0059) identically on every platform. Every
    /// mutation goes through the engine's manners-aware remote entry points.
    pub(crate) fn remote_response(world: &mut World, command: ControlCommand) -> ControlResponse {
        let point = |x: i32, y: i32| Vec2::new(x as f32, y as f32);
        match command {
            ControlCommand::RemoteDo(action) => world.remote_poke(action).into(),
            ControlCommand::WalkTo(x, y) => world.walk_to(point(x, y)).into(),
            ControlCommand::ToyPlace(kind, x, y) => match world.place_toy(kind, point(x, y)) {
                Ok(id) => ControlResponse::Toy(id.0),
                Err(outcome) => outcome.into(),
            },
            ControlCommand::ToyMove(id, x, y) => match world.move_toy(ToyId(id), point(x, y)) {
                PokeOutcome::Unsupported => ControlResponse::Err("GONE".into()),
                outcome => outcome.into(),
            },
            ControlCommand::Snapshot => {
                let bounds = world.layout().bounds();
                let position = world.goose_position();
                ControlResponse::Snapshot(RemoteSnapshot {
                    task: world.current_task().to_string(),
                    x: position.x.round() as i32,
                    y: position.y.round() as i32,
                    left: bounds.min.x.round() as i32,
                    top: bounds.min.y.round() as i32,
                    width: (bounds.max.x - bounds.min.x).round().max(0.0) as u32,
                    height: (bounds.max.y - bounds.min.y).round().max(0.0) as u32,
                    quiet: world.manners_active(),
                })
            }
            ControlCommand::Toys => ControlResponse::Toys(
                world
                    .toys()
                    .toys()
                    .iter()
                    .take(MAX_REPORTED_TOYS)
                    .map(|toy| ToyPosition {
                        id: toy.id.0,
                        kind: toy.kind,
                        x: toy.position.x.round() as i32,
                        y: toy.position.y.round() as i32,
                    })
                    .collect(),
            ),
            _ => ControlResponse::Err("UNSUPPORTED".into()),
        }
    }

    pub(crate) fn load_reload_config(
        command: ControlCommand,
        path: &std::path::Path,
    ) -> Result<Config, ConfigError> {
        let snapshot = ConfigSnapshot::load_existing(path)?;
        if let ControlCommand::ReloadIf(expected) = command {
            if snapshot.revision.reload_token(path)? != expected {
                return Err(ConfigError::Conflict);
            }
        }
        Ok(snapshot.config)
    }

    pub(crate) fn restart_required_reason(current: &Config, next: &Config) -> Option<String> {
        let changes = current.restart_required_changes(next);
        (!changes.is_empty()).then(|| changes.join(", "))
    }

    fn assert_frame(&self, frame: RuntimeFrame, phase: Phase) {
        assert_eq!(frame.id, self.frame_id, "stale runtime frame token");
        assert_eq!(self.phase, phase, "runtime phase ordering violation");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use honk_engine::{Vec2, World};

    #[test]
    fn conditional_reload_rejects_other_files_and_intervening_edits() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let revision = Config::default()
            .save_if_revision(&path, &Default::default())
            .unwrap();
        let command = ControlCommand::ReloadIf(revision.reload_token(&path).unwrap());
        assert!(RuntimeCore::load_reload_config(command, &path).is_ok());
        assert!(
            RuntimeCore::load_reload_config(command, &directory.path().join("other.toml")).is_err()
        );
        std::fs::write(&path, "# a concurrent edit\n").unwrap();
        assert!(matches!(
            RuntimeCore::load_reload_config(command, &path),
            Err(ConfigError::Conflict)
        ));
        assert!(RuntimeCore::load_reload_config(ControlCommand::Reload, &path).is_ok());
    }

    #[test]
    fn ordinary_reload_rejects_a_removed_config_instead_of_applying_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let mut config = Config::default();
        config.audio.enabled = false;
        config.save_atomic(&path).unwrap();
        let reloaded = RuntimeCore::load_reload_config(ControlCommand::Reload, &path).unwrap();
        assert!(!reloaded.audio.enabled);
        std::fs::remove_file(&path).unwrap();
        assert!(RuntimeCore::load_reload_config(ControlCommand::Reload, &path).is_err());
    }

    #[test]
    fn platform_runtimes_share_clock_tick_and_damage_ordering() {
        let mut world = World::new(Rect::new(Vec2::ZERO, Vec2::new(1280.0, 720.0)), 7);
        for _ in 0..1_200 {
            world.tick();
            if world.visual_bounds().is_some() {
                break;
            }
        }
        assert!(
            world.visual_bounds().is_some(),
            "startup should walk on-stage"
        );
        let mut core = RuntimeCore::new();
        let start = core.last_frame_time;
        let frame = core.begin_at(start + 1.0 / 60.0);
        core.tick(&mut world, frame);
        assert!(world.now() > 0.0);
        assert!(core.damage(&world, frame).is_some());
        core.acknowledge_present();
        assert_eq!(core.phase, Phase::Idle);
    }

    #[test]
    #[should_panic(expected = "runtime phase ordering violation")]
    fn damage_cannot_run_before_fixed_step_tick() {
        let mut core = RuntimeCore::new();
        let world = World::new(Rect::new(Vec2::ZERO, Vec2::new(1280.0, 720.0)), 7);
        let frame = core.begin_at(core.last_frame_time + 1.0 / 60.0);
        let _ = core.damage(&world, frame);
    }

    #[test]
    fn restart_required_config_is_identified_before_assignment() {
        let current = Config::default();
        let mut next = current.clone();
        next.platform.wayland = !current.platform.wayland;
        assert_eq!(
            RuntimeCore::restart_required_reason(&current, &next).as_deref(),
            Some("platform.wayland")
        );
    }

    #[test]
    fn next_tick_delay_paces_the_loop_to_one_hundred_twenty_hz() {
        let core = RuntimeCore::new();
        let tick = honk_engine::DT as f64;
        let start = core.last_frame_time;

        let initial = core.next_tick_delay_at(start).as_secs_f64();
        assert!((initial - tick).abs() < 1e-6);

        let partial = core.next_tick_delay_at(start + tick * 0.25).as_secs_f64();
        assert!((partial - tick * 0.75).abs() < 1e-6);
        assert_eq!(core.next_tick_delay_at(start + tick * 2.0), Duration::ZERO);
    }

    #[test]
    fn pacing_accounts_for_remainder_and_frame_work() {
        let mut core = RuntimeCore::new();
        let tick = honk_engine::DT as f64;
        let start = core.last_frame_time;
        core.accumulator.pump(tick * 0.7);
        let remaining = core.next_tick_delay_at(start + tick * 0.1).as_secs_f64();
        assert!((remaining - tick * 0.2).abs() < 1e-6);
        assert_eq!(core.next_tick_delay_at(start + tick), Duration::ZERO);
        core.accumulator.pump(3600.0);
        assert!((core.next_tick_delay_at(start).as_secs_f64() - tick).abs() < 1e-6);
    }

    #[test]
    fn a_deferred_transparent_clear_is_retried_before_stop_completes() {
        let mut world = World::new(Rect::new(Vec2::ZERO, Vec2::new(1280.0, 720.0)), 8);
        let mut core = RuntimeCore::new();
        core.last_visual_bounds = Some(Rect::new(Vec2::ZERO, Vec2::new(100.0, 100.0)));
        world.request_graceful_exit();
        for _ in 0..120 * 30 {
            world.tick();
            if world.graceful_exit_complete() {
                break;
            }
        }
        assert!(world.graceful_exit_complete());
        let mut now = core.last_frame_time + 1.0;
        for _ in 0..4 {
            let frame = core.begin_at(now);
            core.tick(&mut world, frame);
            assert!(core.damage(&world, frame).is_some());
            core.defer_present();
            assert!(!core.graceful_stop_complete(&world));
            now += 1.0 / 60.0;
        }
        let frame = core.begin_at(now);
        core.tick(&mut world, frame);
        assert!(core.damage(&world, frame).is_some());
        core.acknowledge_present();
        assert!(core.graceful_stop_complete(&world));
    }

    #[test]
    fn runtime_stop_waits_for_the_shared_offscreen_exit() {
        let mut world = World::new(Rect::new(Vec2::ZERO, Vec2::new(1280.0, 720.0)), 8);
        for _ in 0..1_200 {
            world.tick();
            if world.visual_bounds().is_some() {
                break;
            }
        }
        world
            .goose
            .foot_marks
            .add(world.goose.position, world.now());
        let mut core = RuntimeCore::new();
        let mut now = core.last_frame_time + 1.0 / 60.0;
        let frame = core.begin_at(now);
        core.tick(&mut world, frame);
        assert!(core.damage(&world, frame).is_some());
        core.acknowledge_present();

        RuntimeCore::begin_graceful_stop(&mut world);
        assert!(world.graceful_exit_requested());
        assert!(!core.graceful_stop_complete(&world));
        let mut saw_unacknowledged_clear = false;
        for _ in 0..(120 * 20) {
            now += honk_engine::DT as f64;
            let frame = core.begin_at(now);
            core.tick(&mut world, frame);
            if core.damage(&world, frame).is_some() {
                if world.visual_bounds().is_none() {
                    assert!(world.graceful_exit_complete());
                    assert!(
                        !core.graceful_stop_complete(&world),
                        "transparent clear is not complete before overlay.present succeeds"
                    );
                    saw_unacknowledged_clear = true;
                }
                core.acknowledge_present();
            }
            if core.graceful_stop_complete(&world) {
                break;
            }
        }
        assert!(saw_unacknowledged_clear);
        assert!(core.graceful_stop_complete(&world));
    }
}
