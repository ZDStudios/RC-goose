//! The simulation world: owns the goose and drives it through the task state machine.
//!
//! A scripted **FirstUX** intro runs once (the goose walks on-stage and introduces itself),
//! then the default roaming state picks a random *pickable* task via the biased
//! [`Deck`](crate::rng::Deck). Tasks set targets/params; [`crate::locomotion`] moves the
//! goose; the gait + footmark logic here is mechanical.

use crate::autumn::{AutumnState, LEAF_RENDER_RADIUS};
use crate::collect_window::{
    CollectWindowCloseEvent, CollectWindowCloseOrigin, CollectWindowCommand, CollectWindowKind,
    CollectWindowSnapshot,
};
use crate::command::{PokeAction, PokeOutcome};
use crate::cursor::{CursorCommand, WorldOptions};
use crate::foreign_window::ForeignWindowSnapshot;
use crate::hearts::Hearts;
use crate::interaction::{PatTracker, Pointer};
use crate::layout::DesktopLayout;
use crate::locomotion;
use crate::math::{Rect, Vec2};
use crate::mood::{LocalHour, LocalTime, MoodKind, MoodMachine, ZParticles};
use crate::render::RenderPalette;
use crate::rig::{GoosePose, Rig, RigAnim, RigInput};
use crate::rng::{Deck, RandomSource, SplitMix64};
use crate::schedule::PresenceSnapshot;
use crate::sound::Sound;
use crate::task::{
    AffectionFollowTask, AnnoyedReactionTask, AutumnLeafPileTask, CollectWindowTask, EdgeEntryTask,
    EdgeWrapTask, ExcursionKind, ExcursionTask, FirstUxTask, GracefulExitTask, HyperTask,
    NabMouseTask, PerchRideTask, PermissionWaitTask, Task, TaskCtx, ToyPlayTask, WalkToTask,
    WanderTask,
};
use crate::time::DT;
use crate::toys::{ToyId, ToyKind, ToyState, TOY_RENDER_RADIUS};
use std::collections::VecDeque;

/// Distance travelled per full walking-gait cycle (radians of `gait_phase` per `TAU`).
const GAIT_CYCLE_DISTANCE: f32 = 22.0;
const SECOND_HOURLY_HONK_DELAY: f64 = 0.35;
const LIFECYCLE_EDGE_MARGIN: f32 = 220.0;
const LIFECYCLE_EDGE_INSET: f32 = 56.0;
const USER_CLOSE_REACTION_PROBABILITY: f64 = 0.30;
/// Per-tick chance that a wandering goose returns to a lying-around toy (about every 5 s).
const TOY_IDLE_PLAY_CHANCE: f64 = 1.0 / 600.0;

/// The whole simulation: one goose roaming within `bounds` (the virtual-desktop space).
pub struct World {
    pub goose: GooseEntity,
    /// Outer union bounds retained for compatibility and off-screen excursion edges.
    pub bounds: Rect,
    /// Actual visible monitor regions used for targets, clipping, and hotplug recovery.
    layout: DesktopLayout,
    rng: SplitMix64,
    /// Dedicated stream so close-reaction probability is stable across unrelated animation RNG.
    reaction_rng: SplitMix64,
    current: Box<dyn Task>,
    /// Factories for the randomly-pickable roaming tasks (the original's `TaskDatabase`).
    pickable: Vec<fn() -> Box<dyn Task>>,
    /// Shuffle-bag over `pickable` indices (no repeats until exhausted).
    deck: Deck<SplitMix64>,
    elapsed: f64,
    /// Sound requests produced this tick, drained by the platform audio backend.
    pending_sounds: Vec<Sound>,
    /// Cursor requests produced this tick, drained by the platform backend.
    pending_cursor_commands: Vec<CursorCommand>,
    /// Collect-window requests produced this tick, drained by the platform backend.
    pending_collect_window_commands: Vec<CollectWindowCommand>,
    /// Runtime options/capabilities that must stay platform-free.
    options: WorldOptions,
    /// Detects pats from hovering cursor sweeps and tracks the happy/calm streak (M6 §5.9).
    pat: PatTracker,
    /// Heart particles emitted while being patted.
    hearts: Hearts,
    /// Sleepy-mood Z particles.
    sleepies: ZParticles,
    /// Dynamic mood state machine.
    mood: MoodMachine,
    /// Built-in Autumn leaf-pile state.
    autumn: AutumnState,
    /// Remote-control toys (ADR 0059).
    toys: ToyState,
    /// Latest runtime-sampled local time, if the platform has provided one.
    local_time: Option<LocalTime>,
    /// Latest platform-reported DND/fullscreen presence state.
    presence: PresenceSnapshot,
    /// Last schedule manners state used to build the pickable task deck.
    last_manners_active: bool,
    /// Last Autumn pickable state used to build the pickable task deck.
    last_autumn_pickable: bool,
    /// Local hour that has already triggered its on-hour honks.
    last_hourly_honk: Option<LocalHour>,
    /// Pending second honk for the current on-hour double honk.
    second_hourly_honk_at: Option<f64>,
    /// Last pointer state fed in via [`World::set_pointer`].
    pointer: Pointer,
    /// Last platform-reported user-dragged foreign window.
    dragged_window: Option<ForeignWindowSnapshot>,
    /// Last platform-reported controlled collect-window state.
    collect_window_snapshot: Option<CollectWindowSnapshot>,
    /// Left button held on the previous pointer update (for click rising-edge detection).
    prev_left_down: bool,
    /// A click landed on the goose; the next tick installs the hyper burst.
    pending_hyper: bool,
    /// A click landed on the goose while mouse stealing is available; the next tick installs
    /// the nab task.
    pending_nab: bool,
    /// A smoke/manual collect action requested by the runtime.
    pending_collect: Option<CollectWindowKind>,
    /// The task that was running before a transient interrupt (hyper), restored when it ends.
    interrupted: Option<Box<dyn Task>>,
    /// When the next long off-screen errand is due (ADR 0016).
    next_excursion_at: f64,
    /// When the next quick puddle hop (the mud source) is due (ADR 0016).
    next_puddle_at: f64,
    /// The current errand should chain a collect-window prank on return.
    excursion_prank: bool,
    /// Wander-path meander state (ADR 0016): phase, smoothed amplitude, its target,
    /// and when the amplitude re-rolls.
    meander_phase: f32,
    meander_amp: f32,
    meander_amp_target: f32,
    next_meander_shift: f64,
    /// A user-close reaction should chain the existing bounded cursor prank after its visible beat.
    close_reaction_nab: bool,
    /// Ordered, typed close evidence. Lingering props can close while another request is active.
    pending_collect_closes: VecDeque<CollectWindowCloseEvent>,
    /// Bounded one-shot key history protects against a backend repeating the same dead snapshot.
    observed_collect_closes: VecDeque<CollectWindowCloseEvent>,
    /// Lifecycle intent is latched independently from the active task so permission/topology
    /// transitions can never cancel an in-progress terminal exit.
    graceful_exit_requested: bool,
}

use crate::entity::GooseEntity;

impl World {
    fn staged_entry(layout: &DesktopLayout, rng: &mut impl RandomSource) -> (Vec2, Vec2) {
        let edge = layout.sample_exposed_edge(rng, Some(0));
        let primary = layout.regions()[0];
        let primary_center = (primary.min + primary.max) * 0.5;
        let boundary = edge.point_near(primary_center, LIFECYCLE_EDGE_INSET);
        let target =
            layout.clamp_point(boundary + edge.direction().inward() * LIFECYCLE_EDGE_MARGIN);
        (
            boundary + edge.direction().outward() * LIFECYCLE_EDGE_MARGIN,
            target,
        )
    }

    /// A world bounded by `bounds`, with the goose entering through a real exposed edge for the
    /// FirstUX intro. `seed` makes the whole simulation deterministic.
    pub fn new(bounds: Rect, seed: u64) -> Self {
        Self::with_options(bounds, seed, WorldOptions::default())
    }

    /// Build a world with explicit runtime options/capabilities.
    pub fn with_options(bounds: Rect, seed: u64, options: WorldOptions) -> Self {
        Self::with_layout_and_options(DesktopLayout::single(bounds), seed, options)
    }

    /// Build a world from real monitor regions with default runtime options.
    pub fn with_layout(layout: DesktopLayout, seed: u64) -> Self {
        Self::with_layout_and_options(layout, seed, WorldOptions::default())
    }

    /// Build a world from real monitor regions and explicit runtime options.
    pub fn with_layout_and_options(
        layout: DesktopLayout,
        seed: u64,
        options: WorldOptions,
    ) -> Self {
        let bounds = layout.bounds();
        let mut rng = SplitMix64::seed(seed);
        let (startup_position, startup_target) = Self::staged_entry(&layout, &mut rng);
        let mut goose = GooseEntity::new();
        goose.parameters = options.parameters;
        // Enter through a genuinely exposed edge on the first monitor. Shared monitor seams
        // are excluded, so startup never materializes on top of another real display.
        goose.position = startup_position;
        goose.target_pos = startup_target;
        goose.current_speed = goose.parameters.walk_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.anim = RigAnim::new(goose.position, goose.direction);
        goose.pose =
            goose
                .anim
                .update(&RigInput::static_pose(goose.position, goose.direction, 0.0));
        goose.rig = goose.pose.primary;

        let mood = MoodMachine::new(0.0, options.mood, &mut rng);
        let presence = PresenceSnapshot::default();
        let local_time = None;
        let last_manners_active = Self::manners_active_for(options, local_time, presence);
        let last_autumn_pickable = false;
        let pickable = Self::pickable_for(
            options,
            mood.current(),
            last_manners_active,
            last_autumn_pickable,
        );
        let deck = Deck::new(pickable.len(), SplitMix64::seed(seed ^ 0x9E37_79B9));

        // First excursion/puddle cadences are drawn up front so the goose settles in
        // before its first off-screen trip (ADR 0016).
        let t = options.timing;
        let next_excursion_at = rng.range(t.excursion_min_gap, t.excursion_max_gap) as f64;
        let next_puddle_at = rng.range(t.puddle_min_gap, t.puddle_max_gap) as f64;

        Self {
            goose,
            bounds,
            layout,
            rng,
            reaction_rng: SplitMix64::seed(seed ^ 0xD1B5_4A32_D192_ED03),
            current: Box::new(FirstUxTask::entering(startup_target)), // scripted intro runs first
            pickable,
            deck,
            elapsed: 0.0,
            pending_sounds: Vec::new(),
            pending_cursor_commands: Vec::new(),
            pending_collect_window_commands: Vec::new(),
            options,
            pat: PatTracker::new(),
            hearts: Hearts::new(),
            sleepies: ZParticles::new(),
            mood,
            autumn: AutumnState::new(),
            toys: ToyState::new(),
            local_time,
            presence,
            last_manners_active,
            last_autumn_pickable,
            last_hourly_honk: None,
            second_hourly_honk_at: None,
            pointer: Pointer::default(),
            dragged_window: None,
            collect_window_snapshot: None,
            prev_left_down: false,
            pending_hyper: false,
            next_excursion_at,
            next_puddle_at,
            excursion_prank: false,
            meander_phase: 0.0,
            meander_amp: 0.0,
            meander_amp_target: 0.0,
            next_meander_shift: 0.0,
            close_reaction_nab: false,
            pending_collect_closes: VecDeque::new(),
            observed_collect_closes: VecDeque::new(),
            graceful_exit_requested: false,
            pending_nab: false,
            pending_collect: None,
            interrupted: None,
        }
    }

    /// The world's monotonic clock (seconds), the time base for footmark fade.
    pub fn now(&self) -> f64 {
        self.elapsed
    }

    /// Current monitor-region topology.
    pub fn layout(&self) -> &DesktopLayout {
        &self.layout
    }

    /// Reconcile a live display-topology change without leaving active targets in removed
    /// monitors or gaps. Transient desktop-control tasks are cancelled safely; ordinary
    /// animation/task state survives with its position and target clamped to visible space.
    pub fn apply_layout(&mut self, layout: DesktopLayout) {
        let was_exiting = self.graceful_exit_requested();
        let was_permission_waiting = self.permission_waiting();
        let was_collecting = self.is_collect_window_active();
        if was_collecting && !was_exiting && !was_permission_waiting {
            self.abandon_collect_window();
        } else if !was_exiting
            && !was_permission_waiting
            && matches!(
                self.current.id(),
                "edge_wrap" | "excursion" | "nab_mouse" | "perch_ride"
            )
        {
            self.resume_or_wander();
        }

        self.bounds = layout.bounds();
        self.layout = layout;
        let permission_anchor = self.layout.clamp_point(self.goose.target_pos);
        let still_visible = self
            .layout
            .clip_rect(self.goose.pose.bounding_box().grow(3.0))
            .is_some();
        if still_visible {
            self.goose.target_pos = self.layout.clamp_point(self.goose.target_pos);
            if was_exiting {
                self.current = Box::new(GracefulExitTask::new());
            } else if was_permission_waiting {
                self.current = Box::new(PermissionWaitTask::new(permission_anchor));
            }
        } else {
            let (entry_position, entry_target) = Self::staged_entry(&self.layout, &mut self.rng);
            self.goose.position = entry_position;
            self.goose.target_pos = entry_target;
            self.goose.velocity = Vec2::ZERO;
            self.goose.anim = RigAnim::new(entry_position, self.goose.direction);
            self.goose.pose = self.goose.anim.update(&RigInput::static_pose(
                entry_position,
                self.goose.direction,
                0.45,
            ));
            self.goose.rig = self.goose.pose.primary;
            self.interrupted = None;
            self.current = if was_exiting {
                Box::new(GracefulExitTask::new())
            } else if was_permission_waiting {
                Box::new(PermissionWaitTask::new(permission_anchor))
            } else {
                Box::new(EdgeEntryTask::new(entry_target))
            };
        }
        if !self.layout.contains(self.pointer.pos) {
            self.pointer.present = false;
            self.prev_left_down = false;
        }
        self.dragged_window = None;
        self.collect_window_snapshot = None;
        self.pending_nab = false;
        self.pending_collect = None;
        self.excursion_prank = false;
        self.close_reaction_nab = false;
        self.autumn.clear();
        self.refresh_schedule_state();
        self.rebuild_pickable();
    }

    fn pickable_for(
        options: WorldOptions,
        mood: MoodKind,
        manners_active: bool,
        autumn_pickable: bool,
    ) -> Vec<fn() -> Box<dyn Task>> {
        // Ordinary deck weighting: four normal wander beats for each edge wrap. This keeps the
        // Pac-Man flourish occasional (20% of baseline deck draws) instead of every other task.
        let mut pickable: Vec<fn() -> Box<dyn Task>> = vec![
            || Box::new(WanderTask::new()) as Box<dyn Task>,
            || Box::new(WanderTask::new()) as Box<dyn Task>,
            || Box::new(WanderTask::new()) as Box<dyn Task>,
            || Box::new(WanderTask::new()) as Box<dyn Task>,
            || Box::new(EdgeWrapTask::new()) as Box<dyn Task>,
        ];
        if manners_active {
            return pickable;
        }
        // Spontaneous cursor attacks are original-parity `AttackRandomly` behavior and
        // stay OFF by default: without it, nabs come only from clicks and `do nab`.
        if options.mouse_steal.active() && options.mouse_steal.attack_randomly {
            pickable.push(|| Box::new(NabMouseTask::new()) as Box<dyn Task>);
        }
        if options.collect_window.active() {
            pickable.push(|| Box::new(CollectWindowTask::new()) as Box<dyn Task>);
        }
        if autumn_pickable {
            pickable.push(|| Box::new(AutumnLeafPileTask::new()) as Box<dyn Task>);
        }
        if mood == MoodKind::Mischievous {
            // Duplicates only tasks already in the deck: spontaneous nab stays gated
            // on `attack_randomly` even for a mischievous goose.
            if options.mouse_steal.active() && options.mouse_steal.attack_randomly {
                pickable.push(|| Box::new(NabMouseTask::new()) as Box<dyn Task>);
            }
            if options.collect_window.active() {
                pickable.push(|| Box::new(CollectWindowTask::new()) as Box<dyn Task>);
            }
        }
        pickable
    }

    fn rebuild_pickable(&mut self) {
        self.pickable = Self::pickable_for(
            self.options,
            self.mood.current(),
            self.manners_active(),
            self.autumn_pickable(),
        );
        self.deck = Deck::new(
            self.pickable.len(),
            SplitMix64::seed(
                self.elapsed.to_bits()
                    ^ ((self.pickable.len() as u64) << 32)
                    ^ 0xA076_1D64_78BD_642F,
            ),
        );
    }

    fn manners_active_for(
        options: WorldOptions,
        local_time: Option<LocalTime>,
        presence: PresenceSnapshot,
    ) -> bool {
        options.appearance.calm_goose || options.schedule.manners_active(local_time, presence)
    }

    /// Atomically apply a complete runtime option set from the control plane.
    pub fn apply_options(&mut self, options: WorldOptions) {
        self.options = options;
        self.goose.parameters = options.parameters;
        self.mood
            .apply_options(options.mood, self.elapsed, &mut self.rng);
        if !options.hourly_honk.on_hour_double_honk {
            self.second_hourly_honk_at = None;
        }
        if !self.autumn_active() {
            self.autumn.clear();
        }
        self.refresh_schedule_state();
        self.rebuild_pickable();

        if !options.interaction.pat_streak {
            self.pat = PatTracker::new();
            if self.current.id() == "affection_follow" {
                self.resume_or_wander();
            }
        }
        if self.is_cursor_mischief_active() && !options.mouse_steal.active() {
            self.resume_or_wander();
        }
        if self.is_perch_ride_active() && !options.foreign_window.watch_active() {
            self.dragged_window = None;
            self.resume_or_wander();
        }
        if self.is_collect_window_active()
            && (!options.collect_window.active()
                || self
                    .current
                    .collect_kind()
                    .is_some_and(|kind| !options.collect_window.kind_active(kind)))
        {
            self.abandon_collect_window();
        }
    }

    /// Replace all active work with a platform-neutral safe wait at `anchor`.
    pub fn enter_permission_wait(&mut self, anchor: Vec2) {
        if self.graceful_exit_requested() {
            return;
        }
        self.pending_cursor_commands.clear();
        self.pending_collect_window_commands.clear();
        if self.is_collect_window_active() {
            self.abandon_collect_window();
        } else {
            self.collect_window_snapshot = None;
        }

        self.current = Box::new(PermissionWaitTask::new(anchor));
        self.interrupted = None;
        self.pending_sounds.clear();
        self.pending_hyper = false;
        self.pending_nab = false;
        self.pending_collect = None;
        self.dragged_window = None;
        self.excursion_prank = false;
        self.close_reaction_nab = false;
        self.pending_collect_closes.clear();
        self.second_hourly_honk_at = None;
        self.pat = PatTracker::new();
        self.hearts = Hearts::new();
        self.sleepies = ZParticles::new();
        self.goose.track_mud_end_time = self.elapsed;
        self.goose.foot_marks = Default::default();
        self.autumn.clear();
        self.toys.clear();
    }

    /// Move the permission-wait anchor after a display-topology change.
    pub fn update_permission_wait_anchor(&mut self, anchor: Vec2) {
        self.current.set_permission_wait_anchor(anchor);
    }

    /// Leave the safe wait and restart the normal first-run introduction.
    pub fn leave_permission_wait(&mut self) {
        if self.graceful_exit_requested() {
            return;
        }
        self.current = Box::new(FirstUxTask::new());
        self.interrupted = None;
        self.pending_hyper = false;
        self.pending_nab = false;
        self.pending_collect = None;
    }

    /// Whether the world is currently holding at a permission-wait anchor.
    pub fn permission_waiting(&self) -> bool {
        self.current.id() == "permission_wait"
    }

    /// Apply a live CLI/TUI poke to the world without exposing OS details to the engine.
    pub fn poke(&mut self, action: PokeAction) -> PokeOutcome {
        if self.graceful_exit_requested() {
            return PokeOutcome::Busy;
        }
        if self.permission_waiting() && action != PokeAction::Honk {
            return PokeOutcome::Busy;
        }
        match action {
            PokeAction::Honk => {
                self.pending_sounds.push(Sound::honk());
                PokeOutcome::Applied
            }
            PokeAction::Mud => {
                self.goose.track_mud_end_time =
                    self.elapsed + self.goose.parameters.duration_to_track_mud as f64;
                PokeOutcome::Applied
            }
            PokeAction::Wander => {
                if self.is_collect_window_active() {
                    return PokeOutcome::Busy;
                }
                self.pending_hyper = false;
                self.pending_nab = false;
                self.pending_collect = None;
                self.interrupted = None;
                self.current = Box::new(WanderTask::new());
                PokeOutcome::Applied
            }
            PokeAction::Meme => self.poke_collect(CollectWindowKind::Meme),
            PokeAction::Note => self.poke_collect(CollectWindowKind::Note),
            PokeAction::Nab => {
                if !self.options.mouse_steal.active() {
                    return PokeOutcome::Unsupported;
                }
                if self.is_cursor_mischief_active()
                    || self.is_perch_ride_active()
                    || self.is_collect_window_active()
                    || self.interrupted.is_some()
                {
                    return PokeOutcome::Busy;
                }
                self.pending_hyper = false;
                self.pending_nab = true;
                PokeOutcome::Applied
            }
        }
    }

    /// Apply a remote-control poke. Unlike local CLI pokes, remote requests also respect the
    /// schedule/presence manners: while quiet, only `Wander` (calm down) is accepted.
    pub fn remote_poke(&mut self, action: PokeAction) -> PokeOutcome {
        if action != PokeAction::Wander && self.manners_active() && !self.remote_blocked() {
            return PokeOutcome::Quiet;
        }
        self.poke(action)
    }

    /// Walk to a requested desktop point, clamped into a real monitor (remote control).
    pub fn walk_to(&mut self, point: Vec2) -> PokeOutcome {
        if let Some(outcome) = self.remote_gate() {
            return outcome;
        }
        if self.is_cursor_mischief_active()
            || self.is_perch_ride_active()
            || self.is_collect_window_active()
        {
            return PokeOutcome::Busy;
        }
        self.pending_hyper = false;
        self.pending_nab = false;
        self.pending_collect = None;
        self.interrupted = None;
        self.current = Box::new(WalkToTask::new(self.layout.clamp_point(point)));
        PokeOutcome::Applied
    }

    /// Place a toy for the goose to play with (remote control). A goose that is only wandering
    /// or playing goes straight for it; busier errands finish first.
    pub fn place_toy(&mut self, kind: ToyKind, point: Vec2) -> Result<ToyId, PokeOutcome> {
        if let Some(outcome) = self.remote_gate() {
            return Err(outcome);
        }
        let position = self.layout.clamp_point(point);
        let id = self
            .toys
            .place(self.elapsed, kind, position)
            .ok_or(PokeOutcome::Busy)?;
        if matches!(self.current.id(), "wander" | "toy_play" | "walk_to") {
            self.toys.release_all();
            self.interrupted = None;
            self.current = Box::new(ToyPlayTask::targeting(id));
        }
        Ok(id)
    }

    /// Move a toy that is not in the goose's beak (remote control). `Unsupported` means it is
    /// gone or being carried.
    pub fn move_toy(&mut self, id: ToyId, point: Vec2) -> PokeOutcome {
        if let Some(outcome) = self.remote_gate() {
            return outcome;
        }
        let position = self.layout.clamp_point(point);
        if self.toys.move_toy(id, position) {
            PokeOutcome::Applied
        } else {
            PokeOutcome::Unsupported
        }
    }

    /// The live remote-control toys (for the renderer and status).
    pub fn toys(&self) -> &ToyState {
        &self.toys
    }

    /// Current body position in desktop coordinates (for remote status).
    pub fn goose_position(&self) -> Vec2 {
        self.goose.position
    }

    fn remote_blocked(&self) -> bool {
        self.graceful_exit_requested() || self.permission_waiting()
    }

    fn remote_gate(&self) -> Option<PokeOutcome> {
        if self.remote_blocked() {
            Some(PokeOutcome::Busy)
        } else if self.manners_active() {
            Some(PokeOutcome::Quiet)
        } else {
            None
        }
    }

    /// Take the sound requests produced since the last call (for the audio backend).
    pub fn take_sounds(&mut self) -> Vec<Sound> {
        std::mem::take(&mut self.pending_sounds)
    }

    /// Take cursor commands emitted since the last call (for the platform backend).
    pub fn take_cursor_commands(&mut self) -> Vec<CursorCommand> {
        std::mem::take(&mut self.pending_cursor_commands)
    }

    /// Take collect-window commands emitted since the last call.
    pub fn take_collect_window_commands(&mut self) -> Vec<CollectWindowCommand> {
        std::mem::take(&mut self.pending_collect_window_commands)
    }

    /// Reflect a backend capability change after startup, e.g. cursor warp failed.
    pub fn set_cursor_warp_supported(&mut self, supported: bool) {
        self.options.mouse_steal.warp_supported = supported;
        if !supported {
            self.pending_cursor_commands.clear();
            if self.is_cursor_mischief_active() {
                self.resume_or_wander();
            }
        }
        self.rebuild_pickable();
    }

    /// Reflect a backend capability change after startup, e.g. move-size hook setup failed.
    pub fn set_foreign_window_watch_supported(&mut self, supported: bool) {
        self.options.foreign_window.capabilities.watch_drag = supported;
        if !supported {
            self.dragged_window = None;
            if self.is_perch_ride_active() {
                self.resume_or_wander();
            }
        }
        self.rebuild_pickable();
    }

    /// Reflect backend collect-window movement/spawn/input capability changes.
    pub fn set_collect_window_supported(&mut self, supported: bool) {
        self.options.collect_window.capabilities.spawn_note = supported;
        self.options.collect_window.capabilities.spawn_image = supported;
        self.options.collect_window.capabilities.move_window = supported;
        self.options.collect_window.capabilities.set_passthrough = supported;
        self.options.collect_window.capabilities.synthesize_text = supported;
        if !supported {
            if self.is_collect_window_active() {
                self.abandon_collect_window();
            } else {
                self.collect_window_snapshot = None;
            }
        }
        self.rebuild_pickable();
    }

    /// Feed one frame of foreign-window drag state in world/desktop coordinates.
    pub fn set_foreign_window_drag(&mut self, dragged_window: Option<ForeignWindowSnapshot>) {
        self.dragged_window = dragged_window;
    }

    /// Feed one frame of controlled collect-window state in world/desktop coordinates.
    pub fn set_collect_window_snapshot(
        &mut self,
        collect_window_snapshot: Option<CollectWindowSnapshot>,
    ) {
        let active = self.current.collect_request();
        match collect_window_snapshot {
            Some(snapshot) if !snapshot.alive => {
                if let Some(event) = CollectWindowCloseEvent::from_dead_snapshot(snapshot) {
                    let duplicate = self.observed_collect_closes.iter().any(|queued| {
                        queued.id == event.id
                            && queued.request == event.request
                            && queued.kind == event.kind
                            && queued.origin == event.origin
                    });
                    if !duplicate {
                        self.observed_collect_closes.push_back(event);
                        if self.observed_collect_closes.len() > 64 {
                            self.observed_collect_closes.pop_front();
                        }
                        self.pending_collect_closes.push_back(event);
                    }
                }
                if active.is_some_and(|(request, kind)| {
                    request == snapshot.request && kind == snapshot.kind
                }) {
                    // Matching dead evidence reaches the active task. An unrelated lingering
                    // close is queued without replacing the active request's live snapshot.
                    self.collect_window_snapshot = Some(snapshot);
                } else if active.is_none() {
                    self.collect_window_snapshot = None;
                }
            }
            Some(snapshot) => {
                if active.is_none()
                    || active.is_some_and(|(request, kind)| {
                        request == snapshot.request && kind == snapshot.kind
                    })
                {
                    self.collect_window_snapshot = Some(snapshot);
                }
            }
            None => self.collect_window_snapshot = None,
        }
    }

    /// Force a collect-window action for smoke tests before M10/M11 public pokes exist.
    pub fn force_collect_window(&mut self, kind: CollectWindowKind) {
        if !self.permission_waiting()
            && self.options.collect_window.capacity_available
            && self.options.collect_window.kind_active(kind)
        {
            self.pending_collect = Some(kind);
        }
    }

    /// A full owned-window set blocks new work, without interrupting the last
    /// admitted delivery or permanently disabling a healthy backend.
    pub fn set_collect_window_capacity(&mut self, available: bool) {
        self.options.collect_window.capacity_available = available;
    }

    /// Owned toplevels may be supported without global positioning (Wayland).
    /// This does not grant any foreign-window or pointer capability.
    pub fn set_collect_window_positioning(&mut self, animated: bool) {
        if !animated
            && self.options.collect_window.capabilities.move_window
            && self.is_collect_window_active()
        {
            self.pending_collect_window_commands
                .retain(|command| !matches!(command, CollectWindowCommand::Move { .. }));
            self.abandon_collect_window();
        }
        self.options.collect_window.capabilities.move_window = animated;
        self.options.collect_window.capabilities.set_passthrough = animated;
    }

    fn poke_collect(&mut self, kind: CollectWindowKind) -> PokeOutcome {
        if !self.options.collect_window.kind_active(kind) {
            return PokeOutcome::Unsupported;
        }
        if !self.options.collect_window.capacity_available
            || self.is_cursor_mischief_active()
            || self.is_perch_ride_active()
            || self.is_collect_window_active()
            || self.interrupted.is_some()
        {
            return PokeOutcome::Busy;
        }
        self.pending_collect = Some(kind);
        PokeOutcome::Applied
    }

    /// The live heart particles (for the renderer).
    pub fn hearts(&self) -> &Hearts {
        &self.hearts
    }

    /// The live sleepy Z particles (for the renderer).
    pub fn sleepies(&self) -> &ZParticles {
        &self.sleepies
    }

    /// Runtime render palette from config.
    pub fn render_palette(&self) -> RenderPalette {
        self.options.palette
    }

    /// World-space bounds of pixels that can be visible in the **current** frame.
    ///
    /// This never contains prior-frame damage. Callers store this value, derive damage with
    /// [`World::damage_bounds`], present that damage, then retain this current value for the
    /// next frame. A fully off-desktop/transparent scene is `None`.
    pub fn visual_bounds(&self) -> Option<Rect> {
        let mut rect = None;
        let mut add = |candidate: Rect| {
            if let Some(visible) = self.layout.clip_rect(candidate) {
                rect = Some(rect.map_or(visible, |current: Rect| current.union(visible)));
            }
        };

        // Keep the complete projected anatomy inside the current bounds.
        add(self.goose.pose.bounding_box().grow(3.0));
        for (mark, scale) in self
            .goose
            .foot_marks
            .active_with_timing(self.elapsed, self.footmark_timing())
        {
            if scale > 0.0 {
                add(Rect::new(mark.position, mark.position).grow(5.0 * scale + 5.0));
            }
        }
        for (pos, alpha) in self.hearts.active(self.elapsed) {
            if alpha > 0.0 {
                add(Rect::new(pos, pos).grow(15.0));
            }
        }
        for (pos, alpha) in self.sleepies.active(self.elapsed) {
            if alpha > 0.0 {
                add(Rect::new(pos, pos).grow(17.0));
            }
        }
        for pile in self.autumn.piles() {
            let spawn = pile.spawn_scale(self.elapsed);
            if 1.0 - pile.fade_out(self.elapsed) <= 0.0 {
                continue;
            }
            add(Rect::new(pile.position, pile.position)
                .grow(pile.radius * spawn + LEAF_RENDER_RADIUS));
            for leaf in &pile.leaves {
                let pos = pile.position + leaf.screen_offset() * spawn;
                add(Rect::new(pos, pos).grow(LEAF_RENDER_RADIUS));
            }
        }
        for toy in self.toys.toys() {
            add(Rect::new(toy.position, toy.position).grow(TOY_RENDER_RADIUS));
        }
        rect.map(Rect::pixel_aligned)
    }

    /// Union previous and current visual bounds into the one-shot region to clear/present.
    pub fn damage_bounds(previous: Option<Rect>, current: Option<Rect>) -> Option<Rect> {
        match (previous, current) {
            (Some(previous), Some(current)) => Some(previous.union(current).pixel_aligned()),
            (Some(rect), None) | (None, Some(rect)) => Some(rect.pixel_aligned()),
            (None, None) => None,
        }
    }

    /// Runtime footmark timing from config.
    pub fn footmark_timing(&self) -> crate::footmarks::FootMarkTiming {
        self.options.footmarks
    }

    /// The current dynamic mood.
    pub fn mood(&self) -> MoodKind {
        self.mood.current()
    }

    /// Feed the current local time. Platform runtimes own local-time sampling.
    pub fn set_local_time(&mut self, local_time: LocalTime) {
        self.local_time = Some(local_time);
        self.refresh_schedule_state();
    }

    /// Feed the latest platform presence state (DND/fullscreen/presentation).
    pub fn set_presence(&mut self, presence: PresenceSnapshot) {
        self.presence = presence;
        self.refresh_schedule_state();
    }

    /// Whether quiet-hours or OS presence manners are currently calming the goose.
    pub fn manners_active(&self) -> bool {
        Self::manners_active_for(self.options, self.local_time, self.presence)
    }

    /// The live built-in Autumn leaf state (for the renderer).
    pub fn autumn(&self) -> &AutumnState {
        &self.autumn
    }

    /// Whether the goose is currently in its post-pat calm window.
    pub fn is_calm(&self) -> bool {
        self.pat.is_calm(self.elapsed)
    }

    /// Whether the world-space `point` is over the goose (its rig bounding box; plan §6).
    pub fn goose_hit(&self, point: Vec2) -> bool {
        self.goose.rig.bounding_box().contains(point)
    }

    /// Whether the active task is controlling the real cursor.
    pub fn is_cursor_mischief_active(&self) -> bool {
        self.current.id() == "nab_mouse"
    }

    /// Whether the active task is reacting to a foreign-window drag.
    pub fn is_perch_ride_active(&self) -> bool {
        self.current.id() == "perch_ride"
    }

    /// Whether the active task is controlling a collected desktop window.
    pub fn is_collect_window_active(&self) -> bool {
        self.current.id() == "collect_window"
    }

    /// Feed one frame of pointer state (cursor + buttons, world space). Detects pats
    /// (hover sweeps → hearts + calm) and a click on the goose (→ a hyper burst next tick).
    pub fn set_pointer(&mut self, pointer: Pointer) {
        if self.graceful_exit_requested()
            || self.permission_waiting()
            || self.is_cursor_mischief_active()
            || self.is_perch_ride_active()
            || self.is_collect_window_active()
        {
            self.pat = PatTracker::new();
            self.pointer = pointer;
            self.prev_left_down = pointer.left_down;
            return;
        }

        // Whether the pointer is over the goose at all — this gates the click reaction.
        let on_goose = pointer.present && self.goose_hit(pointer.pos);
        // Patting (hearts/calm) is a separate interaction, gated by the pat-streak toggle.
        let hovering = self.options.interaction.pat_streak && on_goose && !pointer.left_down;

        // Pat = hovering hover-sweeps. Each registered pat spawns a heart above the goose.
        let pats = self.pat.update(hovering, pointer.pos, self.elapsed);
        if pats > 0 {
            self.goose.anim.pet();
            let head = self.goose.rig.neck_head;
            for _ in 0..pats.min(3) {
                let jitter = Vec2::new(self.rng.range(-7.0, 7.0), self.rng.range(-3.0, 3.0));
                self.hearts.add(head + jitter, self.elapsed);
            }
            self.pending_sounds.push(Sound::Pat);
        }

        if self.pat.take_follow_request()
            && self.interrupted.is_none()
            && self.current.id() == "wander"
            && !self.pending_nab
            && !self.pending_hyper
        {
            let prior = std::mem::replace(
                &mut self.current,
                Box::new(AffectionFollowTask::new(self.elapsed)),
            );
            self.interrupted = Some(prior);
            self.goose.anim.pet();
        }

        // Click = left-button rising edge while over the goose → a hyper burst on the next tick.
        // Independent of the pat streak so disabling pats never disables the click reaction.
        let clicked = on_goose && pointer.left_down && !self.prev_left_down;
        if clicked && !self.permission_waiting() {
            if self.options.mouse_steal.active() {
                self.pending_nab = true;
            } else {
                self.pending_hyper = true;
            }
        }

        self.prev_left_down = pointer.left_down;
        self.pointer = pointer;
    }

    /// Interrupt the current task with a hyper burst, saving the prior task to resume later.
    fn start_hyper(&mut self) {
        if self.permission_waiting()
            || self.current.id() == "hyper"
            || self.is_collect_window_active()
            || self.interrupted.is_some()
        {
            return; // already mid-burst; don't stack
        }
        let prev = std::mem::replace(&mut self.current, Box::new(HyperTask::new()));
        self.interrupted = Some(prev);
    }

    /// Interrupt the current task with a cursor nab, saving the prior task to resume later.
    fn start_nab(&mut self) {
        if self.permission_waiting()
            || self.current.id() == "nab_mouse"
            || self.is_collect_window_active()
            || self.interrupted.is_some()
        {
            return; // already stealing the cursor
        }
        let prev = std::mem::replace(&mut self.current, Box::new(NabMouseTask::new()));
        self.interrupted = Some(prev);
    }

    /// Interrupt the current task with a forced collect-window task.
    fn start_collect_window(&mut self, kind: CollectWindowKind) {
        if self.permission_waiting()
            || self.current.id() == "collect_window"
            || self.interrupted.is_some()
        {
            return; // do not stack long-running desktop-mischief tasks
        }
        let prev = std::mem::replace(&mut self.current, Box::new(CollectWindowTask::forced(kind)));
        self.interrupted = Some(prev);
    }

    /// Interrupt the current task with a foreign-window perch/ride.
    fn start_perch_ride(&mut self) {
        if self.permission_waiting()
            || self.current.id() == "perch_ride"
            || self.interrupted.is_some()
        {
            return; // do not stack transient interrupts
        }
        let prev = std::mem::replace(&mut self.current, Box::new(PerchRideTask::new()));
        self.interrupted = Some(prev);
    }

    fn resume_or_wander(&mut self) {
        self.current = self
            .interrupted
            .take()
            .unwrap_or_else(|| Box::new(WanderTask::new()));
    }

    fn abandon_collect_window(&mut self) {
        if let Some(snapshot) = self
            .collect_window_snapshot
            .filter(|snapshot| snapshot.alive)
        {
            self.pending_collect_window_commands
                .push(CollectWindowCommand::SetPassthrough {
                    id: snapshot.id,
                    passthrough: false,
                });
            if snapshot.kind == CollectWindowKind::Meme {
                self.pending_collect_window_commands
                    .push(CollectWindowCommand::Close { id: snapshot.id });
            }
        }
        self.collect_window_snapshot = None;
        self.pending_collect = None;
        self.resume_or_wander();
    }

    /// The id of the currently running task (e.g. `"first_ux"`, `"wander"`).
    pub fn current_task(&self) -> &'static str {
        self.current.id()
    }

    /// Replace active behavior with a safe walk through the nearest exposed desktop edge.
    /// The runtime must continue ticking/presenting until [`Self::graceful_exit_complete`].
    pub fn request_graceful_exit(&mut self) {
        if self.graceful_exit_requested() {
            return;
        }
        self.graceful_exit_requested = true;
        if self.is_collect_window_active() {
            self.abandon_collect_window();
        }
        self.pending_cursor_commands.clear();
        self.pending_collect = None;
        self.pending_hyper = false;
        self.pending_nab = false;
        self.interrupted = None;
        self.dragged_window = None;
        self.collect_window_snapshot = None;
        self.excursion_prank = false;
        self.close_reaction_nab = false;
        self.pending_collect_closes.clear();
        self.second_hourly_honk_at = None;
        self.pat = PatTracker::new();
        self.hearts = Hearts::new();
        self.sleepies = ZParticles::new();
        self.goose.track_mud_end_time = self.elapsed;
        self.goose.foot_marks = Default::default();
        self.autumn.clear();
        self.toys.clear();
        self.current = Box::new(GracefulExitTask::new());
    }

    /// Whether shutdown has been requested but the exit walk is still in progress.
    pub fn graceful_exit_requested(&self) -> bool {
        self.graceful_exit_requested
    }

    /// True only after the complete rendered goose pose has crossed beyond real monitor pixels.
    pub fn graceful_exit_complete(&self) -> bool {
        self.graceful_exit_requested()
            && self
                .layout
                .clip_rect(self.goose.pose.bounding_box().grow(3.0))
                .is_none()
    }

    /// Pick the next roaming task from the shuffle-bag.
    fn next_task(&mut self) -> Box<dyn Task> {
        let idx = self.deck.draw();
        (self.pickable[idx])()
    }

    fn user_close_reaction_roll(rng: &mut impl RandomSource) -> bool {
        rng.next_f64() < USER_CLOSE_REACTION_PROBABILITY
    }

    fn start_user_close_reaction(&mut self, manners_active: bool) -> bool {
        let goose_visible = self
            .layout
            .clip_rect(self.goose.pose.bounding_box().grow(3.0))
            .is_some();
        if !goose_visible {
            return false;
        }
        if !Self::user_close_reaction_roll(&mut self.reaction_rng) {
            return false;
        }
        // The visual reaction is safe everywhere. Cursor movement is separately gated by the
        // existing backend capability contract, live pointer availability, and manners mode.
        self.close_reaction_nab =
            self.options.mouse_steal.active() && self.pointer.present && !manners_active;
        let prior = std::mem::replace(
            &mut self.current,
            Box::new(AnnoyedReactionTask::new(!manners_active)),
        );
        if self.interrupted.is_none() {
            self.interrupted = Some(prior);
        }
        true
    }

    fn maybe_handle_user_collect_close(&mut self, manners_active: bool) {
        if self.permission_waiting() || self.graceful_exit_requested() {
            self.pending_collect_closes.clear();
            return;
        }
        let Some(event) = self.pending_collect_closes.front().copied() else {
            return;
        };
        if event.origin == CollectWindowCloseOrigin::Program {
            self.pending_collect_closes.pop_front();
            return;
        }
        if self.is_collect_window_active() || self.interrupted.is_some() {
            // An unrelated lingering prop may close during a newer collect/transient. Preserve
            // its keyed event and react exactly once only after the active chain has unwound.
            return;
        }
        let goose_visible = self
            .layout
            .clip_rect(self.goose.pose.bounding_box().grow(3.0))
            .is_some();
        if !goose_visible {
            // Never convert an invisible startup/wrap beat directly into a silent mouse nab.
            return;
        }
        self.pending_collect_closes.pop_front();
        let _ = self.start_user_close_reaction(manners_active);
    }

    /// Advance the world by one fixed [`DT`] tick.
    pub fn tick(&mut self) {
        self.elapsed += DT as f64;
        self.refresh_schedule_state();
        let manners_active = self.manners_active();
        if self.current.id() == "affection_follow" && manners_active {
            self.resume_or_wander();
        }
        let permission_waiting = self.permission_waiting();
        let lifecycle_exiting = self.graceful_exit_requested();
        if !lifecycle_exiting {
            self.apply_hourly_honk();
        }

        let mood_event = self.mood.tick(self.elapsed, &mut self.rng);
        if mood_event.changed {
            self.rebuild_pickable();
        }
        let start_mood_hyper = mood_event.trigger_hyper
            && !lifecycle_exiting
            && !permission_waiting
            && !manners_active
            && !self.is_cursor_mischief_active()
            && !self.is_perch_ride_active()
            && !self.is_collect_window_active()
            && self.interrupted.is_none();
        if let Some(sound) = mood_event.sound.filter(|_| {
            !lifecycle_exiting && !permission_waiting && !manners_active && !start_mood_hyper
        }) {
            self.pending_sounds.push(sound);
        }
        if mood_event.spawn_sleepy_particle && !lifecycle_exiting && !permission_waiting {
            let jitter = Vec2::new(self.rng.range(-5.0, 5.0), self.rng.range(-4.0, 2.0));
            self.sleepies
                .add(self.goose.rig.neck_head + jitter, self.elapsed);
        }
        if start_mood_hyper && !lifecycle_exiting {
            self.start_hyper();
        }

        if !lifecycle_exiting {
            if let Some(kind) = self.pending_collect.take() {
                if self.options.collect_window.capacity_available
                    && self.options.collect_window.kind_active(kind)
                    && !self.is_cursor_mischief_active()
                    && !self.is_perch_ride_active()
                    && !self.is_collect_window_active()
                {
                    self.start_collect_window(kind);
                }
            }
        }

        // A click landed last frame: when cursor stealing is available it takes precedence
        // over the older M6 hyper reaction; otherwise fall back to hyper.
        if self.pending_nab && !lifecycle_exiting {
            self.pending_nab = false;
            if self.options.mouse_steal.active()
                && !self.is_cursor_mischief_active()
                && !self.is_collect_window_active()
            {
                self.pending_hyper = false;
                self.start_nab();
            }
        }

        // Install the hyper burst only when nab did not consume the click.
        if self.pending_hyper
            && !lifecycle_exiting
            && !self.is_cursor_mischief_active()
            && !self.is_collect_window_active()
        {
            self.pending_hyper = false;
            self.start_hyper();
        } else if self.pending_hyper {
            self.pending_hyper = false;
        }

        if !lifecycle_exiting
            && self.options.foreign_window.watch_active()
            && !manners_active
            && self.dragged_window.is_some()
            && !self.is_cursor_mischief_active()
            && !self.is_perch_ride_active()
            && !self.is_collect_window_active()
        {
            self.start_perch_ride();
        }

        self.maybe_handle_user_collect_close(manners_active);

        // Off-screen excursions (ADR 0016): timed interrupts over plain wandering.
        if !lifecycle_exiting {
            self.maybe_start_excursion();
        }

        // Run the current task (it only sets targets/params); pick the next when it's done.
        let calm = self.pat.is_calm(self.elapsed) || manners_active;
        let autumn_targets = if !manners_active && self.autumn_active() {
            self.autumn.targets()
        } else {
            Vec::new()
        };
        let goose_visible = self
            .layout
            .clip_rect(self.goose.pose.bounding_box().grow(3.0))
            .is_some();
        let done = {
            let mut ctx = TaskCtx {
                now: self.elapsed,
                dt: DT,
                bounds: self.bounds,
                layout: &self.layout,
                goose_visible,
                rng: &mut self.rng,
                sounds: &mut self.pending_sounds,
                cursor_commands: &mut self.pending_cursor_commands,
                collect_window_commands: &mut self.pending_collect_window_commands,
                pointer: self.pointer,
                mouse_steal: self.options.mouse_steal,
                foreign_window: self.options.foreign_window,
                collect_window: self.options.collect_window,
                dragged_window: self.dragged_window,
                collect_window_snapshot: self.collect_window_snapshot,
                calm,
                timing: self.options.timing,
                autumn_piles: &autumn_targets,
                toys: &mut self.toys,
            };
            self.current.run(&mut self.goose, &mut ctx)
        };
        let ended_collect = done && self.current.id() == "collect_window";
        if done && !lifecycle_exiting {
            if ended_collect {
                self.collect_window_snapshot = None;
            }
            if self.current.id() == "annoyed_reaction" {
                let can_nab_now = self.close_reaction_nab
                    && self.options.mouse_steal.active()
                    && self.pointer.present
                    && !manners_active;
                self.close_reaction_nab = false;
                if can_nab_now {
                    self.current = Box::new(NabMouseTask::new());
                } else {
                    self.current = self
                        .interrupted
                        .take()
                        .unwrap_or_else(|| Box::new(WanderTask::new()));
                }
            } else if self.current.id() == "excursion"
                && self.excursion_prank
                && !manners_active
                && self.options.collect_window.active()
                && self.options.collect_window.capacity_available
            {
                // Came back from the errand with mischief in mind: chain a collect
                // right away; the suspended task resumes when the collect finishes.
                self.excursion_prank = false;
                self.current = Box::new(CollectWindowTask::new());
            } else {
                if self.current.id() == "excursion" {
                    self.excursion_prank = false;
                }
                // A finished interrupt resumes the task it suspended; otherwise draw next.
                self.current = match self.interrupted.take() {
                    Some(prev) => prev,
                    None => self.next_task(),
                };
            }
            if ended_collect {
                self.maybe_handle_user_collect_close(manners_active);
            }
        }

        self.apply_mood_locomotion_modulation();
        self.apply_manners_locomotion_modulation();

        // Auto-locomotion toward the task's target, with a goosey meander on casual
        // walks (ADR 0016): a smoothly-varying lateral offset bends straight lines
        // into wandering curves, fading out near the target so arrivals still land.
        let before = self.goose.position;
        let meander = self.meander_offset();
        if meander != Vec2::ZERO {
            let saved = self.goose.target_pos;
            self.goose.target_pos = saved + meander;
            locomotion::step(&mut self.goose, DT);
            self.goose.target_pos = saved;
        } else {
            locomotion::step(&mut self.goose, DT);
        }

        // Advance the walking gait by distance travelled (a stopped goose stands still).
        let moved = Vec2::distance(before, self.goose.position);
        self.goose.gait_phase = (self.goose.gait_phase
            + moved * (std::f32::consts::TAU / GAIT_CYCLE_DISTANCE))
            .rem_euclid(std::f32::consts::TAU);

        let speed = self.goose.velocity.magnitude();
        let speed_frac = (speed / self.goose.parameters.walk_speed).min(1.0);
        // A goose stands tall: partly-raised neck at idle, rising a little with speed
        // (mood modifiers then scale the whole posture — sad/sleepy still droop).
        let neck_target = self.mood_neck_lerp(0.45 + speed_frac * 0.25);

        // Blink scheduling stays with the world (it owns the RNG and the clock); the
        // rig animates the lid deterministically from the start time.
        if self.elapsed >= self.goose.anim.next_blink {
            self.goose.anim.start_blink(self.elapsed);
            self.goose.anim.next_blink = self.elapsed + 2.0 + self.rng.next_f64() * 4.5;
        }
        // A honk this tick kicks the tail.
        if self
            .pending_sounds
            .iter()
            .any(|s| matches!(s, Sound::Honk(_)))
        {
            self.goose.anim.flick_tail();
        }

        // Per-step interval: the task-set value when present, else by speed tier.
        let step_time = if self.goose.step_interval > 1e-3 {
            self.goose.step_interval
        } else if self.goose.current_speed > self.goose.parameters.run_speed {
            self.goose.parameters.step_time_charged
        } else {
            self.goose.parameters.step_time_normal
        };

        self.goose.anim.set_expression_options(
            self.options.appearance.expressions,
            self.options.appearance.reduced_motion,
        );
        self.goose.anim.anticipate(self.current.anticipating());
        let pose = self.goose.anim.update(&RigInput {
            center: self.goose.position,
            direction_deg: self.goose.direction,
            neck_target,
            speed,
            velocity: self.goose.velocity,
            step_time,
            now: self.elapsed,
            dt: DT,
        });
        self.goose.rig = pose.primary;
        self.goose.pose = pose;
        self.goose.extending_neck = false;

        // Drop a fading muddy print exactly where a foot actually lands while tracking mud.
        let tracking_mud = !permission_waiting && self.elapsed < self.goose.track_mud_end_time;
        let elapsed = self.elapsed;
        let mut planted = 0u32;
        {
            let anim = &mut self.goose.anim;
            let marks = &mut self.goose.foot_marks;
            anim.feet.drain_plants(|foot| {
                if tracking_mud {
                    marks.add(foot, elapsed);
                    planted += 1;
                }
            });
        }
        if planted > 0 && self.rng.next_f64() < 0.35 {
            // A wet squelch now and then while squishing through mud.
            self.pending_sounds.push(Sound::MudSquish);
        }

        let had_autumn_pickable = self.autumn_pickable();
        self.autumn.tick_layout(
            self.elapsed,
            !lifecycle_exiting && !permission_waiting && self.autumn_active(),
            &self.layout,
            &self.goose,
            &mut self.rng,
        );
        if had_autumn_pickable != self.autumn_pickable() {
            self.refresh_schedule_state();
            self.rebuild_pickable();
        }

        if self.current.id() != "toy_play" {
            self.toys.release_all();
        }
        self.toys.tick(self.elapsed, &self.layout, &self.goose);
        // Now and then a wandering goose goes back to a toy that is lying around.
        if !self.toys.is_empty()
            && self.current.id() == "wander"
            && !lifecycle_exiting
            && !permission_waiting
            && !self.manners_active()
            && self.rng.next_f64() < TOY_IDLE_PLAY_CHANCE
        {
            self.current = Box::new(ToyPlayTask::new());
        }
    }

    /// The current rig (active view), for attach points and single-view rendering.
    pub fn rig(&self) -> &Rig {
        &self.goose.rig
    }

    /// The full drawable projected pose.
    pub fn pose(&self) -> &GoosePose {
        &self.goose.pose
    }

    fn apply_hourly_honk(&mut self) {
        if self.permission_waiting() || self.manners_active() {
            self.second_hourly_honk_at = None;
            return;
        }
        if !self.options.hourly_honk.on_hour_double_honk {
            return;
        }
        if let Some(due) = self.second_hourly_honk_at {
            if self.elapsed >= due {
                self.pending_sounds.push(Sound::high_honk());
                self.second_hourly_honk_at = None;
            }
        }
        let Some(local_time) = self.local_time else {
            return;
        };
        let hour = local_time.hour_key();
        if local_time.is_top_of_hour() && self.last_hourly_honk != Some(hour) {
            self.pending_sounds.push(Sound::high_honk());
            self.second_hourly_honk_at = Some(self.elapsed + SECOND_HOURLY_HONK_DELAY);
            self.last_hourly_honk = Some(hour);
        }
    }

    /// Start a long errand or a quick puddle hop when due (ADR 0016). Only plain
    /// wandering is interrupted, never mischief-in-progress, FirstUX, or manners time.
    fn maybe_start_excursion(&mut self) {
        let errand_due = self.elapsed >= self.next_excursion_at;
        let puddle_due = self.elapsed >= self.next_puddle_at;
        if !errand_due && !puddle_due {
            return;
        }
        let t = self.options.timing;
        if self.manners_active() || self.current.id() != "wander" || self.interrupted.is_some() {
            // Blocked right now: check again shortly instead of spinning.
            if errand_due {
                self.next_excursion_at = self.elapsed + 10.0;
            }
            if puddle_due {
                self.next_puddle_at = self.elapsed + 10.0;
            }
            return;
        }

        let kind = if errand_due {
            self.next_excursion_at =
                self.elapsed + self.rng.range(t.excursion_min_gap, t.excursion_max_gap) as f64;
            // An errand supersedes a due puddle hop; push the hop out.
            if puddle_due {
                self.next_puddle_at =
                    self.elapsed + self.rng.range(t.puddle_min_gap, t.puddle_max_gap) as f64;
            }
            self.excursion_prank =
                self.options.collect_window.active() && self.rng.next_f64() < 0.4;
            ExcursionKind::Errand
        } else {
            self.next_puddle_at =
                self.elapsed + self.rng.range(t.puddle_min_gap, t.puddle_max_gap) as f64;
            ExcursionKind::Puddle {
                mud_secs: self.rng.range(t.puddle_mud_min, t.puddle_mud_max),
            }
        };
        let away = match kind {
            ExcursionKind::Errand => self.rng.range(t.excursion_away_min, t.excursion_away_max),
            ExcursionKind::Puddle { .. } => self.rng.range(t.puddle_away_min, t.puddle_away_max),
        };

        let pos = self.goose.position;
        let roll = self.rng.next_f64();
        // Deliberate errands leave only through real exposed spans. A touching monitor seam
        // is not exposed, so an adjacent display is crossed naturally instead of mistaken for
        // "away" space. Usually choose the nearest edge; occasionally take a scenic side.
        let exit_edge = if roll < 0.8 {
            self.layout.nearest_exposed_edge(pos)
        } else {
            self.layout
                .sample_exposed_edge(&mut self.rng, self.layout.region_at(pos))
        };
        let exit_boundary = exit_edge.point_near(pos, LIFECYCLE_EDGE_INSET);
        let exit = exit_boundary + exit_edge.direction().outward() * LIFECYCLE_EDGE_MARGIN;

        // Puddle hops return through the same exposed edge. Errands may return through any
        // exposed edge, but are staged fully offscreen and always walk back in.
        let entry_edge = match kind {
            ExcursionKind::Puddle { .. } => exit_edge,
            ExcursionKind::Errand => self.layout.sample_exposed_edge(&mut self.rng, None),
        };
        let entry_reference = match kind {
            ExcursionKind::Puddle { .. } => {
                let jitter = self.rng.range(-60.0, 60.0);
                if exit_edge.direction().outward().x != 0.0 {
                    exit_boundary + Vec2::new(0.0, jitter)
                } else {
                    exit_boundary + Vec2::new(jitter, 0.0)
                }
            }
            ExcursionKind::Errand => self.layout.sample_point(&mut self.rng),
        };
        let entry_boundary = entry_edge.point_near(entry_reference, LIFECYCLE_EDGE_INSET);
        let entry = entry_boundary + entry_edge.direction().outward() * LIFECYCLE_EDGE_MARGIN;
        let return_target = self.layout.clamp_point(
            entry_boundary + entry_edge.direction().inward() * self.rng.range(160.0, 380.0),
        );

        let task = Box::new(ExcursionTask::new(kind, exit, entry, return_target, away));
        self.interrupted = Some(std::mem::replace(&mut self.current, task as Box<dyn Task>));
    }

    /// Lateral wander offset for casual walks; `Vec2::ZERO` when meander is inactive.
    fn meander_offset(&mut self) -> Vec2 {
        let id = self.current.id();
        if id != "wander" {
            return Vec2::ZERO;
        }
        self.meander_phase = (self.meander_phase + DT * 2.2).rem_euclid(std::f32::consts::TAU);
        if self.elapsed >= self.next_meander_shift {
            self.meander_amp_target = (self.rng.next_f64() as f32) * 2.0 - 1.0;
            self.next_meander_shift = self.elapsed + 0.8 + self.rng.next_f64() * 1.4;
        }
        let k = 1.0 - (-3.0 * DT).exp();
        self.meander_amp += (self.meander_amp_target - self.meander_amp) * k;

        let to_target = self.goose.target_pos - self.goose.position;
        let dist = to_target.magnitude();
        if dist <= 25.0 {
            return Vec2::ZERO;
        }
        let fade = ((dist - 25.0) / 120.0).clamp(0.0, 1.0);
        let perp = to_target.normalize().perpendicular();
        perp * (self.meander_amp * self.meander_phase.sin() * 48.0 * fade)
    }

    fn apply_mood_locomotion_modulation(&mut self) {
        if self.graceful_exit_requested()
            || self.permission_waiting()
            || self.current.id() == "affection_follow"
            || !self.mood.options().dynamic_moods
        {
            return;
        }
        match self.mood.current() {
            MoodKind::Content => {}
            MoodKind::Hyper => {
                self.goose.current_speed *= 1.08;
                self.goose.current_acceleration *= 1.05;
            }
            MoodKind::Sad => {
                self.goose.current_speed *= 0.72;
                self.goose.current_acceleration *= 0.8;
            }
            MoodKind::Sleepy => {
                self.goose.current_speed *= 0.55;
                self.goose.current_acceleration *= 0.65;
            }
            MoodKind::Mischievous => {
                self.goose.current_speed *= 1.04;
            }
        }
    }

    fn apply_manners_locomotion_modulation(&mut self) {
        if self.manners_active() && self.current.id() == "wander" {
            self.goose.current_speed *= 0.6;
            self.goose.current_acceleration *= 0.7;
        }
    }

    fn mood_neck_lerp(&self, base: f32) -> f32 {
        if self.goose.extending_neck {
            return 1.0;
        }
        if !self.mood.options().dynamic_moods {
            return base;
        }
        match self.mood.current() {
            MoodKind::Content => base,
            MoodKind::Hyper => base.max(0.65),
            MoodKind::Sad => base * 0.25,
            MoodKind::Sleepy => base * 0.15,
            MoodKind::Mischievous => (base + 0.16).min(1.0),
        }
    }

    fn autumn_active(&self) -> bool {
        self.options.schedule.autumn_active(self.local_time)
    }

    fn autumn_pickable(&self) -> bool {
        self.autumn_active() && self.autumn.has_unkicked_piles()
    }

    fn refresh_schedule_state(&mut self) {
        let manners_active = self.manners_active();
        let autumn_pickable = self.autumn_pickable();
        if manners_active != self.last_manners_active
            || autumn_pickable != self.last_autumn_pickable
        {
            self.last_manners_active = manners_active;
            self.last_autumn_pickable = autumn_pickable;
            self.rebuild_pickable();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collect_window::{
        CollectWindowCapabilities, CollectWindowCloseOrigin, CollectWindowId, CollectWindowKind,
        CollectWindowOptions, CollectWindowRequestId, CollectWindowSnapshot,
    };
    use crate::cursor::{AppearanceOptions, InteractionOptions, MouseStealOptions, TimingOptions};
    use crate::entity::ParametersTable;
    use crate::footmarks::FootMarkTiming;
    use crate::foreign_window::{ForeignWindowId, ForeignWindowOptions};
    use crate::layout::DesktopLayout;
    use crate::mood::{HourlyHonkOptions, MoodIntensity, MoodOptions};
    use crate::schedule::{LocalMinute, PresenceSnapshot, ScheduleOptions};

    fn bounds() -> Rect {
        Rect {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(1000.0, 800.0),
        }
    }

    fn window_snapshot(id: u64, anchor: Vec2) -> ForeignWindowSnapshot {
        ForeignWindowSnapshot::top_center(
            ForeignWindowId(id),
            Rect {
                min: Vec2::new(anchor.x - 150.0, anchor.y),
                max: Vec2::new(anchor.x + 150.0, anchor.y + 180.0),
            },
        )
    }

    fn world_with_window_watch(seed: u64) -> World {
        let mut w = World::with_options(
            bounds(),
            seed,
            WorldOptions {
                foreign_window: ForeignWindowOptions::with_backend_support(true, false),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(WanderTask::new());
        w
    }

    fn world_with_collect(seed: u64) -> World {
        let mut w = World::with_options(
            bounds(),
            seed,
            WorldOptions {
                collect_window: CollectWindowOptions::with_backend_support(
                    CollectWindowCapabilities {
                        spawn_note: true,
                        spawn_image: true,
                        move_window: true,
                        set_passthrough: true,
                        synthesize_text: true,
                    },
                    1,
                    1,
                ),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(WanderTask::new());
        w
    }

    #[test]
    fn goose_walks_in_during_first_ux() {
        let mut w = World::new(bounds(), 1);
        assert_eq!(w.current_task(), "first_ux");
        let start = w.goose.position;
        for _ in 0..240 {
            w.tick();
        }
        // It walks on-stage (upward) during the intro.
        assert!(Vec2::distance(start, w.goose.position) > 1.0);
    }

    #[test]
    fn roaming_leaves_bounds_only_for_explicit_edge_behaviors() {
        let mut w = World::new(bounds(), 2);
        // Warm up past the off-stage entrance (it reaches centre within ~1 s of walking).
        for _ in 0..1_000 {
            w.tick();
        }
        for _ in 0..5_000 {
            w.tick();
            let p = w.goose.position;
            if !w.layout.contains(p) {
                assert!(
                    matches!(w.current_task(), "edge_wrap" | "excursion"),
                    "ordinary task {} left the real desktop at {p:?}",
                    w.current_task()
                );
                assert!(
                    p.x >= -500.0 && p.x <= 1500.0 && p.y >= -500.0 && p.y <= 1300.0,
                    "edge behavior drifted unbounded at {p:?}"
                );
            }
        }
    }

    #[test]
    fn hands_off_first_ux_to_roaming() {
        let mut w = World::new(bounds(), 3);
        // FirstUX = walk in + a FIRST_WANDER_TIME pause; well past it we're roaming.
        let mut saw_wander = false;
        for _ in 0..6_000 {
            w.tick();
            if w.current_task() == "wander" {
                saw_wander = true;
                break;
            }
        }
        assert!(saw_wander, "should hand off from first_ux to wander");
    }

    #[test]
    fn first_ux_uses_configured_intro_pause() {
        let mut w = World::with_options(
            bounds(),
            4,
            WorldOptions {
                timing: TimingOptions {
                    first_wander_time: 0.1,
                    ..TimingOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        for _ in 0..1_000 {
            w.tick();
            if w.current_task() != "first_ux" {
                return;
            }
        }
        panic!("short first-wander timing should hand off to wandering quickly");
    }

    #[test]
    fn excursions_walk_off_screen_and_back() {
        let mut w = World::with_options(
            bounds(),
            7,
            WorldOptions {
                timing: TimingOptions {
                    first_wander_time: 0.1,
                    excursion_min_gap: 1.0,
                    excursion_max_gap: 1.5,
                    excursion_away_min: 0.4,
                    excursion_away_max: 0.6,
                    // Keep puddle hops out of this test's way.
                    puddle_min_gap: 100_000.0,
                    puddle_max_gap: 100_001.0,
                    ..TimingOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        let b = bounds();
        let mut saw_excursion = false;
        let mut left_bounds = false;
        for _ in 0..(120 * 90) {
            w.tick();
            if w.current_task() == "excursion" {
                saw_excursion = true;
            }
            if !b.contains(w.goose.position) {
                left_bounds = true;
            }
            if saw_excursion
                && left_bounds
                && w.current_task() == "wander"
                && b.contains(w.goose.position)
            {
                return; // A full round trip: left the screen and came back to roaming.
            }
        }
        panic!("no completed excursion (started={saw_excursion}, left_bounds={left_bounds})");
    }

    #[test]
    fn puddle_hops_bring_back_mud() {
        let mut w = World::with_options(
            bounds(),
            11,
            WorldOptions {
                timing: TimingOptions {
                    first_wander_time: 0.1,
                    puddle_min_gap: 1.0,
                    puddle_max_gap: 1.4,
                    puddle_away_min: 0.3,
                    puddle_away_max: 0.4,
                    puddle_mud_min: 5.0,
                    puddle_mud_max: 6.0,
                    // Keep long errands out of this test's way.
                    excursion_min_gap: 100_000.0,
                    excursion_max_gap: 100_001.0,
                    ..TimingOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        for _ in 0..(120 * 90) {
            w.tick();
            if w.goose.track_mud_end_time > 0.0 {
                return; // Mud started — and only a puddle hop can start it now.
            }
        }
        panic!("puddle hop never delivered mud");
    }

    #[test]
    fn plain_wandering_never_tracks_mud() {
        let mut w = World::with_options(
            bounds(),
            9,
            WorldOptions {
                timing: TimingOptions {
                    first_wander_time: 0.1,
                    min_wandering_time: 0.5,
                    max_wandering_time: 1.0,
                    excursion_min_gap: 100_000.0,
                    excursion_max_gap: 100_001.0,
                    puddle_min_gap: 100_000.0,
                    puddle_max_gap: 100_001.0,
                    ..TimingOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        for _ in 0..(120 * 30) {
            w.tick();
            assert!(
                w.goose.track_mud_end_time < 0.0,
                "mud started without a puddle hop"
            );
        }
    }

    #[test]
    fn deterministic_for_seed() {
        let mut a = World::new(bounds(), 42);
        let mut b = World::new(bounds(), 42);
        for _ in 0..4_000 {
            a.tick();
            b.tick();
        }
        assert_eq!(a.goose.position, b.goose.position);
        assert_eq!(a.current_task(), b.current_task());
    }

    #[test]
    fn tracks_mud_and_drops_fading_prints() {
        let mut w = World::new(bounds(), 5);
        // Force mud-tracking on while the goose walks in (it's moving, so it steps).
        w.goose.track_mud_end_time = 1_000.0;
        for _ in 0..700 {
            w.tick();
        }
        assert!(
            w.goose.foot_marks.alive_count(w.now()) > 0,
            "expected muddy prints while tracking mud and moving"
        );
        // With tracking off and enough time elapsed, prints fade away.
        w.goose.track_mud_end_time = -1.0;
        let faded_at = w.now() + 10.0; // past the 8.5 s lifetime
        assert_eq!(w.goose.foot_marks.alive_count(faded_at), 0);
    }

    #[test]
    fn damage_bounds_include_previous_visual_and_current_clipped_visuals() {
        let mut w = World::new(bounds(), 500);
        w.goose.position = Vec2::new(5.0, 6.0);
        w.goose.anim = RigAnim::new(w.goose.position, w.goose.direction);
        w.goose.pose = w.goose.anim.update(&RigInput::static_pose(
            w.goose.position,
            w.goose.direction,
            0.0,
        ));
        w.goose.rig = w.goose.pose.primary;
        w.goose.foot_marks.add(Vec2::new(900.0, 700.0), w.now());

        let previous = Rect::new(Vec2::new(990.0, 790.0), Vec2::new(1000.0, 800.0));
        let current = w.visual_bounds().expect("current pixels");
        let dirty = World::damage_bounds(Some(previous), Some(current)).expect("damage");

        assert_eq!(dirty.min, Vec2::new(0.0, 0.0));
        assert_eq!(dirty.max, Vec2::new(1000.0, 800.0));
        assert!(dirty.contains(Vec2::new(900.0, 700.0)));
    }

    /// Rub the static test pose over timed pointer samples without advancing locomotion.
    fn pat_the_goose(w: &mut World, frames: usize) {
        let anchor = w.goose.rig.body_center;
        // Baseline frame so the first real move has a previous position to measure from.
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: false,
        });
        for i in 0..frames {
            w.elapsed += DT as f64;
            let dx = if i % 2 == 0 { 6.0 } else { -6.0 };
            w.set_pointer(Pointer {
                pos: anchor + Vec2::new(dx, 0.0),
                present: true,
                left_down: false,
            });
        }
    }

    #[test]
    fn simulation_clock_advances_after_fourteen_days() {
        let mut w = World::new(bounds(), 420);
        w.elapsed = 14.0 * 24.0 * 60.0 * 60.0;
        let before = w.now();

        w.tick();

        assert!(
            w.now() > before,
            "fixed ticks must remain representable after two weeks"
        );
    }

    #[test]
    fn task_deadlines_expire_after_fourteen_days() {
        let mut w = World::new(bounds(), 421);
        w.elapsed = 14.0 * 24.0 * 60.0 * 60.0;
        w.current = Box::new(HyperTask::new());
        w.interrupted = Some(Box::new(WanderTask::new()));

        for _ in 0..400 {
            w.tick();
        }

        assert_eq!(
            w.current_task(),
            "wander",
            "hyper deadline should still elapse"
        );
    }

    #[test]
    fn world_visual_phases_wrap_in_long_running_sessions() {
        let mut w = World::new(bounds(), 431);
        w.current = Box::new(WanderTask::new());
        w.goose.position = Vec2::new(100.0, 100.0);
        w.goose.target_pos = Vec2::new(900.0, 700.0);
        w.meander_phase = std::f32::consts::TAU * 100_000.0;
        w.goose.gait_phase = std::f32::consts::TAU * 100_000.0;

        w.tick();

        assert!(w.meander_phase < std::f32::consts::TAU);
        assert!(w.goose.gait_phase < std::f32::consts::TAU);
    }

    #[test]
    fn startup_is_fully_offscreen_then_walks_through_an_exposed_edge() {
        let layout = DesktopLayout::new(vec![
            Rect::new(Vec2::new(-800.0, 0.0), Vec2::new(0.0, 600.0)),
            Rect::new(Vec2::new(0.0, 100.0), Vec2::new(1000.0, 800.0)),
        ])
        .expect("layout");
        let mut world = World::with_layout(layout.clone(), 432);
        assert_eq!(world.current_task(), "first_ux");
        assert!(
            world.visual_bounds().is_none(),
            "startup begins fully hidden"
        );
        assert!(layout.contains(world.goose.target_pos));

        let mut prior = world.goose.position;
        let mut emerged = false;
        for _ in 0..(120 * 12) {
            world.tick();
            let step = Vec2::distance(prior, world.goose.position);
            assert!(
                step < 5.0,
                "startup must walk continuously, not teleport ({step})"
            );
            prior = world.goose.position;
            if world.visual_bounds().is_some() {
                emerged = true;
                break;
            }
        }
        assert!(emerged, "startup goose should walk into a real monitor");
    }

    #[test]
    fn ordinary_wrap_crosses_hidden_space_and_reenters_without_a_visible_teleport() {
        let layout = DesktopLayout::new(vec![
            Rect::new(Vec2::ZERO, Vec2::new(500.0, 400.0)),
            Rect::new(Vec2::new(700.0, 100.0), Vec2::new(1100.0, 500.0)),
        ])
        .expect("gapped layout");
        let mut world = World::with_layout(layout, 433);
        place_static_goose(&mut world, Vec2::new(250.0, 200.0));
        world.current = Box::new(EdgeWrapTask::new());
        world.interrupted = None;

        let mut prior_position = world.goose.position;
        let mut prior_visible = world.visual_bounds().is_some();
        let mut hidden_teleport = false;
        let mut reentered = false;
        for _ in 0..(120 * 30) {
            world.tick();
            let visible = world.visual_bounds().is_some();
            let moved = Vec2::distance(prior_position, world.goose.position);
            if moved > 100.0 {
                assert!(
                    !prior_visible && !visible,
                    "the wrap teleport must be fully hidden on both sides"
                );
                hidden_teleport = true;
            }
            if hidden_teleport && visible {
                reentered = true;
                break;
            }
            prior_position = world.goose.position;
            prior_visible = visible;
        }
        assert!(
            hidden_teleport,
            "the deterministic wrap should reach its hidden hop"
        );
        assert!(
            reentered,
            "the wrapped goose should walk back through the far edge"
        );
    }

    #[test]
    fn touching_monitor_corridor_crosses_naturally_without_wrap_or_hidden_frame() {
        let layout = DesktopLayout::new(vec![
            Rect::new(Vec2::ZERO, Vec2::new(500.0, 400.0)),
            Rect::new(Vec2::new(500.0, 100.0), Vec2::new(1000.0, 500.0)),
        ])
        .expect("touching layout");
        let mut world = World::with_layout(layout.clone(), 436);
        place_static_goose(&mut world, Vec2::new(420.0, 220.0));
        world.current = Box::new(EdgeEntryTask::new(Vec2::new(620.0, 220.0)));
        world.interrupted = Some(Box::new(WanderTask::new()));
        let mut crossed = false;
        for _ in 0..600 {
            world.tick();
            assert!(
                world.visual_bounds().is_some(),
                "shared seam must stay visible"
            );
            assert_ne!(world.current_task(), "edge_wrap");
            if layout.region_at(world.goose.position) == Some(1) {
                crossed = true;
                break;
            }
        }
        assert!(
            crossed,
            "goose should continue into the touching monitor corridor"
        );
    }

    #[test]
    fn graceful_exit_walks_until_the_complete_pose_is_offscreen() {
        let mut world = World::new(bounds(), 434);
        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.current = Box::new(WanderTask::new());
        world.request_graceful_exit();
        assert!(world.graceful_exit_requested());
        assert!(!world.graceful_exit_complete());

        let mut prior = world.goose.position;
        for _ in 0..(120 * 20) {
            world.tick();
            let step = Vec2::distance(prior, world.goose.position);
            assert!(step < 5.0, "graceful shutdown must not teleport ({step})");
            prior = world.goose.position;
            if world.graceful_exit_complete() {
                break;
            }
        }
        assert!(world.graceful_exit_complete());
        assert!(world.visual_bounds().is_none());
        assert_eq!(world.current_task(), "graceful_exit");
        assert_eq!(world.poke(PokeAction::Honk), PokeOutcome::Busy);
    }

    #[test]
    fn four_k_graceful_exit_completes_well_inside_terminal_wait_budget() {
        let four_k = Rect::new(Vec2::ZERO, Vec2::new(3840.0, 2160.0));
        let mut world = World::new(four_k, 435);
        place_static_goose(&mut world, Vec2::new(1920.0, 1080.0));
        world.current = Box::new(WanderTask::new());
        world.request_graceful_exit();

        let mut ticks = 0usize;
        while ticks < 120 * 15 && !world.graceful_exit_complete() {
            let prior = world.goose.position;
            world.tick();
            ticks += 1;
            assert!(
                Vec2::distance(prior, world.goose.position) <= 2.0,
                "the brisk exit still uses continuous bounded locomotion"
            );
        }
        assert!(world.graceful_exit_complete());
        assert!(
            ticks < 120 * 15,
            "4K exit took {:.2}s",
            ticks as f32 / 120.0
        );
    }

    #[test]
    fn large_signed_monitor_wall_uses_adaptive_exit_inside_wait_budget() {
        let mut regions = Vec::new();
        for row in 0..5 {
            for column in 0..5 {
                let min = Vec2::new(
                    -4800.0 + column as f32 * 1920.0,
                    -2700.0 + row as f32 * 1080.0,
                );
                regions.push(Rect::new(min, min + Vec2::new(1920.0, 1080.0)));
            }
        }
        let mut world = World::with_layout(DesktopLayout::new(regions).expect("monitor wall"), 437);
        place_static_goose(&mut world, Vec2::ZERO);
        world.current = Box::new(WanderTask::new());
        world.request_graceful_exit();

        let mut ticks = 0usize;
        while ticks < 120 * 20 && !world.graceful_exit_complete() {
            world.tick();
            ticks += 1;
            assert!(
                world.goose.current_speed <= 400.0,
                "lifecycle exit must stay inside the normal charge tier"
            );
        }
        assert!(world.graceful_exit_complete());
        assert!(
            ticks < 120 * 20,
            "adaptive monitor-wall exit took {:.2}s",
            ticks as f32 / 120.0
        );
    }

    #[test]
    fn graceful_exit_ignores_near_zero_user_locomotion_tuning() {
        let mut options = WorldOptions::default();
        options.parameters.walk_speed = 0.001;
        options.parameters.run_speed = 0.001;
        options.parameters.charge_speed = 0.001;
        options.parameters.acceleration_normal = 0.001;
        options.parameters.acceleration_charged = 0.001;
        let mut world = World::with_options(
            Rect::new(Vec2::ZERO, Vec2::new(3840.0, 2160.0)),
            438,
            options,
        );
        place_static_goose(&mut world, Vec2::new(1920.0, 1080.0));
        world.current = Box::new(WanderTask::new());
        world.request_graceful_exit();

        let mut ticks = 0usize;
        while ticks < 120 * 15 && !world.graceful_exit_complete() {
            let prior = world.goose.position;
            world.tick();
            ticks += 1;
            assert!(world.goose.current_speed <= 400.0);
            assert!(
                Vec2::distance(prior, world.goose.position) < 5.0,
                "lifecycle-owned motion remains continuous"
            );
        }
        assert!(world.graceful_exit_complete());
        assert!(ticks < 120 * 15, "stop exceeded the bounded IPC wait");
    }

    #[test]
    fn region_layout_never_samples_targets_in_monitor_gaps() {
        let left = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        let right = Rect::new(Vec2::new(200.0, 0.0), Vec2::new(300.0, 100.0));
        let layout = DesktopLayout::new(vec![left, right]).expect("valid layout");
        let mut w = World::with_layout(layout.clone(), 422);

        for _ in 0..512 {
            w.current = Box::new(WanderTask::new());
            w.tick();
            assert!(
                layout.contains(w.goose.target_pos),
                "sampled target {:?} landed in the monitor gap",
                w.goose.target_pos
            );
        }
    }

    #[test]
    fn apply_layout_reconciles_an_active_excursion_after_hotplug() {
        let left = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        let right = Rect::new(Vec2::new(200.0, 0.0), Vec2::new(300.0, 100.0));
        let mut w = World::with_layout(
            DesktopLayout::new(vec![left, right]).expect("valid layout"),
            423,
        );
        w.goose.position = Vec2::new(250.0, 50.0);
        w.goose.target_pos = Vec2::new(390.0, 50.0);
        w.goose.velocity = Vec2::new(80.0, 0.0);
        w.current = Box::new(ExcursionTask::new(
            ExcursionKind::Errand,
            Vec2::new(390.0, 50.0),
            Vec2::new(390.0, 60.0),
            Vec2::new(250.0, 60.0),
            90.0,
        ));
        w.interrupted = Some(Box::new(WanderTask::new()));
        w.excursion_prank = true;

        let replacement = DesktopLayout::new(vec![left]).expect("valid layout");
        w.apply_layout(replacement.clone());

        assert!(!replacement.contains(w.goose.position));
        assert!(replacement.contains(w.goose.target_pos));
        assert_eq!(w.goose.velocity, Vec2::ZERO);
        assert_eq!(w.current_task(), "edge_entry");
        assert!(w.visual_bounds().is_none());
        assert!(!w.excursion_prank);
        for _ in 0..600 {
            w.tick();
            if w.current_task() != "edge_entry" {
                break;
            }
        }
        assert!(w.visual_bounds().is_some());
        assert_ne!(w.current_task(), "edge_entry");
    }

    #[test]
    fn autumn_targets_also_avoid_monitor_gaps() {
        let left = Rect::new(Vec2::new(0.0, 0.0), Vec2::new(100.0, 100.0));
        let right = Rect::new(Vec2::new(900.0, 0.0), Vec2::new(1000.0, 100.0));
        let layout = DesktopLayout::new(vec![left, right]).expect("valid layout");
        let mut options = WorldOptions::default();
        options.schedule.seasonal = true;
        let mut w = World::with_layout_and_options(layout.clone(), 424, options);
        w.set_local_time(LocalTime {
            day: 20260901,
            hour: 12,
            minute: 0,
            second: 0,
        });

        for _ in 0..1300 {
            w.tick();
        }

        assert!(!w.autumn().piles().is_empty());
        assert!(
            w.autumn()
                .piles()
                .iter()
                .all(|pile| layout.contains(pile.position)),
            "Autumn pile targets must be sampled from real regions"
        );
    }

    fn place_static_goose(w: &mut World, position: Vec2) {
        w.goose.position = position;
        w.goose.anim = RigAnim::new(position, 0.0);
        w.goose.pose = w
            .goose
            .anim
            .update(&RigInput::static_pose(position, 0.0, 0.45));
        w.goose.rig = w.goose.pose.primary;
    }

    fn set_next_close_reaction(world: &mut World, expected: bool) {
        let seed = (0..10_000)
            .find(|seed| {
                let mut rng = SplitMix64::seed(*seed);
                World::user_close_reaction_roll(&mut rng) == expected
            })
            .expect("both sides of the close-reaction roll");
        world.reaction_rng = SplitMix64::seed(seed);
    }

    #[test]
    fn current_visual_bounds_never_absorb_prior_damage() {
        let mut w = World::new(bounds(), 425);
        place_static_goose(&mut w, Vec2::new(120.0, 120.0));
        let first = w.visual_bounds().expect("first visible frame");

        place_static_goose(&mut w, Vec2::new(500.0, 120.0));
        let second = w.visual_bounds().expect("second visible frame");
        let damage = World::damage_bounds(Some(first), Some(second)).expect("move damage");
        assert!(damage.contains(first.min) && damage.contains(second.max));

        place_static_goose(&mut w, Vec2::new(800.0, 120.0));
        let third = w.visual_bounds().expect("third visible frame");
        let next_damage =
            World::damage_bounds(Some(second), Some(third)).expect("next move damage");
        assert!(
            !next_damage.contains(first.min),
            "feeding prior damage back would make the rectangle grow forever"
        );
    }

    #[test]
    fn four_k_frame_damage_stays_local_and_does_not_accumulate() {
        let desktop = Rect::new(Vec2::ZERO, Vec2::new(3840.0, 2160.0));
        let mut world = World::new(desktop, 4_000);
        let mut previous = None;
        let mut largest_area = 0.0_f32;

        for frame in 0..1_000 {
            let position = Vec2::new(160.0 + frame as f32 * 3.4, 1080.0);
            place_static_goose(&mut world, position);
            let current = world.visual_bounds().expect("visible 4K frame");
            let damage = World::damage_bounds(previous, Some(current)).expect("frame damage");
            assert!(desktop.contains(damage.min));
            assert!(desktop.contains(damage.max));
            largest_area = largest_area.max(damage.width() * damage.height());
            previous = Some(current);
        }

        assert!(
            largest_area < 512.0 * 512.0,
            "one-frame 4K damage unexpectedly grew to {largest_area} pixels"
        );
    }

    #[test]
    fn fully_hidden_visuals_have_no_bounds_or_damage() {
        let mut w = World::new(bounds(), 426);
        place_static_goose(&mut w, Vec2::new(-10_000.0, -10_000.0));

        let current = w.visual_bounds();

        assert_eq!(current, None);
        assert_eq!(World::damage_bounds(None, current), None);
    }

    #[test]
    fn vertical_puddle_return_preserves_the_departure_edge() {
        let mut found = None;
        for seed in 0..512 {
            let mut w = World::with_options(
                bounds(),
                seed,
                WorldOptions {
                    timing: TimingOptions {
                        puddle_min_gap: 0.0,
                        puddle_max_gap: 0.0,
                        puddle_away_min: 0.0,
                        puddle_away_max: 0.0,
                        excursion_min_gap: 10_000.0,
                        excursion_max_gap: 10_001.0,
                        ..TimingOptions::default()
                    },
                    ..WorldOptions::default()
                },
            );
            w.current = Box::new(WanderTask::new());
            w.next_puddle_at = 0.0;
            w.next_excursion_at = f64::INFINITY;
            w.tick();
            let exit = w.goose.target_pos;
            if exit.x >= w.bounds.min.x
                && exit.x <= w.bounds.max.x
                && (exit.y < w.bounds.min.y || exit.y > w.bounds.max.y)
            {
                found = Some((w, exit));
                break;
            }
        }
        let (mut w, exit) = found.expect("a deterministic seed should choose a vertical edge");

        w.goose.position = exit;
        w.goose.velocity = Vec2::ZERO;
        w.tick(); // Depart -> Away.
        w.tick(); // Away -> Return, placing the goose at the staged entry.

        assert!(
            w.goose.position.y < w.bounds.min.y || w.goose.position.y > w.bounds.max.y,
            "a vertical puddle return must reappear beyond the same top/bottom edge"
        );
        assert!((w.goose.position.x - exit.x).abs() <= 61.0);
    }

    #[test]
    fn delayed_excursion_prank_is_cancelled_when_manners_activate() {
        let mut w = World::new(bounds(), 427);
        let at = w.goose.position;
        w.current = Box::new(ExcursionTask::new(ExcursionKind::Errand, at, at, at, 0.0));
        w.interrupted = Some(Box::new(WanderTask::new()));
        w.excursion_prank = true;
        w.tick(); // Depart -> Away.
        w.tick(); // Away -> Return.
        w.set_presence(PresenceSnapshot::do_not_disturb());

        w.tick(); // Finish the return while manners are active.

        assert_eq!(w.current_task(), "wander");
        assert!(!w.excursion_prank);
    }

    #[test]
    fn disabling_notes_cancels_an_active_note_even_when_memes_stay_enabled() {
        let mut w = world_with_collect(428);
        w.force_collect_window(CollectWindowKind::Note);
        w.tick();
        assert_eq!(w.current_task(), "collect_window");
        w.take_collect_window_commands();
        let mut options = w.options;
        options.collect_window.notes_enabled = false;
        assert!(options.collect_window.kind_active(CollectWindowKind::Meme));

        w.apply_options(options);

        assert_eq!(w.current_task(), "wander");
    }

    #[test]
    fn runtime_capability_changes_refresh_the_pickable_deck() {
        let mut w = World::with_options(
            bounds(),
            429,
            WorldOptions {
                collect_window: CollectWindowOptions {
                    available_notes: 1,
                    available_memes: 0,
                    ..CollectWindowOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        assert_eq!(w.pickable.len(), 5);

        w.set_collect_window_supported(true);

        assert_eq!(w.pickable.len(), 6);
    }

    #[test]
    fn entering_hyper_mood_emits_one_honk_not_two() {
        let mut w = World::with_options(
            bounds(),
            430,
            WorldOptions {
                mood: MoodOptions {
                    dynamic_moods: true,
                    intensity: MoodIntensity::Spicy,
                },
                ..WorldOptions::default()
            },
        );
        let mut prior = w.mood();
        for _ in 0..240_000 {
            w.tick();
            let now = w.mood();
            let sounds = w.take_sounds();
            if prior != MoodKind::Hyper && now == MoodKind::Hyper {
                let honks = sounds
                    .iter()
                    .filter(|sound| matches!(sound, Sound::Honk(_)))
                    .count();
                assert_eq!(honks, 1, "Hyper transition and task must share one honk");
                return;
            }
            prior = now;
        }
        panic!("seed never entered Hyper mood");
    }

    #[test]
    fn hourly_and_hyper_share_one_immediate_honk() {
        let mut w = World::with_options(
            bounds(),
            432,
            WorldOptions {
                mood: MoodOptions {
                    dynamic_moods: false,
                    intensity: MoodIntensity::Normal,
                },
                hourly_honk: HourlyHonkOptions {
                    on_hour_double_honk: true,
                },
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(WanderTask::new());
        w.pending_hyper = true;
        w.set_local_time(LocalTime {
            day: 20260710,
            hour: 10,
            minute: 0,
            second: 0,
        });

        w.tick();

        assert_eq!(
            w.take_sounds(),
            vec![Sound::high_honk()],
            "the top-of-hour and Hyper entry must not stack identical honks"
        );
    }

    #[test]
    fn hovering_sweeps_pat_the_goose_spawning_hearts_and_calm() {
        let mut w = World::new(bounds(), 1);
        pat_the_goose(&mut w, 360);
        assert!(
            w.hearts().alive_count(w.now()) >= 1,
            "patting spawns heart particles"
        );
        assert!(w.is_calm(), "patting calms the goose");
    }

    #[test]
    fn configured_pat_streak_off_disables_hearts_and_calm() {
        let mut w = World::with_options(
            bounds(),
            1,
            WorldOptions {
                interaction: InteractionOptions { pat_streak: false },
                ..WorldOptions::default()
            },
        );
        pat_the_goose(&mut w, 360);
        assert_eq!(w.hearts().alive_count(w.now()), 0);
        assert!(!w.is_calm());
    }

    fn rub_until_following() -> World {
        let mut world = World::new(bounds(), 42);
        world.current = Box::new(WanderTask::new());
        world.goose.position = Vec2::new(500.0, 400.0);
        world.goose.target_pos = world.goose.position;
        world.goose.anim = RigAnim::new(world.goose.position, 0.0);
        world.goose.pose =
            world
                .goose
                .anim
                .update(&RigInput::static_pose(world.goose.position, 0.0, 0.0));
        world.goose.rig = world.goose.pose.primary;
        for frame in 0..360 {
            world.set_pointer(Pointer {
                pos: world.goose.rig.body_center
                    + Vec2::new(if frame % 2 == 0 { 6.0 } else { -6.0 }, 0.0),
                present: true,
                left_down: false,
            });
            if frame < 300 {
                assert_ne!(world.current_task(), "affection_follow");
                assert_eq!(world.hearts().alive_count(world.now()), 0);
                assert_eq!(world.goose.pose.primary.pleased, 0.0);
                assert!(!world.is_calm());
                assert!(!world.take_sounds().contains(&Sound::Pat));
            }
            assert!(world.take_cursor_commands().is_empty());
            world.tick();
            if world.current_task() == "affection_follow" {
                break;
            }
        }
        assert_eq!(world.current_task(), "affection_follow");
        assert!(world.hearts().alive_count(world.now()) > 0);
        world
    }

    #[test]
    fn sustained_rubbing_invites_finite_walking_follow_with_a_gap_and_no_cursor_commands() {
        let mut world = rub_until_following();
        let followed_at = world.now();
        let cursor = world
            .layout
            .clamp_point(world.goose.position + Vec2::new(250.0, 100.0));
        let initial_distance = Vec2::distance(cursor, world.goose.position);
        while world.now() - followed_at < 5.0 {
            world.set_pointer(Pointer {
                pos: cursor,
                present: true,
                left_down: false,
            });
            world.tick();
            assert_eq!(world.current_task(), "affection_follow");
            assert!(world.take_cursor_commands().is_empty());
            assert!(world.take_collect_window_commands().is_empty());
            assert!(world.goose.velocity.magnitude() <= world.goose.parameters.walk_speed + 0.01);
        }
        let settled_distance = Vec2::distance(cursor, world.goose.position);
        assert!(settled_distance < initial_distance - 50.0);
        assert!(
            (89.0..95.0).contains(&settled_distance),
            "follow gap: {settled_distance}"
        );
        while world.now() - followed_at < crate::task::AFFECTION_FOLLOW_SECONDS + 0.05 {
            world.tick();
            assert!(world.take_cursor_commands().is_empty());
        }
        assert_eq!(world.current_task(), "wander");
        assert!(world.interrupted.is_none());
    }

    #[test]
    fn affection_follow_cancels_on_pointer_loss_opt_out_manners_and_shutdown() {
        let mut world = rub_until_following();
        world.set_pointer(Pointer::default());
        world.tick();
        assert_eq!(world.current_task(), "wander");

        let mut world = rub_until_following();
        let mut options = world.options;
        options.interaction.pat_streak = false;
        world.apply_options(options);
        assert_eq!(world.current_task(), "wander");

        let mut world = rub_until_following();
        let mut options = world.options;
        options.appearance.calm_goose = true;
        world.apply_options(options);
        world.tick();
        assert_eq!(world.current_task(), "wander");

        let mut world = rub_until_following();
        world.enter_permission_wait(Vec2::new(300.0, 300.0));
        assert_eq!(world.current_task(), "permission_wait");
        assert!(world.interrupted.is_none());
        assert!(world.take_cursor_commands().is_empty());
        world.request_graceful_exit();
        assert_eq!(world.current_task(), "graceful_exit");
    }

    #[test]
    fn cursor_off_the_goose_does_not_pat() {
        let mut w = World::new(bounds(), 1);
        let away = w.bounds.max + Vec2::new(50.0, 50.0); // well outside the goose
        w.set_pointer(Pointer {
            pos: away,
            present: true,
            left_down: false,
        });
        for i in 0..12 {
            let dx = if i % 2 == 0 { 20.0 } else { -20.0 };
            w.set_pointer(Pointer {
                pos: away + Vec2::new(dx, 0.0),
                present: true,
                left_down: false,
            });
        }
        assert_eq!(w.hearts().alive_count(w.now()), 0, "no pats off the goose");
        assert!(!w.is_calm());
    }

    #[test]
    fn clicking_the_goose_triggers_hyper_then_resumes_prior_task() {
        let mut w = World::new(bounds(), 5);
        // Warm up into the roaming wander task.
        for _ in 0..6_000 {
            w.tick();
            if w.current_task() == "wander" {
                break;
            }
        }
        assert_eq!(w.current_task(), "wander");

        // Default engine options do not assume cursor warp support, so click falls back to
        // the M6 hyper behavior: release/idle baseline frame, then the press edge.
        let anchor = w.goose.rig.body_center;
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: false,
        });
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: true,
        });
        w.tick();
        assert_eq!(w.current_task(), "hyper", "a click sends the goose hyper");

        // After the burst it resumes the task it interrupted.
        for _ in 0..(120 * 3) {
            w.tick();
        }
        assert_eq!(
            w.current_task(),
            "wander",
            "the hyper burst resumes the prior task"
        );
    }

    #[test]
    fn clicking_the_goose_triggers_hyper_even_with_pat_streak_off() {
        // Disabling the hover-pat streak (hearts/calm) must NOT also disable the M6 click
        // reaction. Patting and clicking are distinct interactions; turning off pats should
        // leave click-to-hyper working.
        let mut w = World::with_options(
            bounds(),
            5,
            WorldOptions {
                interaction: InteractionOptions { pat_streak: false },
                ..WorldOptions::default()
            },
        );
        for _ in 0..6_000 {
            w.tick();
            if w.current_task() == "wander" {
                break;
            }
        }
        assert_eq!(w.current_task(), "wander");

        let anchor = w.goose.rig.body_center;
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: false,
        });
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: true,
        });
        w.tick();
        assert_eq!(
            w.current_task(),
            "hyper",
            "clicking the goose triggers hyper even when the pat streak is disabled"
        );
    }

    #[test]
    fn clicking_the_goose_triggers_nab_when_mouse_steal_is_supported() {
        let mut w = World::with_options(
            bounds(),
            8,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        // Warm up into roaming so this verifies a normal user click, not first-run setup.
        for _ in 0..6_000 {
            w.tick();
            if w.current_task() == "wander" {
                break;
            }
        }
        assert_eq!(w.current_task(), "wander");

        let anchor = w.goose.rig.body_center;
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: false,
        });
        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: true,
        });
        w.tick();

        assert_eq!(
            w.current_task(),
            "nab_mouse",
            "with cursor warp support, clicking the goose should steal the cursor instead of hyper"
        );
        assert!(
            !w.take_cursor_commands().is_empty(),
            "click-triggered nab should emit a cursor warp command"
        );
        assert_eq!(
            w.take_sounds(),
            vec![Sound::Bite],
            "click-triggered nab bites when it catches the cursor"
        );
    }

    #[test]
    fn clicking_away_from_the_goose_does_not_trigger_hyper() {
        let mut w = World::new(bounds(), 6);
        let away = w.bounds.max + Vec2::new(50.0, 50.0);
        w.set_pointer(Pointer {
            pos: away,
            present: true,
            left_down: false,
        });
        w.set_pointer(Pointer {
            pos: away,
            present: true,
            left_down: true,
        });
        w.tick();
        assert_ne!(
            w.current_task(),
            "hyper",
            "clicks off the goose pass through"
        );
    }

    #[test]
    fn emits_sound_requests_while_roaming() {
        let mut w = World::new(bounds(), 7);
        let mut heard = false;
        // Run well past FirstUX into roaming; the goose honks on retarget / squishes in mud.
        for _ in 0..30_000 {
            w.tick();
            if !w.take_sounds().is_empty() {
                heard = true;
                break;
            }
        }
        assert!(
            heard,
            "the goose should request sounds (honk/mud) while roaming"
        );
    }

    #[test]
    fn baseline_roaming_weights_edge_wrap_to_one_in_five_deck_draws() {
        let world = World::new(bounds(), 701);
        let ids: Vec<_> = world
            .pickable
            .iter()
            .map(|factory| factory().id())
            .collect();
        assert_eq!(ids.iter().filter(|id| **id == "wander").count(), 4);
        assert_eq!(ids.iter().filter(|id| **id == "edge_wrap").count(), 1);
        assert_eq!(ids.len(), 5);
    }

    #[test]
    fn nab_is_pickable_only_when_enabled_and_supported() {
        let default_world = World::new(bounds(), 1);
        assert_eq!(
            default_world.pickable.len(),
            5,
            "ordinary roaming weights four wander beats to one edge wrap"
        );

        let disabled = World::with_options(
            bounds(),
            1,
            WorldOptions {
                mouse_steal: MouseStealOptions {
                    enabled: false,
                    warp_supported: true,
                    ..MouseStealOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        assert_eq!(disabled.pickable.len(), 5);

        let supported = World::with_options(
            bounds(),
            1,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        assert_eq!(
            supported.pickable.len(),
            6,
            "nab_mouse joins roaming only when the backend can warp the cursor"
        );
    }

    #[test]
    fn nab_reaches_a_stationary_pointer_through_real_locomotion_and_cancels_on_revoke() {
        for target in [
            Vec2::new(700.0, 350.0),
            Vec2::new(400.0, 150.0),
            Vec2::new(180.0, 420.0),
        ] {
            let mut world = World::with_options(
                bounds(),
                91,
                WorldOptions {
                    mouse_steal: MouseStealOptions::with_backend_support(true),
                    ..WorldOptions::default()
                },
            );
            place_static_goose(&mut world, Vec2::new(400.0, 350.0));
            world.current = Box::new(NabMouseTask::new());
            let mut grabbed = false;
            for _ in 0..1200 {
                world.set_pointer(Pointer {
                    pos: target,
                    present: true,
                    left_down: false,
                });
                world.tick();
                if !world.take_cursor_commands().is_empty() {
                    grabbed = true;
                    break;
                }
                world.take_sounds();
            }
            assert!(
                grabbed,
                "never reached pointer {target:?}; beak={:?}",
                world.goose.rig.beak_tip
            );
            let mut options = world.options;
            options.mouse_steal.warp_supported = false;
            world.apply_options(options);
            world.tick();
            assert!(world.take_cursor_commands().is_empty());
            assert_ne!(world.current_task(), "nab_mouse");
        }
    }

    #[test]
    fn cursor_capability_revocation_discards_an_already_queued_real_warp() {
        let mut world = World::with_options(
            bounds(),
            9,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        world.current = Box::new(NabMouseTask::new());
        world.set_pointer(Pointer {
            pos: world.goose.rig.beak_tip,
            present: true,
            left_down: false,
        });
        world.tick();
        assert!(
            !world.pending_cursor_commands.is_empty(),
            "The actual task must queue a warp before revocation"
        );
        world.set_cursor_warp_supported(false);
        assert!(world.take_cursor_commands().is_empty());
        assert_ne!(world.current_task(), "nab_mouse");
    }

    #[test]
    fn cursor_commands_are_queued_and_drained_once() {
        let mut w = World::with_options(
            bounds(),
            9,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(NabMouseTask::new());
        let pointer = w.goose.rig.beak_tip;
        w.set_pointer(Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        });
        w.tick();

        assert_eq!(
            w.take_sounds(),
            vec![Sound::Bite],
            "nab emits the bite sound when it grabs"
        );
        assert_eq!(
            w.take_cursor_commands(),
            vec![CursorCommand::WarpTo(pointer)],
            "nab emits a platform-free cursor warp"
        );
        assert!(
            w.take_cursor_commands().is_empty(),
            "cursor commands drain exactly once"
        );
    }

    #[test]
    fn nab_suppresses_pat_and_click_hyper_interactions() {
        let mut w = World::with_options(
            bounds(),
            10,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(NabMouseTask::new());

        let anchor = w.goose.rig.body_center;
        for i in 0..12 {
            let dx = if i % 2 == 0 { 6.0 } else { -6.0 };
            w.set_pointer(Pointer {
                pos: anchor + Vec2::new(dx, 0.0),
                present: true,
                left_down: false,
            });
        }
        assert_eq!(
            w.hearts().alive_count(w.now()),
            0,
            "synthetic cursor movement during nab must not pat the goose"
        );

        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: true,
        });
        w.tick();
        assert_ne!(
            w.current_task(),
            "hyper",
            "click edges during nab must not interrupt into hyper"
        );
    }

    #[test]
    fn foreign_window_drag_does_not_start_without_watch_capability() {
        let mut w = World::new(bounds(), 11);
        w.current = Box::new(WanderTask::new());
        w.set_foreign_window_drag(Some(window_snapshot(1, Vec2::new(600.0, 100.0))));
        w.tick();
        assert_ne!(
            w.current_task(),
            "perch_ride",
            "default engine options do not assume foreign-window watch support"
        );
    }

    #[test]
    fn foreign_window_drag_interrupts_and_release_before_arrival_resumes() {
        let mut w = world_with_window_watch(12);
        assert_eq!(w.current_task(), "wander");

        w.set_foreign_window_drag(Some(window_snapshot(2, Vec2::new(900.0, 80.0))));
        w.tick();
        assert_eq!(w.current_task(), "perch_ride");

        w.set_foreign_window_drag(None);
        w.tick();
        assert_eq!(
            w.current_task(),
            "wander",
            "releasing before arrival resumes the interrupted task"
        );
    }

    #[test]
    fn foreign_window_drag_rides_moving_anchor_until_release() {
        let mut w = world_with_window_watch(13);
        let first_anchor = Vec2::new(420.0, 90.0);
        w.goose.position = first_anchor + Vec2::new(1.0, 1.0);

        w.set_foreign_window_drag(Some(window_snapshot(3, first_anchor)));
        w.tick();
        assert_eq!(w.current_task(), "perch_ride");
        assert_eq!(w.goose.position, first_anchor);

        let moved_anchor = Vec2::new(500.0, 110.0);
        w.set_foreign_window_drag(Some(window_snapshot(3, moved_anchor)));
        w.tick();
        assert_eq!(w.current_task(), "perch_ride");
        assert_eq!(w.goose.position, moved_anchor);
        assert_eq!(w.goose.velocity, Vec2::ZERO);

        w.set_foreign_window_drag(None);
        w.tick();
        assert_eq!(w.current_task(), "wander");
    }

    #[test]
    fn foreign_window_watch_capability_loss_abandons_cleanly() {
        let mut w = world_with_window_watch(14);
        let anchor = Vec2::new(430.0, 100.0);
        w.goose.position = anchor + Vec2::new(1.0, 0.0);
        w.set_foreign_window_drag(Some(window_snapshot(4, anchor)));
        w.tick();
        assert_eq!(w.current_task(), "perch_ride");

        w.set_foreign_window_watch_supported(false);
        w.tick();
        assert_eq!(w.current_task(), "wander");
    }

    #[test]
    fn perch_ride_suppresses_pat_and_click_hyper_interactions() {
        let mut w = world_with_window_watch(15);
        let anchor = w.goose.rig.body_center;
        w.set_foreign_window_drag(Some(window_snapshot(5, anchor)));
        w.tick();
        assert_eq!(w.current_task(), "perch_ride");

        for i in 0..12 {
            let dx = if i % 2 == 0 { 6.0 } else { -6.0 };
            w.set_pointer(Pointer {
                pos: anchor + Vec2::new(dx, 0.0),
                present: true,
                left_down: false,
            });
        }
        assert_eq!(
            w.hearts().alive_count(w.now()),
            0,
            "cursor motion during perch/ride must not pat the goose"
        );

        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: true,
        });
        w.tick();
        assert_eq!(
            w.current_task(),
            "perch_ride",
            "click edges during perch/ride must not interrupt into hyper"
        );
    }

    #[test]
    fn forced_collect_window_queues_spawn_and_drains_once() {
        let mut w = world_with_collect(16);
        w.force_collect_window(CollectWindowKind::Note);
        w.tick();
        assert_eq!(w.current_task(), "collect_window");
        assert!(matches!(
            w.take_collect_window_commands().as_slice(),
            [CollectWindowCommand::Spawn { .. }]
        ));
        assert!(w.take_collect_window_commands().is_empty());
    }

    #[test]
    fn collect_window_suppresses_pat_and_click_hyper_interactions() {
        let mut w = world_with_collect(17);
        w.force_collect_window(CollectWindowKind::Meme);
        w.tick();
        assert_eq!(w.current_task(), "collect_window");

        let anchor = w.goose.rig.body_center;
        for i in 0..12 {
            let dx = if i % 2 == 0 { 6.0 } else { -6.0 };
            w.set_pointer(Pointer {
                pos: anchor + Vec2::new(dx, 0.0),
                present: true,
                left_down: false,
            });
        }
        assert_eq!(w.hearts().alive_count(w.now()), 0);

        w.set_pointer(Pointer {
            pos: anchor,
            present: true,
            left_down: true,
        });
        w.tick();
        assert_eq!(w.current_task(), "collect_window");
    }

    #[test]
    fn collect_window_capability_loss_abandons_cleanly() {
        let mut w = world_with_collect(18);
        w.force_collect_window(CollectWindowKind::Meme);
        w.tick();
        let request = match w.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        w.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(1),
            request: CollectWindowRequestId(request.0),
            kind: CollectWindowKind::Meme,
            rect: Rect {
                min: Vec2::new(200.0, 100.0),
                max: Vec2::new(500.0, 300.0),
            },
            alive: true,
            close_origin: None,
        }));
        w.set_collect_window_supported(false);
        w.tick();
        assert_eq!(w.current_task(), "wander");
    }

    #[test]
    fn collect_window_horizontal_approach_reaches_the_prop_with_normal_locomotion() {
        let mut world = world_with_collect(433);
        place_static_goose(&mut world, Vec2::new(150.0, 400.0));
        world.goose.direction = 0.0;
        world.goose.target_pos = world.goose.position;
        world.goose.velocity = Vec2::ZERO;
        world.force_collect_window(CollectWindowKind::Note);
        world.tick();
        let request = match world.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(433),
            request,
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::new(600.0, 300.0), Vec2::new(900.0, 500.0)),
            alive: true,
            close_origin: None,
        }));

        let mut grabbed = false;
        let mut typed = false;
        for _ in 0..(120 * 15) {
            world.tick();
            for command in world.take_collect_window_commands() {
                match command {
                    CollectWindowCommand::SetPassthrough {
                        passthrough: true, ..
                    } => grabbed = true,
                    CollectWindowCommand::TypeNote { .. } => typed = true,
                    _ => {}
                }
            }
            if typed {
                break;
            }
        }

        assert!(
            grabbed,
            "a horizontal side-view approach must bring the beak to the prop"
        );
        assert!(
            typed,
            "the fixed-tick collect flow must progress through release and typing"
        );
    }

    #[test]
    fn delivered_note_recovery_withdraws_the_airborne_walking_prediction() {
        for (seed, delay, initial) in [
            (3, 0, Vec2::new(500.0, 150.0)),
            (3, 10, Vec2::new(500.0, 300.0)),
            (1_788_900_900_000_000_000, 40, Vec2::new(430.0, 320.0)),
            (1_788_900_900_000_000_000, 600, Vec2::new(430.0, 312.0)),
        ] {
            let mut options = WorldOptions::default();
            options.timing.first_wander_time = 600.0;
            options.mood.dynamic_moods = true;
            options.schedule.seasonal = false;
            options.collect_window = CollectWindowOptions::with_backend_support(
                CollectWindowCapabilities {
                    spawn_note: true,
                    spawn_image: true,
                    move_window: true,
                    set_passthrough: true,
                    synthesize_text: true,
                },
                12,
                15,
            );
            let mut world = World::with_options(
                Rect::new(Vec2::ZERO, Vec2::new(1280.0, 900.0)),
                seed,
                options,
            );
            for _ in 0..delay {
                world.tick();
            }
            world.force_collect_window(CollectWindowKind::Note);
            let mut native = None;
            let mut arrival = None;
            let mut typed = false;
            let mut settled = false;
            let size = Vec2::new(420.0, 288.0);
            for tick in 0..1800 {
                let before_feet = world.goose.rig.feet_pose;
                world.tick();
                for (before, after) in before_feet.into_iter().zip(world.goose.rig.feet_pose) {
                    if !before.swinging && !after.swinging {
                        assert_eq!(before.pos, after.pos, "delivery moved a planted contact");
                    }
                }
                for command in world.take_collect_window_commands() {
                    match command {
                        CollectWindowCommand::Spawn { request, payload } => {
                            native = Some(CollectWindowSnapshot {
                                id: CollectWindowId(1),
                                request,
                                kind: payload.kind(),
                                rect: Rect::new(initial, initial + size),
                                alive: true,
                                close_origin: None,
                            });
                        }
                        CollectWindowCommand::Move { top_left, .. } => {
                            let prop = native.as_mut().expect("move follows native creation");
                            prop.rect = Rect::new(top_left, top_left + size);
                        }
                        CollectWindowCommand::TypeNote { .. } => {
                            typed = true;
                        }
                        _ => {}
                    }
                }
                // Exercise delayed native creation followed by actual delivery commands.
                if tick > 60 {
                    world.set_collect_window_snapshot(native);
                }
                if typed && arrival.is_none() && world.goose.velocity == Vec2::ZERO {
                    arrival = Some((tick, world.goose.rig.feet_pose));
                }
                if let Some((typed_at, feet)) = arrival {
                    assert_eq!(world.goose.velocity, Vec2::ZERO);
                    let stance = crate::feet::FeetState::new(
                        world.goose.position,
                        Vec2::from_angle_degrees(world.goose.direction),
                    )
                    .poses();
                    for (index, foot) in world.goose.rig.feet_pose.iter().enumerate() {
                        let arrival_distance = Vec2::distance(feet[index].pos, stance[index].pos);
                        // The native-height witness reverses during pickup without
                        // a zero-speed tick. A stale swing must not starve the other
                        // planted foot beyond the existing Run/Charge gait bound.
                        assert!(
                            arrival_distance <= 26.0,
                            "delivery seed {seed} leaves foot {index} {arrival_distance:.3}px from its stopped home"
                        );
                        assert!(
                            Vec2::distance(foot.pos, stance[index].pos)
                                <= arrival_distance.max(8.0) + 0.001,
                            "a delivered note must not leave a foot extending its old walking lead"
                        );
                    }
                    if tick >= typed_at + 60 {
                        assert!(world.goose.rig.feet_pose.iter().all(|foot| !foot.swinging));
                        settled = true;
                        break;
                    }
                }
            }
            assert!(arrival.is_some(), "actual note delivery did not finish");
            assert!(
                settled,
                "delivery recovery did not complete its observation window"
            );
        }
    }

    #[test]
    fn poke_honk_queues_sound_without_ticking() {
        let mut w = World::new(bounds(), 19);
        assert_eq!(w.poke(PokeAction::Honk), PokeOutcome::Applied);
        assert_eq!(w.take_sounds(), vec![Sound::honk()]);
        assert!(w.take_sounds().is_empty());
    }

    #[test]
    fn permission_wait_walks_to_anchor_and_never_finishes() {
        let anchor = Vec2::new(700.0, 500.0);
        let mut world = World::new(bounds(), 31);
        world
            .pending_cursor_commands
            .push(CursorCommand::WarpTo(Vec2::new(1.0, 2.0)));
        world
            .pending_collect_window_commands
            .push(CollectWindowCommand::Close {
                id: CollectWindowId(9),
            });
        world.pending_hyper = true;
        world.pending_nab = true;
        world.pending_collect = Some(CollectWindowKind::Meme);
        world.interrupted = Some(Box::new(WanderTask::new()));

        world.enter_permission_wait(Vec2::new(600.0, 400.0));
        assert!(world.permission_waiting());
        assert!(world.interrupted.is_none());
        assert!(!world.pending_hyper);
        assert!(!world.pending_nab);
        assert!(world.pending_collect.is_none());
        world.update_permission_wait_anchor(anchor);
        for _ in 0..(120 * 12) {
            world.tick();
        }
        assert_eq!(world.current_task(), "permission_wait");
        assert!(Vec2::distance(world.goose.position, anchor) < 3.0);
        assert!(world.take_cursor_commands().is_empty());
        assert!(world.take_collect_window_commands().is_empty());
    }

    #[test]
    fn permission_wait_allows_only_honk_and_grant_resumes_first_ux() {
        let mut world = World::new(bounds(), 32);
        world.enter_permission_wait(Vec2::new(700.0, 500.0));
        assert_eq!(world.poke(PokeAction::Honk), PokeOutcome::Applied);
        for action in [
            PokeAction::Wander,
            PokeAction::Mud,
            PokeAction::Nab,
            PokeAction::Meme,
            PokeAction::Note,
        ] {
            assert_eq!(world.poke(action), PokeOutcome::Busy);
        }
        world.leave_permission_wait();
        assert!(!world.permission_waiting());
        assert_eq!(world.current_task(), "first_ux");
    }

    #[test]
    fn permission_wait_blocks_pats_and_clears_pat_visuals_and_sound() {
        let mut world = World::new(bounds(), 33);
        pat_the_goose(&mut world, 360);
        assert!(world.is_calm());
        assert!(world.hearts().alive_count(world.now()) > 0);
        assert!(world.take_sounds().contains(&Sound::Pat));

        pat_the_goose(&mut world, 360);
        world.sleepies.add(world.goose.rig.neck_head, world.now());
        world.enter_permission_wait(Vec2::new(700.0, 500.0));

        assert!(!world.is_calm());
        assert_eq!(world.hearts().alive_count(world.now()), 0);
        assert_eq!(world.sleepies().alive_count(world.now()), 0);
        assert!(world.take_sounds().is_empty());

        pat_the_goose(&mut world, 720);
        assert!(!world.is_calm());
        assert_eq!(world.hearts().alive_count(world.now()), 0);
        assert!(world.take_sounds().is_empty());
    }

    #[test]
    fn permission_wait_clears_and_suppresses_mud_autumn_and_queued_sound() {
        let mut options = WorldOptions::default();
        options.schedule.seasonal = true;
        let mut world = World::with_options(bounds(), 34, options);
        world.set_local_time(LocalTime {
            day: 20260915,
            hour: 12,
            minute: 0,
            second: 0,
        });
        for _ in 0..1_300 {
            world.tick();
        }
        assert!(!world.autumn().piles().is_empty());

        let now = world.now();
        world.goose.track_mud_end_time = now + 100.0;
        world.goose.foot_marks.add(Vec2::new(10.0, 10.0), now);
        world.pending_sounds.push(Sound::MudSquish);
        world.enter_permission_wait(Vec2::new(700.0, 500.0));

        assert!(world.goose.track_mud_end_time <= world.now());
        assert_eq!(world.goose.foot_marks.alive_count(world.now()), 0);
        assert!(world.autumn().piles().is_empty());
        assert!(world.take_sounds().is_empty());

        for _ in 0..1_300 {
            world.tick();
        }
        assert_eq!(world.goose.foot_marks.alive_count(world.now()), 0);
        assert!(world.autumn().piles().is_empty());
        assert!(world.take_sounds().is_empty());
        assert_eq!(world.poke(PokeAction::Honk), PokeOutcome::Applied);
        assert_eq!(world.take_sounds(), vec![Sound::honk()]);
    }

    #[test]
    fn permission_wait_cleans_up_an_active_collect_window() {
        let mut world = world_with_collect(35);
        world.force_collect_window(CollectWindowKind::Meme);
        world.tick();
        let request = match world.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        let id = CollectWindowId(35);
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id,
            request,
            kind: CollectWindowKind::Meme,
            rect: Rect {
                min: Vec2::new(200.0, 100.0),
                max: Vec2::new(500.0, 300.0),
            },
            alive: true,
            close_origin: None,
        }));
        world
            .pending_collect_window_commands
            .push(CollectWindowCommand::Move {
                id,
                top_left: Vec2::new(300.0, 200.0),
            });

        world.enter_permission_wait(Vec2::new(700.0, 500.0));

        assert_eq!(world.current_task(), "permission_wait");
        assert_eq!(
            world.take_collect_window_commands(),
            vec![
                CollectWindowCommand::SetPassthrough {
                    id,
                    passthrough: false,
                },
                CollectWindowCommand::Close { id },
            ]
        );
    }

    #[test]
    fn permission_wait_survives_visible_and_offscreen_topology_changes() {
        let mut world = World::new(bounds(), 492);
        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.enter_permission_wait(Vec2::new(600.0, 450.0));
        world.apply_layout(DesktopLayout::single(Rect::new(
            Vec2::ZERO,
            Vec2::new(900.0, 700.0),
        )));
        assert!(world.permission_waiting());

        world.apply_layout(DesktopLayout::single(Rect::new(
            Vec2::new(2000.0, -600.0),
            Vec2::new(3200.0, 300.0),
        )));
        assert!(world.permission_waiting());
        assert_eq!(world.current_task(), "permission_wait");
        for _ in 0..240 {
            world.tick();
            assert!(world.permission_waiting());
        }
    }

    #[test]
    fn graceful_exit_latch_rejects_permission_transitions_and_survives_hotplug() {
        let mut world = World::new(bounds(), 493);
        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.request_graceful_exit();
        world.enter_permission_wait(Vec2::new(600.0, 450.0));
        world.leave_permission_wait();
        assert!(world.graceful_exit_requested());
        assert_eq!(world.current_task(), "graceful_exit");

        world.apply_layout(DesktopLayout::single(Rect::new(
            Vec2::new(-200.0, -100.0),
            Vec2::new(1400.0, 900.0),
        )));
        assert!(world.graceful_exit_requested());
        assert_eq!(world.current_task(), "graceful_exit");
        for _ in 0..120 * 15 {
            world.tick();
            if world.graceful_exit_complete() {
                break;
            }
        }
        assert!(world.graceful_exit_complete());
    }

    #[test]
    fn poke_mud_extends_tracking_window() {
        let mut w = World::new(bounds(), 20);
        assert!(w.goose.track_mud_end_time < w.now());
        assert_eq!(w.poke(PokeAction::Mud), PokeOutcome::Applied);
        assert!(
            (w.goose.track_mud_end_time - (w.now() + 10.0)).abs() < f64::EPSILON,
            "the default direct mud command must use the ADR 0039 ten-second window"
        );
    }

    #[test]
    fn apply_options_hot_applies_parameters_and_footmark_timing() {
        let mut w = World::new(bounds(), 201);
        let parameters = ParametersTable {
            walk_speed: 123.0,
            run_speed: 234.0,
            duration_to_track_mud: 4.25,
            ..ParametersTable::default()
        };
        let footmarks = FootMarkTiming {
            lifetime: 3.5,
            shrink_time: 1.25,
        };

        w.apply_options(WorldOptions {
            parameters,
            footmarks,
            ..WorldOptions::default()
        });

        assert_eq!(w.goose.parameters.walk_speed, 123.0);
        assert_eq!(w.goose.parameters.run_speed, 234.0);
        assert_eq!(w.footmark_timing(), footmarks);
        assert_eq!(w.poke(PokeAction::Mud), PokeOutcome::Applied);
        assert!((w.goose.track_mud_end_time - (w.now() + 4.25)).abs() < f64::EPSILON);
    }

    #[test]
    fn poke_note_uses_collect_window_path() {
        let mut w = world_with_collect(21);
        assert_eq!(w.poke(PokeAction::Note), PokeOutcome::Applied);
        w.tick();
        assert_eq!(w.current_task(), "collect_window");
        assert!(matches!(
            w.take_collect_window_commands().as_slice(),
            [CollectWindowCommand::Spawn { .. }]
        ));
    }

    #[test]
    fn full_owned_prop_capacity_is_busy_and_recovers_without_capability_loss() {
        for seed in [21, 73, 88] {
            let mut w = world_with_collect(seed);
            w.set_collect_window_capacity(false);
            assert_eq!(w.poke(PokeAction::Note), PokeOutcome::Busy);
            assert_eq!(w.poke(PokeAction::Meme), PokeOutcome::Busy);
            w.force_collect_window(CollectWindowKind::Note);
            assert!(w.pending_collect.is_none());
            assert!(w.options.collect_window.active());
            w.set_collect_window_capacity(true);
            assert_eq!(w.poke(PokeAction::Note), PokeOutcome::Applied);
            w.tick();
            assert!(matches!(
                w.take_collect_window_commands().as_slice(),
                [CollectWindowCommand::Spawn {
                    payload: crate::CollectWindowPayload::Note { .. },
                    ..
                }]
            ));
        }
    }

    #[test]
    fn poke_unsupported_collect_reports_unsupported() {
        let mut w = World::new(bounds(), 22);
        assert_eq!(w.poke(PokeAction::Meme), PokeOutcome::Unsupported);
        w.tick();
        assert_ne!(w.current_task(), "collect_window");
    }

    #[test]
    fn poke_collect_reports_busy_during_collect_window() {
        let mut w = world_with_collect(23);
        assert_eq!(w.poke(PokeAction::Meme), PokeOutcome::Applied);
        w.tick();
        assert_eq!(w.current_task(), "collect_window");
        assert_eq!(w.poke(PokeAction::Note), PokeOutcome::Busy);
    }

    #[test]
    fn poke_nab_reports_unsupported_without_cursor_capability() {
        let mut w = World::new(bounds(), 24);
        assert_eq!(w.poke(PokeAction::Nab), PokeOutcome::Unsupported);
    }

    #[test]
    fn poke_nab_starts_on_next_tick_when_supported() {
        let mut w = World::with_options(
            bounds(),
            25,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(WanderTask::new());
        let pointer = w.goose.rig.beak_tip;
        w.set_pointer(Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        });
        assert_eq!(w.poke(PokeAction::Nab), PokeOutcome::Applied);
        w.tick();
        assert_eq!(w.current_task(), "nab_mouse");
    }

    #[test]
    fn apply_options_rebuilds_pickable_tasks() {
        let mut w = World::new(bounds(), 26);
        assert_eq!(w.pickable.len(), 5);
        w.apply_options(WorldOptions {
            mouse_steal: MouseStealOptions::with_backend_support(true),
            collect_window: CollectWindowOptions::with_backend_support(
                CollectWindowCapabilities {
                    spawn_note: true,
                    spawn_image: true,
                    move_window: true,
                    set_passthrough: true,
                    synthesize_text: true,
                },
                1,
                1,
            ),
            ..WorldOptions::default()
        });
        assert_eq!(w.pickable.len(), 7);
    }

    #[test]
    fn mischievous_bias_duplicates_only_already_active_pickable_tasks() {
        let collect_options = CollectWindowOptions::with_backend_support(
            CollectWindowCapabilities {
                spawn_note: true,
                spawn_image: true,
                move_window: true,
                set_passthrough: true,
                synthesize_text: true,
            },
            1,
            1,
        );

        assert_eq!(
            World::pickable_for(WorldOptions::default(), MoodKind::Mischievous, false, false).len(),
            5,
            "unsupported defaults retain weighted wander plus safe edge wrapping"
        );
        assert_eq!(
            World::pickable_for(
                WorldOptions {
                    mouse_steal: MouseStealOptions::with_backend_support(true),
                    ..WorldOptions::default()
                },
                MoodKind::Mischievous,
                false,
                false
            )
            .len(),
            7,
            "active nab appears once normally and once as mischievous bias"
        );
        assert_eq!(
            World::pickable_for(
                WorldOptions {
                    collect_window: collect_options,
                    ..WorldOptions::default()
                },
                MoodKind::Mischievous,
                false,
                false
            )
            .len(),
            7,
            "active collect appears once normally and once as mischievous bias"
        );
        assert_eq!(
            World::pickable_for(
                WorldOptions {
                    mouse_steal: MouseStealOptions::with_backend_support(true),
                    collect_window: collect_options,
                    ..WorldOptions::default()
                },
                MoodKind::Mischievous,
                false,
                false
            )
            .len(),
            9,
            "mischievous mode duplicates only the two already-enabled mischief tasks"
        );
        assert_eq!(
            World::pickable_for(
                WorldOptions {
                    mouse_steal: MouseStealOptions::with_backend_support(true),
                    collect_window: collect_options,
                    ..WorldOptions::default()
                },
                MoodKind::Mischievous,
                true,
                true,
            )
            .len(),
            5,
            "manners suppress random mischief and Autumn chase, retaining calm roaming"
        );
        assert_eq!(
            World::pickable_for(WorldOptions::default(), MoodKind::Content, false, true).len(),
            6,
            "Autumn chase is pickable when an unkicked pile exists"
        );
    }

    #[test]
    fn on_hour_double_honk_emits_two_honks_without_same_hour_repeat() {
        let mut w = World::with_options(
            bounds(),
            260,
            WorldOptions {
                mood: MoodOptions {
                    dynamic_moods: false,
                    intensity: MoodIntensity::Normal,
                },
                hourly_honk: HourlyHonkOptions {
                    on_hour_double_honk: true,
                },
                ..WorldOptions::default()
            },
        );
        w.set_local_time(LocalTime {
            day: 20260628,
            hour: 13,
            minute: 0,
            second: 0,
        });

        let mut sounds = Vec::new();
        for _ in 0..100 {
            w.tick();
            sounds.extend(w.take_sounds());
        }

        assert_eq!(
            sounds,
            vec![Sound::high_honk(), Sound::high_honk()],
            "top-of-hour behavior is exactly one immediate honk plus one delayed honk"
        );

        for _ in 0..200 {
            w.tick();
            sounds.extend(w.take_sounds());
        }
        assert_eq!(
            sounds.len(),
            2,
            "holding the same top-of-hour snapshot does not repeat within that local hour"
        );

        w.set_local_time(LocalTime {
            day: 20260628,
            hour: 14,
            minute: 0,
            second: 0,
        });
        w.tick();
        assert_eq!(w.take_sounds(), vec![Sound::high_honk()]);
    }

    #[test]
    fn quiet_hours_suppress_on_hour_honks_but_direct_honk_still_works() {
        let mut w = World::with_options(
            bounds(),
            261,
            WorldOptions {
                mood: MoodOptions {
                    dynamic_moods: false,
                    intensity: MoodIntensity::Normal,
                },
                hourly_honk: HourlyHonkOptions {
                    on_hour_double_honk: true,
                },
                schedule: ScheduleOptions {
                    quiet_hours_enabled: true,
                    quiet_start: LocalMinute::new(22, 0).unwrap(),
                    quiet_end: LocalMinute::new(8, 0).unwrap(),
                    ..ScheduleOptions::default()
                },
                ..WorldOptions::default()
            },
        );
        w.set_local_time(LocalTime {
            day: 20260628,
            hour: 23,
            minute: 0,
            second: 0,
        });
        assert!(w.manners_active());
        for _ in 0..100 {
            w.tick();
        }
        assert!(w.take_sounds().is_empty(), "on-hour honks are suppressed");
        assert_eq!(w.poke(PokeAction::Honk), PokeOutcome::Applied);
        assert_eq!(w.take_sounds(), vec![Sound::honk()]);
    }

    #[test]
    fn calm_goose_suppresses_on_hour_honks_but_direct_honk_still_works() {
        let mut w = World::with_options(
            bounds(),
            265,
            WorldOptions {
                appearance: AppearanceOptions {
                    calm_goose: true,
                    ..AppearanceOptions::default()
                },
                mood: MoodOptions {
                    dynamic_moods: false,
                    intensity: MoodIntensity::Normal,
                },
                hourly_honk: HourlyHonkOptions {
                    on_hour_double_honk: true,
                },
                ..WorldOptions::default()
            },
        );
        w.set_local_time(LocalTime {
            day: 20260701,
            hour: 9,
            minute: 0,
            second: 0,
        });
        assert!(w.manners_active());
        for _ in 0..100 {
            w.tick();
        }
        assert!(w.take_sounds().is_empty(), "on-hour honks are suppressed");
        assert_eq!(w.poke(PokeAction::Honk), PokeOutcome::Applied);
        assert_eq!(w.take_sounds(), vec![Sound::honk()]);
    }

    #[test]
    fn presence_fullscreen_suppresses_random_window_ride() {
        let mut w = World::with_options(
            bounds(),
            262,
            WorldOptions {
                foreign_window: ForeignWindowOptions::with_backend_support(true, true),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(WanderTask::new());
        w.set_presence(PresenceSnapshot::fullscreen());
        w.set_foreign_window_drag(Some(ForeignWindowSnapshot {
            id: ForeignWindowId(1),
            rect: Rect {
                min: Vec2::new(100.0, 100.0),
                max: Vec2::new(300.0, 200.0),
            },
            ride_anchor: Vec2::new(200.0, 90.0),
        }));
        w.tick();
        assert_eq!(w.current_task(), "wander");
    }

    #[test]
    fn direct_nab_is_allowed_during_manners_when_capable() {
        let mut w = World::with_options(
            bounds(),
            263,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(WanderTask::new());
        w.set_pointer(Pointer {
            pos: w.goose.rig.beak_tip,
            present: true,
            left_down: false,
        });
        w.set_presence(PresenceSnapshot::do_not_disturb());
        assert!(w.manners_active());
        assert_eq!(w.poke(PokeAction::Nab), PokeOutcome::Applied);
        w.tick();
        assert_eq!(w.current_task(), "nab_mouse");
    }

    #[test]
    fn autumn_spawns_only_inside_autumn_window() {
        let mut options = WorldOptions::default();
        options.schedule.seasonal = true;
        let mut w = World::with_options(bounds(), 264, options);
        w.set_local_time(LocalTime {
            day: 20260831,
            hour: 12,
            minute: 0,
            second: 0,
        });
        for _ in 0..1300 {
            w.tick();
        }
        assert!(w.autumn().piles().is_empty());

        w.set_local_time(LocalTime {
            day: 20260901,
            hour: 12,
            minute: 0,
            second: 0,
        });
        for _ in 0..1300 {
            w.tick();
        }
        assert!(!w.autumn().piles().is_empty());
    }

    #[test]
    fn disabling_either_seasonal_switch_clears_leaves_until_enabled_again() {
        for disable_master in [true, false] {
            let mut options = WorldOptions::default();
            options.schedule.seasonal = true;
            let mut world = World::with_options(bounds(), 265, options);
            world.set_local_time(LocalTime {
                day: 20260915,
                hour: 12,
                minute: 0,
                second: 0,
            });
            for _ in 0..1_300 {
                world.tick();
            }
            assert!(!world.autumn().piles().is_empty());

            if disable_master {
                options.schedule.seasonal = false;
            } else {
                options.schedule.autumn = false;
            }
            world.apply_options(options);
            assert!(world.autumn().piles().is_empty());
            for _ in 0..1_300 {
                world.tick();
            }
            assert!(world.autumn().piles().is_empty());

            options.schedule.seasonal = true;
            options.schedule.autumn = true;
            world.apply_options(options);
            for _ in 0..1_300 {
                world.tick();
            }
            assert!(!world.autumn().piles().is_empty());
        }
    }

    #[test]
    fn apply_options_abandons_unsupported_active_nab() {
        let mut w = World::with_options(
            bounds(),
            27,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                ..WorldOptions::default()
            },
        );
        w.current = Box::new(NabMouseTask::new());
        w.apply_options(WorldOptions::default());
        assert_eq!(w.current_task(), "wander");
    }

    #[test]
    fn apply_options_releases_active_collect_window() {
        let mut w = world_with_collect(28);
        assert_eq!(w.poke(PokeAction::Meme), PokeOutcome::Applied);
        w.tick();
        let request = match w.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        let id = CollectWindowId(7);
        w.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id,
            request,
            kind: CollectWindowKind::Meme,
            rect: Rect {
                min: Vec2::new(300.0, 200.0),
                max: Vec2::new(500.0, 320.0),
            },
            alive: true,
            close_origin: None,
        }));
        w.apply_options(WorldOptions::default());
        assert_eq!(w.current_task(), "wander");
        assert_eq!(
            w.take_collect_window_commands(),
            vec![
                CollectWindowCommand::SetPassthrough {
                    id,
                    passthrough: false
                },
                CollectWindowCommand::Close { id }
            ]
        );
    }

    #[test]
    fn lost_prop_positioning_cancels_pending_motion_and_preserves_the_note() {
        let mut world = world_with_collect(28);
        assert_eq!(world.poke(PokeAction::Note), PokeOutcome::Applied);
        world.tick();
        let request = match world.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        let id = CollectWindowId(7);
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id,
            request,
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::new(300.0, 200.0), Vec2::new(500.0, 320.0)),
            alive: true,
            close_origin: None,
        }));
        world
            .pending_collect_window_commands
            .push(CollectWindowCommand::Move {
                id,
                top_left: Vec2::new(310.0, 200.0),
            });
        world.set_collect_window_positioning(false);
        assert!(!world.is_collect_window_active());
        assert_eq!(
            world.take_collect_window_commands(),
            vec![CollectWindowCommand::SetPassthrough {
                id,
                passthrough: false
            }]
        );
        assert!(world
            .options
            .collect_window
            .kind_active(CollectWindowKind::Note));
    }

    fn stage_closed_note(world: &mut World, origin: CollectWindowCloseOrigin) {
        world.current = Box::new(CollectWindowTask::forced(CollectWindowKind::Note));
        world.interrupted = Some(Box::new(WanderTask::new()));
        world.tick();
        let request = match world.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(88),
            request,
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::ZERO, Vec2::ZERO),
            alive: false,
            close_origin: Some(origin),
        }));
    }

    #[test]
    fn user_close_can_react_then_chain_the_existing_bounded_nab() {
        let mut world = World::with_options(
            bounds(),
            500,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                collect_window: CollectWindowOptions::with_backend_support(
                    CollectWindowCapabilities {
                        spawn_note: true,
                        spawn_image: true,
                        move_window: true,
                        set_passthrough: true,
                        synthesize_text: true,
                    },
                    1,
                    1,
                ),
                ..WorldOptions::default()
            },
        );
        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.set_pointer(Pointer {
            pos: world.goose.rig.beak_tip,
            present: true,
            left_down: false,
        });
        stage_closed_note(&mut world, CollectWindowCloseOrigin::User);
        set_next_close_reaction(&mut world, true);
        world.tick();
        assert_eq!(world.current_task(), "annoyed_reaction");
        assert!(world.close_reaction_nab);

        for _ in 0..120 {
            world.set_pointer(Pointer {
                pos: world.goose.rig.beak_tip,
                present: true,
                left_down: false,
            });
            world.tick();
            if world.current_task() == "nab_mouse" {
                break;
            }
        }
        assert_eq!(world.current_task(), "nab_mouse");
        world.set_pointer(Pointer {
            pos: world.goose.rig.beak_tip,
            present: true,
            left_down: false,
        });
        world.tick();
        assert!(
            world
                .take_cursor_commands()
                .iter()
                .any(|command| matches!(command, CursorCommand::WarpTo(_))),
            "the reaction chains the already-bounded nab implementation"
        );
    }

    #[test]
    fn program_cleanup_and_negative_roll_do_not_start_close_reaction() {
        let mut program_close = world_with_collect(501);
        stage_closed_note(&mut program_close, CollectWindowCloseOrigin::Program);
        set_next_close_reaction(&mut program_close, true);
        program_close.tick();
        assert_eq!(program_close.current_task(), "wander");

        let mut negative_roll = world_with_collect(502);
        stage_closed_note(&mut negative_roll, CollectWindowCloseOrigin::User);
        set_next_close_reaction(&mut negative_roll, false);
        negative_roll.tick();
        assert_eq!(negative_roll.current_task(), "wander");
    }

    #[test]
    fn closing_a_lingering_honk_note_after_collection_still_reacts() {
        let mut world = world_with_collect(505);
        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.current = Box::new(WanderTask::new());
        world.interrupted = None;
        set_next_close_reaction(&mut world, true);
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(89),
            request: CollectWindowRequestId(1234),
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::ZERO, Vec2::ZERO),
            alive: false,
            close_origin: Some(CollectWindowCloseOrigin::User),
        }));

        world.tick();

        assert_eq!(world.current_task(), "annoyed_reaction");
        assert!(
            world.interrupted.is_some(),
            "normal roaming is resumed afterward"
        );
    }

    #[test]
    fn hidden_user_close_waits_for_a_visible_goose_and_reacts_once() {
        let mut world = world_with_collect(506);
        world.current = Box::new(WanderTask::new());
        world.interrupted = None;
        set_next_close_reaction(&mut world, true);
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(90),
            request: CollectWindowRequestId(1235),
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::new(100.0, 100.0), Vec2::new(300.0, 250.0)),
            alive: false,
            close_origin: Some(CollectWindowCloseOrigin::User),
        }));

        world.tick();
        assert_ne!(world.current_task(), "annoyed_reaction");
        assert_eq!(world.pending_collect_closes.len(), 1);

        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.tick();
        assert_eq!(world.current_task(), "annoyed_reaction");
        assert!(world.pending_collect_closes.is_empty());

        for _ in 0..240 {
            world.tick();
        }
        assert_ne!(world.current_task(), "annoyed_reaction");
        assert!(world.pending_collect_closes.is_empty());

        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(90),
            request: CollectWindowRequestId(1235),
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::new(100.0, 100.0), Vec2::new(300.0, 250.0)),
            alive: false,
            close_origin: Some(CollectWindowCloseOrigin::User),
        }));
        assert!(
            world.pending_collect_closes.is_empty(),
            "the same backend close evidence is exactly-once"
        );
    }

    #[test]
    fn unrelated_lingering_close_does_not_abort_newer_collect_request() {
        let mut world = world_with_collect(507);
        place_static_goose(&mut world, Vec2::new(500.0, 400.0));
        world.current = Box::new(CollectWindowTask::forced(CollectWindowKind::Meme));
        world.interrupted = Some(Box::new(WanderTask::new()));
        world.tick();
        let active_request = match world.take_collect_window_commands().as_slice() {
            [CollectWindowCommand::Spawn { request, .. }] => *request,
            other => panic!("unexpected commands: {other:?}"),
        };
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(91),
            request: active_request,
            kind: CollectWindowKind::Meme,
            rect: Rect::new(Vec2::new(300.0, 200.0), Vec2::new(500.0, 350.0)),
            alive: true,
            close_origin: None,
        }));
        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(89),
            request: CollectWindowRequestId(1234),
            kind: CollectWindowKind::Note,
            rect: Rect::new(Vec2::new(100.0, 100.0), Vec2::new(280.0, 220.0)),
            alive: false,
            close_origin: Some(CollectWindowCloseOrigin::User),
        }));
        set_next_close_reaction(&mut world, true);

        world.tick();
        assert_eq!(world.current_task(), "collect_window");
        assert_eq!(world.pending_collect_closes.len(), 1);
        assert_eq!(
            world
                .collect_window_snapshot
                .map(|snapshot| snapshot.request),
            Some(active_request),
            "unrelated close must not replace the active request snapshot"
        );

        world.set_collect_window_snapshot(Some(CollectWindowSnapshot {
            id: CollectWindowId(91),
            request: active_request,
            kind: CollectWindowKind::Meme,
            rect: Rect::new(Vec2::new(300.0, 200.0), Vec2::new(500.0, 350.0)),
            alive: false,
            close_origin: Some(CollectWindowCloseOrigin::Program),
        }));
        world.tick();
        assert_eq!(world.current_task(), "annoyed_reaction");
        assert_eq!(world.pending_collect_closes.len(), 1);
        world.tick();
        assert!(world.pending_collect_closes.is_empty());
    }

    #[test]
    fn unsupported_cursor_and_manners_keep_user_close_reaction_visible_only() {
        let mut unsupported = world_with_collect(503);
        place_static_goose(&mut unsupported, Vec2::new(500.0, 400.0));
        unsupported.set_pointer(Pointer {
            pos: unsupported.goose.rig.beak_tip,
            present: true,
            left_down: false,
        });
        stage_closed_note(&mut unsupported, CollectWindowCloseOrigin::User);
        set_next_close_reaction(&mut unsupported, true);
        unsupported.tick();
        assert_eq!(unsupported.current_task(), "annoyed_reaction");
        assert!(!unsupported.close_reaction_nab);
        for _ in 0..120 {
            unsupported.tick();
        }
        assert_ne!(unsupported.current_task(), "nab_mouse");
        assert!(unsupported.take_cursor_commands().is_empty());

        let mut manners = World::with_options(
            bounds(),
            504,
            WorldOptions {
                mouse_steal: MouseStealOptions::with_backend_support(true),
                collect_window: CollectWindowOptions::with_backend_support(
                    CollectWindowCapabilities {
                        spawn_note: true,
                        spawn_image: true,
                        move_window: true,
                        set_passthrough: true,
                        synthesize_text: true,
                    },
                    1,
                    1,
                ),
                ..WorldOptions::default()
            },
        );
        place_static_goose(&mut manners, Vec2::new(500.0, 400.0));
        manners.set_presence(PresenceSnapshot::do_not_disturb());
        manners.set_pointer(Pointer {
            pos: manners.goose.rig.beak_tip,
            present: true,
            left_down: false,
        });
        stage_closed_note(&mut manners, CollectWindowCloseOrigin::User);
        set_next_close_reaction(&mut manners, true);
        manners.tick();
        assert_eq!(manners.current_task(), "annoyed_reaction");
        assert!(!manners.close_reaction_nab);
        manners.tick();
        assert!(manners.take_sounds().is_empty(), "DND reaction stays quiet");
    }

    #[test]
    fn user_close_reaction_distribution_stays_near_thirty_percent() {
        let mut rng = SplitMix64::seed(0xC105_E300);
        let trials = 20_000;
        let reactions = (0..trials)
            .filter(|_| World::user_close_reaction_roll(&mut rng))
            .count();
        let rate = reactions as f64 / trials as f64;
        assert!(
            (rate - USER_CLOSE_REACTION_PROBABILITY).abs() <= 0.02,
            "observed reaction rate {rate:.4}"
        );
    }

    #[test]
    fn remote_walk_to_clamps_arrives_lingers_and_returns_to_the_deck() {
        let mut w = World::new(bounds(), 901);
        while w.current_task() == "first_ux" {
            w.tick();
        }
        assert_eq!(w.walk_to(Vec2::new(-5_000.0, 200.0)), PokeOutcome::Applied);
        assert_eq!(w.current_task(), "walk_to");
        let mut arrived = false;
        for _ in 0..(120 * 30) {
            w.tick();
            if w.current_task() != "walk_to" {
                break;
            }
            arrived |= w.layout().contains(w.goose_position())
                && (w.goose_position().y - 200.0).abs() < 8.0;
        }
        assert!(arrived, "the clamped target stays on a real monitor");
        assert_ne!(w.current_task(), "walk_to", "the walk ends after its dwell");
    }

    #[test]
    fn remote_requests_respect_manners_while_local_pokes_keep_their_gates() {
        let mut w = World::new(bounds(), 902);
        w.set_presence(PresenceSnapshot::do_not_disturb());
        assert!(w.manners_active());
        assert_eq!(w.remote_poke(PokeAction::Honk), PokeOutcome::Quiet);
        assert_eq!(w.remote_poke(PokeAction::Mud), PokeOutcome::Quiet);
        assert_eq!(w.walk_to(Vec2::new(10.0, 10.0)), PokeOutcome::Quiet);
        assert_eq!(
            w.place_toy(ToyKind::Ball, Vec2::new(10.0, 10.0)),
            Err(PokeOutcome::Quiet)
        );
        assert_eq!(w.remote_poke(PokeAction::Wander), PokeOutcome::Applied);
        assert_eq!(w.poke(PokeAction::Honk), PokeOutcome::Applied);

        w.set_presence(PresenceSnapshot::default());
        assert_eq!(w.remote_poke(PokeAction::Honk), PokeOutcome::Applied);
        w.request_graceful_exit();
        assert_eq!(w.walk_to(Vec2::new(10.0, 10.0)), PokeOutcome::Busy);
        assert!(w.toys().is_empty());
    }

    fn settled(seed: u64) -> World {
        let mut w = World::new(bounds(), seed);
        while w.current_task() == "first_ux" {
            w.tick();
        }
        w
    }

    #[test]
    fn placing_a_ball_sends_the_goose_to_kick_it() {
        let mut w = settled(903);
        let id = w.place_toy(ToyKind::Ball, Vec2::new(400.0, 400.0)).unwrap();
        assert_eq!(w.current_task(), "toy_play");
        let mut kicked = false;
        for _ in 0..(120 * 15) {
            w.tick();
            kicked |= w
                .toys()
                .get(id)
                .is_some_and(|toy| toy.last_kicked.is_some());
            if kicked {
                break;
            }
        }
        assert!(kicked);
        assert!(w.layout().contains(w.toys().get(id).unwrap().position));
    }

    #[test]
    fn the_goose_eats_crumbs() {
        let mut w = settled(904);
        let id = w
            .place_toy(ToyKind::Crumbs, Vec2::new(300.0, 350.0))
            .unwrap();
        for _ in 0..(120 * 15) {
            w.tick();
            if w.toys().get(id).is_none() {
                break;
            }
        }
        assert!(w.toys().get(id).is_none(), "crumbs were eaten");
    }

    #[test]
    fn the_goose_fetches_a_stick_back_to_the_pointer() {
        let mut w = settled(905);
        let pointer = Vec2::new(150.0, 150.0);
        w.set_pointer(Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        });
        let id = w
            .place_toy(ToyKind::Stick, Vec2::new(600.0, 450.0))
            .unwrap();
        let mut carried = false;
        for _ in 0..(120 * 20) {
            w.tick();
            let toy = *w.toys().get(id).unwrap();
            carried |= toy.carried;
            if carried && !toy.carried {
                break;
            }
        }
        let toy = *w.toys().get(id).unwrap();
        assert!(carried, "the goose picked the stick up");
        assert!(!toy.carried, "and dropped it again");
        assert!(
            Vec2::distance(toy.position, pointer) < 80.0,
            "near the pointer"
        );
        assert!(!w.move_toy(ToyId(9_999), pointer).eq(&PokeOutcome::Applied));
    }

    #[test]
    fn toys_are_bounded_and_movable_until_carried() {
        let mut w = settled(906);
        let id = w
            .place_toy(ToyKind::Ball, Vec2::new(-500.0, 100.0))
            .unwrap();
        assert!(w.layout().contains(w.toys().get(id).unwrap().position));
        assert_eq!(
            w.move_toy(id, Vec2::new(500.0, 400.0)),
            PokeOutcome::Applied
        );
        for _ in 1..crate::toys::MAX_TOYS {
            w.place_toy(ToyKind::Crumbs, Vec2::new(100.0, 100.0))
                .unwrap();
        }
        assert_eq!(
            w.place_toy(ToyKind::Stick, Vec2::new(100.0, 100.0)),
            Err(PokeOutcome::Busy)
        );
    }
}
