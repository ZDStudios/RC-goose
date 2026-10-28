//! The goose's task state machine — the AI.
//!
//! The in-tree task model uses a default roaming state that picks a random *pickable* task
//! via the biased [`Deck`](crate::rng::Deck);
//! a task only sets `target_pos` / speed / acceleration and the engine auto-locomotes
//! (see [`crate::locomotion`]). A scripted **FirstUX** intro runs once before roaming.
//!
//! This `Task` trait is the documented internal extension seam (plan §18) — adding a
//! behavior means adding a `Task` impl and registering it; there is no external mod ABI.
//! Richer autonomous tasks such as Autumn build on the same internal seam; there is no
//! external mod ABI.

use crate::autumn::{AutumnPileId, AutumnPileTarget};
#[cfg(test)]
use crate::collect_window::CollectWindowCloseOrigin;
use crate::collect_window::{
    CollectWindowCommand, CollectWindowKind, CollectWindowOptions, CollectWindowPayload,
    CollectWindowRequestId, CollectWindowSnapshot,
};
use crate::cursor::{CursorCommand, MouseStealOptions, TimingOptions};
use crate::entity::GooseEntity;
use crate::foreign_window::{ForeignWindowOptions, ForeignWindowSnapshot};
use crate::interaction::Pointer;
use crate::layout::{DesktopLayout, ExposedEdge};
use crate::math::{Rect, Vec2};
use crate::rng::{RandomSource, SplitMix64};
use crate::sound::Sound;
use crate::toys::{ToyId, ToyKind, ToyState};

/// Verified wander timings (`config.ini`): seconds. Config-driven values arrive with the
/// TOML loader in a later round; these are the defaults.
pub const FIRST_WANDER_TIME: f32 = 20.0;
pub const MIN_WANDERING_TIME: f32 = 20.0;
pub const MAX_WANDERING_TIME: f32 = 40.0;

/// How long the click→charge "hyper" burst lasts, in seconds (M6, plan §5.6 hyper).
pub const HYPER_DURATION: f64 = 2.5;
pub const AFFECTION_FOLLOW_SECONDS: f64 = 15.0;
pub const AFFECTION_FOLLOW_GAP: f32 = 90.0;
const COLLECT_SPAWN_TIMEOUT: f64 = 3.0;
const COLLECT_VISIBLE_DWELL: f64 = 4.0;
const COLLECT_PICKUP_DISTANCE: f32 = 42.0;
const COLLECT_RELEASE_DISTANCE: f32 = 5.0;
const EDGE_CORNER_INSET: f32 = 56.0;
const OFFSCREEN_TRAVEL: f32 = 220.0;
const EDGE_ENTRY_DEPTH: f32 = 220.0;
const ANNOYED_REACTION_DURATION: f64 = 0.75;
const LIFECYCLE_EXIT_TARGET_SECONDS: f32 = 12.0;
const LIFECYCLE_EXIT_MIN_SPEED: f32 = 200.0;
const LIFECYCLE_EXIT_MAX_SPEED: f32 = 400.0;
const LIFECYCLE_EXIT_MIN_ACCELERATION: f32 = 2300.0;
const LIFECYCLE_EXIT_ACCELERATION_SECONDS: f32 = 0.25;
const LIFECYCLE_EXIT_STEP_TIME: f32 = 0.14;

/// Per-tick context handed to a running task.
pub struct TaskCtx<'a> {
    /// World clock (seconds).
    pub now: f64,
    /// Fixed tick duration.
    pub dt: f32,
    /// Roaming bounds (the virtual-desktop space).
    pub bounds: Rect,
    /// Actual visible monitor regions; ordinary targets must stay inside one of them.
    pub layout: &'a DesktopLayout,
    /// Whether any part of the current goose pose intersects a real monitor.
    pub goose_visible: bool,
    /// Shared RNG for target/dwell choices.
    pub rng: &'a mut SplitMix64,
    /// Sound requests a task wants played this frame.
    pub sounds: &'a mut Vec<Sound>,
    /// Cursor commands a task wants the platform backend to apply this frame.
    pub cursor_commands: &'a mut Vec<CursorCommand>,
    /// Collect-window commands a task wants the platform backend to apply this frame.
    pub collect_window_commands: &'a mut Vec<CollectWindowCommand>,
    /// Last pointer snapshot in world/desktop coordinates.
    pub pointer: Pointer,
    /// Mouse-stealing tuning and backend support.
    pub mouse_steal: MouseStealOptions,
    /// Foreign-window tuning and backend support.
    pub foreign_window: ForeignWindowOptions,
    /// Collect-window tuning/content and backend support.
    pub collect_window: CollectWindowOptions,
    /// The user-dragged foreign window currently being watched, if any.
    pub dragged_window: Option<ForeignWindowSnapshot>,
    /// The backend-reported state of the active collect window, if any.
    pub collect_window_snapshot: Option<CollectWindowSnapshot>,
    /// The goose is in its post-pat calm window (suppresses spontaneous honks; M6 §5.9).
    pub calm: bool,
    /// Runtime timing values loaded from config or defaults.
    pub timing: TimingOptions,
    /// Built-in Autumn leaf piles currently available as task targets.
    pub autumn_piles: &'a [AutumnPileTarget],
    /// Remote-control toys; only the toy-play task changes them.
    pub toys: &'a mut ToyState,
}

/// A goose behavior. Tasks set targets/params only; locomotion is the engine's job.
pub trait Task {
    /// Stable identifier (for `do <id>` pokes and debugging).
    fn id(&self) -> &'static str;
    /// Current anticipation beat; visual only, never grants a capability or delays cancellation.
    fn anticipating(&self) -> bool {
        false
    }
    /// Selected collect-window content, when this task controls one. Used to cancel one
    /// disabled media class without disabling the still-valid sibling class.
    fn collect_kind(&self) -> Option<CollectWindowKind> {
        None
    }
    /// Identity of the active collect request, if this task has emitted one.
    fn collect_request(&self) -> Option<(CollectWindowRequestId, CollectWindowKind)> {
        None
    }
    /// Update the stable anchor used by the permission-wait task after a display-layout
    /// change. Other tasks deliberately ignore this platform-neutral lifecycle signal.
    fn set_permission_wait_anchor(&mut self, _anchor: Vec2) {}
    /// Advance one tick; return `true` when finished (the engine then picks the next task).
    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool;
}

fn arrived(goose: &GooseEntity, tol: f32) -> bool {
    Vec2::distance(goose.position, goose.target_pos) < tol
}

/// Translate an interaction point for the beak into the body/ground target consumed by
/// locomotion. Driving the body directly to the interaction point can never satisfy a beak-based
/// arrival gate in side view because the beak remains a neck-height away after the body stops.
fn beak_locomotion_target(goose: &GooseEntity, beak_target: Vec2) -> Vec2 {
    beak_target - (goose.rig.beak_tip - goose.position)
}

fn random_point(ctx: &mut TaskCtx) -> Vec2 {
    ctx.layout.sample_point(ctx.rng)
}

/// Roam to random points for a random dwell, occasionally tracking mud. The default
/// pickable task (the original `Task_Wander`, with mud folded in for now).
#[derive(Default)]
pub struct WanderTask {
    end_time: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct EdgePassage {
    exit_edge: ExposedEdge,
    exit_target: Vec2,
    entry_point: Vec2,
    return_target: Vec2,
}

impl EdgePassage {
    fn wrap(goose: &GooseEntity, ctx: &mut TaskCtx) -> Self {
        let region = ctx.layout.region_at(goose.position);
        let exit_edge = ctx.layout.sample_exposed_edge(ctx.rng, region);
        let exit_point = exit_edge.point_near(goose.position, EDGE_CORNER_INSET);
        let entry_edge = ctx.layout.opposite_exposed_edge(exit_edge, exit_point);
        let entry_boundary = entry_edge.point_near(exit_point, EDGE_CORNER_INSET);
        Self {
            exit_edge,
            exit_target: exit_point + exit_edge.direction().outward() * OFFSCREEN_TRAVEL,
            entry_point: entry_boundary + entry_edge.direction().outward() * OFFSCREEN_TRAVEL,
            return_target: ctx
                .layout
                .clamp_point(entry_boundary + entry_edge.direction().inward() * EDGE_ENTRY_DEPTH),
        }
    }

    fn depart(goose: &GooseEntity, ctx: &DesktopLayout) -> Self {
        let exit_edge = ctx.nearest_exposed_edge(goose.position);
        let exit_point = exit_edge.point_near(goose.position, EDGE_CORNER_INSET);
        Self {
            exit_edge,
            exit_target: exit_point + exit_edge.direction().outward() * OFFSCREEN_TRAVEL,
            entry_point: Vec2::ZERO,
            return_target: Vec2::ZERO,
        }
    }

    fn extend_exit_if_needed(&mut self, goose: &GooseEntity, visible: bool) {
        if visible && arrived(goose, 8.0) {
            self.exit_target =
                self.exit_target + self.exit_edge.direction().outward() * OFFSCREEN_TRAVEL;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EdgeWrapState {
    Depart,
    Return,
}

/// An ordinary roaming beat that walks through a genuinely exposed edge and wraps, while
/// fully hidden, to the far opposite exposed edge. Shared monitor seams are never selected.
#[derive(Default)]
pub struct EdgeWrapTask {
    passage: Option<EdgePassage>,
    state: Option<EdgeWrapState>,
}

impl EdgeWrapTask {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Task for EdgeWrapTask {
    fn id(&self) -> &'static str {
        "edge_wrap"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        goose.current_speed = goose.parameters.walk_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        let passage = self
            .passage
            .get_or_insert_with(|| EdgePassage::wrap(goose, ctx));
        let state = self.state.get_or_insert(EdgeWrapState::Depart);
        match *state {
            EdgeWrapState::Depart => {
                goose.target_pos = passage.exit_target;
                passage.extend_exit_if_needed(goose, ctx.goose_visible);
                if !ctx.goose_visible {
                    // This is the only teleport in the wrap contract, and both its source and
                    // destination are fully outside real monitor regions.
                    goose.position = passage.entry_point;
                    goose.target_pos = passage.return_target;
                    goose.velocity = Vec2::ZERO;
                    *state = EdgeWrapState::Return;
                }
                false
            }
            EdgeWrapState::Return => {
                goose.target_pos = passage.return_target;
                arrived(goose, 6.0)
            }
        }
    }
}

/// Terminal lifecycle task: walk out through the nearest exposed edge and remain hidden.
#[derive(Default)]
pub struct GracefulExitTask {
    passage: Option<EdgePassage>,
    speed: Option<f32>,
}

/// Walks into the current topology after a hot-plug invalidated an offscreen/removed-monitor
/// position. The world stages the start fully outside an exposed edge before installing it.
pub struct EdgeEntryTask {
    target: Vec2,
}

impl EdgeEntryTask {
    pub fn new(target: Vec2) -> Self {
        Self { target }
    }
}

impl Task for EdgeEntryTask {
    fn id(&self) -> &'static str {
        "edge_entry"
    }

    fn run(&mut self, goose: &mut GooseEntity, _ctx: &mut TaskCtx) -> bool {
        goose.current_speed = goose.parameters.walk_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.target_pos = self.target;
        arrived(goose, 6.0)
    }
}

/// Seconds the goose lingers at a remotely requested spot before resuming its own plans.
pub const WALK_TO_DWELL: f64 = 3.0;

/// Walks to one requested desktop point (remote control), lingers briefly, then finishes so the
/// ordinary deck resumes. The world clamps the target into a real monitor before installing it.
pub struct WalkToTask {
    target: Vec2,
    arrived_at: Option<f64>,
}

impl WalkToTask {
    pub fn new(target: Vec2) -> Self {
        Self {
            target,
            arrived_at: None,
        }
    }
}

impl Task for WalkToTask {
    fn id(&self) -> &'static str {
        "walk_to"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        goose.current_speed = goose.parameters.run_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.target_pos = self.target;
        if self.arrived_at.is_none() && arrived(goose, 6.0) {
            self.arrived_at = Some(ctx.now);
        }
        self.arrived_at
            .is_some_and(|at| ctx.now - at >= WALK_TO_DWELL)
    }
}

/// Give up on a toy errand that cannot finish (unreachable toy, pointer far away).
pub const TOY_PLAY_TIMEOUT: f64 = 20.0;
const TOY_BEAK_REACH: f32 = 22.0;

/// Plays with one remote-control toy: kick a ball, fetch a stick back to the pointer, or eat
/// crumbs. Movement only; the toy physics live in [`ToyState`].
#[derive(Default)]
pub struct ToyPlayTask {
    target: Option<ToyId>,
    started: Option<f64>,
    return_to: Option<Vec2>,
}

impl ToyPlayTask {
    /// Play with whichever toy is available when the task starts.
    pub fn new() -> Self {
        Self::default()
    }

    /// Go straight for one toy (a freshly placed one).
    pub fn targeting(id: ToyId) -> Self {
        Self {
            target: Some(id),
            ..Self::default()
        }
    }
}

impl Task for ToyPlayTask {
    fn id(&self) -> &'static str {
        "toy_play"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        let started = *self.started.get_or_insert(ctx.now);
        if ctx.now - started > TOY_PLAY_TIMEOUT {
            ctx.toys.release_all();
            return true;
        }
        let id = match self.target {
            Some(id) => id,
            None => {
                let free: Vec<ToyId> = ctx
                    .toys
                    .toys()
                    .iter()
                    .filter(|toy| !toy.carried)
                    .map(|toy| toy.id)
                    .collect();
                if free.is_empty() {
                    return true;
                }
                let index = (ctx.rng.next_f64() * free.len() as f64) as usize;
                let id = free[index.min(free.len() - 1)];
                self.target = Some(id);
                id
            }
        };
        let Some(toy) = ctx.toys.get(id).copied() else {
            return true;
        };
        goose.current_speed = goose.parameters.run_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        match toy.kind {
            ToyKind::Ball => {
                if toy.last_kicked.is_some_and(|at| at >= started) {
                    return true;
                }
                // Aim a little past the ball so the goose runs through it.
                let through = (toy.position - goose.position).normalize() * 40.0;
                goose.target_pos = ctx.layout.clamp_point(toy.position + through);
                false
            }
            ToyKind::Crumbs => {
                goose.target_pos = beak_locomotion_target(goose, toy.position);
                if Vec2::distance(goose.rig.beak_tip, toy.position) < TOY_BEAK_REACH {
                    ctx.toys.remove(id);
                    return true;
                }
                false
            }
            ToyKind::Stick if !toy.carried => {
                goose.target_pos = beak_locomotion_target(goose, toy.position);
                if Vec2::distance(goose.rig.beak_tip, toy.position) < TOY_BEAK_REACH {
                    ctx.toys.pick_up(id);
                    self.return_to = Some(if ctx.pointer.present {
                        ctx.layout.clamp_point(ctx.pointer.pos)
                    } else {
                        random_point(ctx)
                    });
                }
                false
            }
            ToyKind::Stick => {
                let destination = *self.return_to.get_or_insert(goose.position);
                goose.target_pos = beak_locomotion_target(goose, destination);
                if Vec2::distance(goose.rig.beak_tip, destination) < TOY_BEAK_REACH * 2.0 {
                    ctx.toys.drop_toy(id, goose.rig.beak_tip);
                    return true;
                }
                false
            }
        }
    }
}

impl GracefulExitTask {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Task for GracefulExitTask {
    fn id(&self) -> &'static str {
        "graceful_exit"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        let passage = self
            .passage
            .get_or_insert_with(|| EdgePassage::depart(goose, ctx.layout));
        // Lifecycle movement deliberately ignores user-tunable speeds. Typical displays use the
        // ordinary run tier; large supported monitor walls scale only up to the normal charge
        // tier so the exit stays articulated without stretching the gait. Absurd topologies time
        // out fail-closed at the IPC layer instead of inventing a super-charge speed.
        let speed = *self.speed.get_or_insert_with(|| {
            (Vec2::distance(goose.position, passage.exit_target) / LIFECYCLE_EXIT_TARGET_SECONDS)
                .clamp(LIFECYCLE_EXIT_MIN_SPEED, LIFECYCLE_EXIT_MAX_SPEED)
        });
        goose.current_speed = speed;
        goose.current_acceleration =
            LIFECYCLE_EXIT_MIN_ACCELERATION.max(speed / LIFECYCLE_EXIT_ACCELERATION_SECONDS);
        goose.step_interval = LIFECYCLE_EXIT_STEP_TIME;
        goose.target_pos = passage.exit_target;
        passage.extend_exit_if_needed(goose, ctx.goose_visible);
        if !ctx.goose_visible {
            goose.velocity = Vec2::ZERO;
            goose.target_pos = goose.position;
        }
        false
    }
}

impl WanderTask {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default)]
pub struct AutumnLeafPileTask {
    target: Option<AutumnPileTarget>,
    target_pos: Option<Vec2>,
}

impl AutumnLeafPileTask {
    pub fn new() -> Self {
        Self::default()
    }

    fn target_still_exists(ctx: &TaskCtx, id: AutumnPileId) -> bool {
        ctx.autumn_piles.iter().any(|pile| pile.id == id)
    }
}

impl Task for AutumnLeafPileTask {
    fn id(&self) -> &'static str {
        "autumn_leaf_pile"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        let target = match self.target {
            Some(target) if Self::target_still_exists(ctx, target.id) => target,
            Some(_) => return true,
            None => {
                if ctx.autumn_piles.is_empty() {
                    return true;
                }
                let idx = (ctx.rng.next_f64() * ctx.autumn_piles.len() as f64).floor() as usize;
                let target = ctx.autumn_piles[idx.min(ctx.autumn_piles.len() - 1)];
                self.target = Some(target);
                target
            }
        };

        let target_pos = *self.target_pos.get_or_insert_with(|| {
            let toward_pile = (target.position - goose.position).normalize();
            let approach = if toward_pile == Vec2::ZERO {
                Vec2::new(1.0, 0.0)
            } else {
                toward_pile
            };
            target.position + approach * (target.radius * 4.0)
        });

        goose.current_speed = goose.parameters.run_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.target_pos = ctx.layout.clamp_point(target_pos);
        arrived(goose, 20.0)
    }
}

impl Task for WanderTask {
    fn id(&self) -> &'static str {
        "wander"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        // Re-assert walk-tier locomotion every tick so the goose cleanly resumes its stroll
        // after a transient interrupt (e.g. a hyper burst) left a faster tier on it.
        goose.current_speed = goose.parameters.walk_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;

        if self.end_time.is_none() {
            goose.target_pos = random_point(ctx);
            self.end_time = Some(
                ctx.now
                    + ctx
                        .rng
                        .range(ctx.timing.min_wandering_time, ctx.timing.max_wandering_time)
                        as f64,
            );
        }
        if arrived(goose, 1.5) {
            goose.target_pos = random_point(ctx);
            // Sometimes it honks for no reason at all — unless it's been freshly patted,
            // when it stays content and quiet for the calm window (§5.9). Mud tracking no
            // longer starts here: it comes home from off-screen puddle hops (ADR 0016), so
            // muddy feet read as an event with a story instead of a constant state.
            if !ctx.calm && ctx.rng.next_f64() < 0.25 {
                ctx.sounds.push(Sound::honk());
            }
        }
        ctx.now >= self.end_time.unwrap()
    }
}

/// A brief, invited walk behind the pointer. This task only moves the goose: it never emits
/// pointer or window commands and keeps a body-sized gap even when the cursor stops.
pub struct AffectionFollowTask {
    ends_at: f64,
}

impl AffectionFollowTask {
    pub fn new(now: f64) -> Self {
        Self {
            ends_at: now + AFFECTION_FOLLOW_SECONDS,
        }
    }
}

impl Task for AffectionFollowTask {
    fn id(&self) -> &'static str {
        "affection_follow"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        goose.current_speed = goose.parameters.walk_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.can_decelerate_immediately = true;
        let delta = ctx.pointer.pos - goose.position;
        let distance = delta.magnitude();
        goose.target_pos = if ctx.pointer.present && distance > AFFECTION_FOLLOW_GAP + 4.0 {
            ctx.layout
                .clamp_point(ctx.pointer.pos - delta.normalize() * AFFECTION_FOLLOW_GAP)
        } else {
            goose.position
        };
        !ctx.pointer.present || ctx.pointer.left_down || ctx.now >= self.ends_at
    }
}

/// Why the goose is leaving the screen, and what it brings back (ADR 0016).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExcursionKind {
    /// A long "errand" — gone ~1.5–2 minutes; the world may chain a collect-window
    /// prank onto the return.
    Errand,
    /// A quick hop just past the edge; the goose comes back seconds later tracking
    /// mud for `mud_secs` — as if it found a puddle out there.
    Puddle { mud_secs: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ExcursionState {
    Depart,
    Away { until: f64 },
    Return,
}

/// Waddle off-screen, disappear for a while, and come back somewhere else — the
/// "goose has other business" behavior (ADR 0016). Installed by the world as a timed
/// interrupt over wandering; parameters (exit/entry points, away time) are chosen by
/// the world's RNG so the whole thing stays deterministic per seed.
pub struct ExcursionTask {
    kind: ExcursionKind,
    /// Off-screen point the goose walks out to.
    exit: Vec2,
    /// Off-screen point it reappears at after the away timer.
    entry: Vec2,
    /// On-screen point it walks back in to.
    return_target: Vec2,
    away_secs: f32,
    state: ExcursionState,
}

impl ExcursionTask {
    pub fn new(
        kind: ExcursionKind,
        exit: Vec2,
        entry: Vec2,
        return_target: Vec2,
        away_secs: f32,
    ) -> Self {
        Self {
            kind,
            exit,
            entry,
            return_target,
            away_secs,
            state: ExcursionState::Depart,
        }
    }
}

impl Task for ExcursionTask {
    fn id(&self) -> &'static str {
        "excursion"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        match self.state {
            ExcursionState::Depart => {
                // A casual waddle out — deliberately NOT clamped to bounds.
                goose.current_speed = goose.parameters.walk_speed;
                goose.current_acceleration = goose.parameters.acceleration_normal;
                goose.target_pos = self.exit;
                if !ctx.goose_visible {
                    self.state = ExcursionState::Away {
                        until: ctx.now + self.away_secs as f64,
                    };
                } else if arrived(goose, 8.0) {
                    let boundary = ctx.layout.clamp_point(self.exit);
                    let mut outward = (self.exit - boundary).normalize();
                    if outward == Vec2::ZERO {
                        outward = ctx
                            .layout
                            .nearest_exposed_edge(goose.position)
                            .direction()
                            .outward();
                    }
                    self.exit = self.exit + outward * OFFSCREEN_TRAVEL;
                }
                false
            }
            ExcursionState::Away { until } => {
                // Parked out of sight; hold still (no footsteps, no drift).
                goose.target_pos = goose.position;
                goose.velocity = Vec2::ZERO;
                if ctx.now >= until {
                    // Reappear at the staged entry point (still off-screen) and walk in.
                    goose.position = self.entry;
                    goose.target_pos = self.return_target;
                    self.state = ExcursionState::Return;
                }
                false
            }
            ExcursionState::Return => {
                goose.current_speed = goose.parameters.walk_speed;
                goose.current_acceleration = goose.parameters.acceleration_normal;
                goose.target_pos = self.return_target;
                if arrived(goose, 6.0) {
                    if let ExcursionKind::Puddle { mud_secs } = self.kind {
                        // Came home through a puddle: track mud for a while.
                        goose.track_mud_end_time = ctx.now + mud_secs as f64;
                    }
                    return true;
                }
                false
            }
        }
    }
}

/// The click→charge reaction: a short, fast, erratic "hyper" burst (plan §5.6 hyper / M6).
/// Installed as a transient interrupt when you click the goose; when it finishes the world
/// restores whatever task was running before. The full self-triggered mood FSM is M13.
#[derive(Default)]
pub struct HyperTask {
    end_time: Option<f64>,
}

/// Short visible beat used after a person closes a Honk300 prop: the goose plants its feet,
/// raises its neck, and shakes its head before the world optionally chains a bounded nab.
pub struct AnnoyedReactionTask {
    until: Option<f64>,
    base_direction: f32,
    audible: bool,
    sounded: bool,
}

impl AnnoyedReactionTask {
    pub fn new(audible: bool) -> Self {
        Self {
            until: None,
            base_direction: 0.0,
            audible,
            sounded: false,
        }
    }
}

impl Task for AnnoyedReactionTask {
    fn id(&self) -> &'static str {
        "annoyed_reaction"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        let until = *self.until.get_or_insert_with(|| {
            self.base_direction = goose.direction;
            ctx.now + ANNOYED_REACTION_DURATION
        });
        goose.current_speed = 0.0;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.target_pos = goose.position;
        goose.velocity = Vec2::ZERO;
        goose.extending_neck = true;
        let phase = ((until - ctx.now) as f32 * 28.0).sin();
        goose.direction = self.base_direction + phase * 13.0;
        if self.audible && !self.sounded {
            self.sounded = true;
            ctx.sounds.push(Sound::high_honk());
        }
        ctx.now >= until
    }
}

impl HyperTask {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Task for HyperTask {
    fn id(&self) -> &'static str {
        "hyper"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        // Charge tier for the whole burst, re-asserted each tick.
        goose.current_speed = goose.parameters.charge_speed;
        goose.current_acceleration = goose.parameters.acceleration_charged;

        if self.end_time.is_none() {
            goose.target_pos = random_point(ctx);
            // An on-hour or mood transition may already have queued a honk this tick.
            // Hyper shares that immediate sound instead of stacking identical playback.
            if !ctx
                .sounds
                .iter()
                .any(|sound| matches!(sound, Sound::Honk(_)))
            {
                ctx.sounds.push(Sound::high_honk());
            }
            self.end_time = Some(ctx.now + HYPER_DURATION);
        } else if arrived(goose, 3.0) {
            // Bolt to a fresh spot the instant it arrives — erratic, no dwell.
            goose.target_pos = random_point(ctx);
            if ctx.rng.next_f64() < 0.5 {
                ctx.sounds.push(Sound::high_honk());
            }
        }
        ctx.now >= self.end_time.unwrap()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum NabState {
    SeekingMouse,
    DraggingMouseAway {
        original_vector_to_mouse: Vec2,
        grabbed_at: f64,
        target: Vec2,
    },
}

/// Cursor-stealing behavior (M7): chase the live pointer, grab it at the beak, then run a
/// bounded hyper-style burst while emitting platform-free cursor-warp commands.
pub struct NabMouseTask {
    state: NabState,
    bite_played: bool,
}

impl Default for NabMouseTask {
    fn default() -> Self {
        Self::new()
    }
}

impl NabMouseTask {
    pub fn new() -> Self {
        Self {
            state: NabState::SeekingMouse,
            bite_played: false,
        }
    }

    fn hyper_target(ctx: &mut TaskCtx) -> Vec2 {
        random_point(ctx)
    }
}

impl Task for NabMouseTask {
    fn anticipating(&self) -> bool {
        matches!(self.state, NabState::SeekingMouse)
    }
    fn id(&self) -> &'static str {
        "nab_mouse"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        if !ctx.mouse_steal.active() || !ctx.pointer.present {
            return true;
        }

        goose.current_speed = goose.parameters.charge_speed;
        goose.current_acceleration = goose.parameters.acceleration_charged;
        goose.extending_neck = true;

        match self.state {
            NabState::SeekingMouse => {
                goose.target_pos = ctx
                    .layout
                    .clamp_point(beak_locomotion_target(goose, ctx.pointer.pos));

                if Vec2::distance(goose.rig.beak_tip, ctx.pointer.pos)
                    <= ctx.mouse_steal.grab_distance
                {
                    let original_vector_to_mouse = ctx.pointer.pos - goose.rig.beak_tip;
                    let target = Self::hyper_target(ctx);
                    self.state = NabState::DraggingMouseAway {
                        original_vector_to_mouse,
                        grabbed_at: ctx.now,
                        target,
                    };
                    if !self.bite_played {
                        self.bite_played = true;
                        ctx.sounds.push(Sound::Bite);
                    }
                    ctx.cursor_commands.push(CursorCommand::WarpTo(
                        goose.rig.beak_tip + original_vector_to_mouse,
                    ));
                }
                false
            }
            NabState::DraggingMouseAway {
                original_vector_to_mouse,
                grabbed_at,
                mut target,
            } => {
                if arrived(goose, 3.0) {
                    target = Self::hyper_target(ctx);
                    self.state = NabState::DraggingMouseAway {
                        original_vector_to_mouse,
                        grabbed_at,
                        target,
                    };
                    if ctx.rng.next_f64() < 0.5 {
                        ctx.sounds.push(Sound::high_honk());
                    }
                }

                goose.target_pos = target;
                let desired_cursor = ctx
                    .layout
                    .clamp_point(goose.rig.beak_tip + original_vector_to_mouse);

                if Vec2::distance(ctx.pointer.pos, desired_cursor) > ctx.mouse_steal.drop_distance {
                    return true;
                }

                ctx.cursor_commands
                    .push(CursorCommand::WarpTo(desired_cursor));
                ctx.now - grabbed_at >= ctx.mouse_steal.succ_time as f64
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PerchRideState {
    Seeking,
    Riding,
}

/// Foreign-window perch-and-ride behavior (M8): run to a user-dragged window's title-bar
/// anchor, then ride it until the drag ends. Window discovery and geometry stay in the
/// platform backend; the engine only receives opaque IDs and world-space anchors.
pub struct PerchRideTask {
    window: Option<ForeignWindowSnapshot>,
    state: PerchRideState,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CollectState {
    Choose,
    WaitForSpawn {
        request: CollectWindowRequestId,
        payload: CollectWindowPayload,
        deadline: f64,
    },
    RunToPickup {
        request: CollectWindowRequestId,
        payload: CollectWindowPayload,
    },
    DraggingBack {
        request: CollectWindowRequestId,
        payload: CollectWindowPayload,
        window_offset_to_beak: Vec2,
        release_at: Vec2,
    },
    Release {
        request: CollectWindowRequestId,
        payload: CollectWindowPayload,
        typed: bool,
        visible_until: f64,
    },
}

/// Autonomous collect-window dispatcher (M9): drag in a Note or Meme prop using only
/// platform-neutral commands and snapshots.
pub struct CollectWindowTask {
    state: CollectState,
    forced: Option<CollectWindowKind>,
}

impl Default for CollectWindowTask {
    fn default() -> Self {
        Self::new()
    }
}

impl CollectWindowTask {
    pub fn new() -> Self {
        Self {
            state: CollectState::Choose,
            forced: None,
        }
    }

    pub fn forced(kind: CollectWindowKind) -> Self {
        Self {
            state: CollectState::Choose,
            forced: Some(kind),
        }
    }

    fn choose_payload(&mut self, ctx: &mut TaskCtx) -> Option<CollectWindowPayload> {
        let mut kinds = [None, None];
        let mut len = 0usize;
        for kind in [CollectWindowKind::Note, CollectWindowKind::Meme] {
            if self.forced.is_some_and(|forced| forced != kind) {
                continue;
            }
            if ctx.collect_window.kind_active(kind) {
                kinds[len] = Some(kind);
                len += 1;
            }
        }
        if len == 0 {
            return None;
        }
        let kind = kinds[(ctx.rng.range(0.0, len as f32) as usize).min(len - 1)].unwrap();
        match kind {
            CollectWindowKind::Note => Some(CollectWindowPayload::Note {
                index: (ctx
                    .rng
                    .range(0.0, ctx.collect_window.available_notes as f32)
                    as u32)
                    .min(ctx.collect_window.available_notes.saturating_sub(1)),
            }),
            CollectWindowKind::Meme => Some(CollectWindowPayload::Meme {
                index: (ctx
                    .rng
                    .range(0.0, ctx.collect_window.available_memes as f32)
                    as u32)
                    .min(ctx.collect_window.available_memes.saturating_sub(1)),
            }),
        }
    }

    fn request_id(ctx: &TaskCtx, payload: CollectWindowPayload) -> CollectWindowRequestId {
        let kind_bit = match payload.kind() {
            CollectWindowKind::Note => 0x10_0000,
            CollectWindowKind::Meme => 0x20_0000,
        };
        CollectWindowRequestId(((ctx.now * 120.0).round() as u64) ^ kind_bit)
    }

    fn live_snapshot(
        ctx: &TaskCtx,
        request: CollectWindowRequestId,
        payload: CollectWindowPayload,
    ) -> Option<CollectWindowSnapshot> {
        let snapshot = ctx.collect_window_snapshot?;
        if snapshot.request == request && snapshot.kind == payload.kind() && snapshot.alive {
            Some(snapshot)
        } else {
            None
        }
    }

    fn active_request_payload(&self) -> Option<(CollectWindowRequestId, CollectWindowPayload)> {
        match self.state {
            CollectState::Choose => None,
            CollectState::WaitForSpawn {
                request, payload, ..
            }
            | CollectState::RunToPickup { request, payload }
            | CollectState::DraggingBack {
                request, payload, ..
            }
            | CollectState::Release {
                request, payload, ..
            } => Some((request, payload)),
        }
    }
}

impl Task for CollectWindowTask {
    fn anticipating(&self) -> bool {
        matches!(
            self.state,
            CollectState::WaitForSpawn { .. } | CollectState::RunToPickup { .. }
        )
    }
    fn id(&self) -> &'static str {
        "collect_window"
    }

    fn collect_kind(&self) -> Option<CollectWindowKind> {
        self.forced.or(match self.state {
            CollectState::Choose => None,
            CollectState::WaitForSpawn { payload, .. }
            | CollectState::RunToPickup { payload, .. }
            | CollectState::DraggingBack { payload, .. }
            | CollectState::Release { payload, .. } => Some(payload.kind()),
        })
    }

    fn collect_request(&self) -> Option<(CollectWindowRequestId, CollectWindowKind)> {
        self.active_request_payload()
            .map(|(request, payload)| (request, payload.kind()))
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        if let Some((request, payload)) = self.active_request_payload() {
            if ctx.collect_window_snapshot.is_some_and(|snapshot| {
                snapshot.request == request && snapshot.kind == payload.kind() && !snapshot.alive
            }) {
                return true;
            }
        }
        if !ctx.collect_window.active()
            || self
                .collect_kind()
                .is_some_and(|kind| !ctx.collect_window.kind_active(kind))
        {
            return true;
        }

        goose.current_speed = goose.parameters.run_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;

        match self.state {
            CollectState::Choose => {
                if !ctx.collect_window.capacity_available {
                    return true;
                }
                let Some(payload) = self.choose_payload(ctx) else {
                    return true;
                };
                let request = Self::request_id(ctx, payload);
                ctx.collect_window_commands
                    .push(CollectWindowCommand::Spawn { request, payload });
                self.state = CollectState::WaitForSpawn {
                    request,
                    payload,
                    deadline: ctx.now + COLLECT_SPAWN_TIMEOUT,
                };
                false
            }
            CollectState::WaitForSpawn {
                request,
                payload,
                deadline,
            } => {
                if let Some(snapshot) = Self::live_snapshot(ctx, request, payload) {
                    if ctx.collect_window.capabilities.move_window {
                        goose.target_pos = ctx
                            .layout
                            .clamp_point(beak_locomotion_target(goose, snapshot.center()));
                        self.state = CollectState::RunToPickup { request, payload };
                    } else {
                        // The compositor owns placement. Do not invent a global
                        // position or send the goose toward a local window rect.
                        goose.target_pos = goose.position;
                        self.state = CollectState::Release {
                            request,
                            payload,
                            typed: false,
                            visible_until: ctx.now + COLLECT_VISIBLE_DWELL,
                        };
                    }
                    false
                } else {
                    ctx.now >= deadline
                }
            }
            CollectState::RunToPickup { request, payload } => {
                let Some(snapshot) = Self::live_snapshot(ctx, request, payload) else {
                    return true;
                };
                let pickup = snapshot.center();
                goose.target_pos = ctx
                    .layout
                    .clamp_point(beak_locomotion_target(goose, pickup));
                if Vec2::distance(goose.rig.beak_tip, pickup) <= COLLECT_PICKUP_DISTANCE {
                    let offset = snapshot.rect.min - goose.rig.beak_tip;
                    let release_at = ctx
                        .layout
                        .clamp_point((ctx.bounds.min + ctx.bounds.max) * 0.5);
                    ctx.collect_window_commands
                        .push(CollectWindowCommand::SetPassthrough {
                            id: snapshot.id,
                            passthrough: true,
                        });
                    self.state = CollectState::DraggingBack {
                        request,
                        payload,
                        window_offset_to_beak: offset,
                        release_at,
                    };
                }
                false
            }
            CollectState::DraggingBack {
                request,
                payload,
                window_offset_to_beak,
                release_at,
            } => {
                let Some(snapshot) = Self::live_snapshot(ctx, request, payload) else {
                    return true;
                };
                goose.target_pos = ctx.layout.clamp_point(release_at);
                ctx.collect_window_commands
                    .push(CollectWindowCommand::Move {
                        id: snapshot.id,
                        top_left: goose.rig.beak_tip + window_offset_to_beak,
                    });
                if Vec2::distance(goose.position, release_at) <= COLLECT_RELEASE_DISTANCE {
                    self.state = CollectState::Release {
                        request,
                        payload,
                        typed: false,
                        visible_until: ctx.now + COLLECT_VISIBLE_DWELL,
                    };
                }
                false
            }
            CollectState::Release {
                request,
                payload,
                typed,
                visible_until,
            } => {
                let Some(snapshot) = Self::live_snapshot(ctx, request, payload) else {
                    return true;
                };
                if !typed {
                    if ctx.collect_window.capabilities.set_passthrough {
                        ctx.collect_window_commands
                            .push(CollectWindowCommand::SetPassthrough {
                                id: snapshot.id,
                                passthrough: false,
                            });
                    }
                    if let CollectWindowPayload::Note { index } = payload {
                        ctx.collect_window_commands
                            .push(CollectWindowCommand::Focus { id: snapshot.id });
                        ctx.collect_window_commands
                            .push(CollectWindowCommand::TypeNote {
                                id: snapshot.id,
                                note_index: index,
                            });
                    }
                    self.state = CollectState::Release {
                        request,
                        payload,
                        typed: true,
                        visible_until,
                    };
                } else if ctx.now >= visible_until {
                    if payload.kind() == CollectWindowKind::Meme {
                        ctx.collect_window_commands
                            .push(CollectWindowCommand::Close { id: snapshot.id });
                    }
                    return true;
                }
                false
            }
        }
    }
}

impl Default for PerchRideTask {
    fn default() -> Self {
        Self::new()
    }
}

impl PerchRideTask {
    pub fn new() -> Self {
        Self {
            window: None,
            state: PerchRideState::Seeking,
        }
    }
}

impl Task for PerchRideTask {
    fn anticipating(&self) -> bool {
        self.state == PerchRideState::Seeking
    }
    fn id(&self) -> &'static str {
        "perch_ride"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        if !ctx.foreign_window.watch_active() {
            return true;
        }

        let Some(snapshot) = ctx.dragged_window else {
            return true;
        };

        if let Some(current) = self.window {
            if current.id != snapshot.id {
                return true;
            }
        } else {
            self.window = Some(snapshot);
        }

        match self.state {
            PerchRideState::Seeking => {
                goose.current_speed = goose.parameters.run_speed;
                goose.current_acceleration = goose.parameters.acceleration_normal;
                goose.target_pos = snapshot.ride_anchor;

                if Vec2::distance(goose.position, snapshot.ride_anchor) <= 6.0 {
                    self.state = PerchRideState::Riding;
                    goose.position = snapshot.ride_anchor;
                    goose.target_pos = snapshot.ride_anchor;
                    goose.velocity = Vec2::ZERO;
                }
                false
            }
            PerchRideState::Riding => {
                goose.position = snapshot.ride_anchor;
                goose.target_pos = snapshot.ride_anchor;
                goose.velocity = Vec2::ZERO;
                goose.current_speed = 0.0;
                goose.current_acceleration = 0.0;
                false
            }
        }
    }
}

/// A safe holding task used while a platform runtime waits for a required permission.
/// The goose walks to a runtime-selected anchor, settles there, and never yields to roaming.
pub struct PermissionWaitTask {
    anchor: Vec2,
}

impl PermissionWaitTask {
    pub fn new(anchor: Vec2) -> Self {
        Self { anchor }
    }
}

impl Task for PermissionWaitTask {
    fn id(&self) -> &'static str {
        "permission_wait"
    }

    fn set_permission_wait_anchor(&mut self, anchor: Vec2) {
        self.anchor = anchor;
    }

    fn run(&mut self, goose: &mut GooseEntity, _ctx: &mut TaskCtx) -> bool {
        goose.current_speed = goose.parameters.walk_speed;
        goose.current_acceleration = goose.parameters.acceleration_normal;
        goose.target_pos = self.anchor;
        if arrived(goose, 2.0) {
            goose.current_speed = 0.0;
            goose.velocity = Vec2::ZERO;
        }
        false
    }
}

/// The scripted first-run intro: the goose walks in to centre stage, pauses to "introduce
/// itself" for [`FIRST_WANDER_TIME`], then yields to roaming. (`FirstUX_FirstTask` /
/// `FirstUX_SecondTask` in the original; text/honk flourishes arrive with M5 audio + notes.)
#[derive(Default)]
pub struct FirstUxTask {
    intro_until: Option<f64>,
    entry_target: Option<Vec2>,
}

impl FirstUxTask {
    pub fn new() -> Self {
        Self::default()
    }

    /// FirstUX variant used at process startup, preserving the on-screen destination paired
    /// with the off-screen spawn selected by the world.
    pub fn entering(entry_target: Vec2) -> Self {
        Self {
            intro_until: None,
            entry_target: Some(entry_target),
        }
    }
}

impl Task for FirstUxTask {
    fn id(&self) -> &'static str {
        "first_ux"
    }

    fn run(&mut self, goose: &mut GooseEntity, ctx: &mut TaskCtx) -> bool {
        match self.intro_until {
            None => {
                // Walk in to centre stage.
                goose.current_speed = goose.parameters.walk_speed;
                goose.current_acceleration = goose.parameters.acceleration_normal;
                goose.target_pos = self.entry_target.unwrap_or_else(|| {
                    ctx.layout
                        .clamp_point((ctx.bounds.min + ctx.bounds.max) * 0.5)
                });
                if arrived(goose, 2.0) {
                    self.intro_until = Some(ctx.now + ctx.timing.first_wander_time as f64);
                }
                false
            }
            // Pause centre stage for the intro beat, then hand off to roaming.
            Some(until) => ctx.now >= until,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_ctx<'a>(
        now: f64,
        rng: &'a mut SplitMix64,
        sounds: &'a mut Vec<Sound>,
        cursor_commands: &'a mut Vec<CursorCommand>,
    ) -> TaskCtx<'a> {
        let collect_window_commands = Box::leak(Box::new(Vec::new()));
        let layout = Box::leak(Box::new(DesktopLayout::single(ctx_bounds())));
        TaskCtx {
            now,
            dt: 1.0 / 120.0,
            bounds: ctx_bounds(),
            layout,
            goose_visible: true,
            rng,
            sounds,
            cursor_commands,
            collect_window_commands,
            pointer: Pointer::default(),
            mouse_steal: MouseStealOptions::default(),
            foreign_window: ForeignWindowOptions::default(),
            collect_window: CollectWindowOptions::default(),
            dragged_window: None,
            collect_window_snapshot: None,
            calm: false,
            timing: TimingOptions::default(),
            autumn_piles: &[],
            toys: Box::leak(Box::new(ToyState::new())),
        }
    }

    fn ctx_bounds() -> Rect {
        Rect {
            min: Vec2::new(0.0, 0.0),
            max: Vec2::new(1000.0, 800.0),
        }
    }

    fn dragged_window(anchor: Vec2) -> ForeignWindowSnapshot {
        ForeignWindowSnapshot::top_center(
            crate::foreign_window::ForeignWindowId(42),
            Rect {
                min: Vec2::new(anchor.x - 100.0, anchor.y),
                max: Vec2::new(anchor.x + 100.0, anchor.y + 120.0),
            },
        )
    }

    fn collect_options(notes: u32, memes: u32) -> CollectWindowOptions {
        CollectWindowOptions::with_backend_support(
            crate::collect_window::CollectWindowCapabilities {
                spawn_note: true,
                spawn_image: true,
                move_window: true,
                set_passthrough: true,
                synthesize_text: true,
            },
            notes,
            memes,
        )
    }

    fn collect_snapshot(
        request: CollectWindowRequestId,
        kind: CollectWindowKind,
        rect: Rect,
    ) -> CollectWindowSnapshot {
        CollectWindowSnapshot {
            id: crate::collect_window::CollectWindowId(99),
            request,
            kind,
            rect,
            alive: true,
            close_origin: None,
        }
    }

    fn collect_close_snapshot(
        request: CollectWindowRequestId,
        kind: CollectWindowKind,
        origin: CollectWindowCloseOrigin,
    ) -> CollectWindowSnapshot {
        CollectWindowSnapshot {
            id: crate::collect_window::CollectWindowId(99),
            request,
            kind,
            rect: Rect::new(Vec2::ZERO, Vec2::ZERO),
            alive: false,
            close_origin: Some(origin),
        }
    }

    #[test]
    fn wander_picks_in_bounds_targets_and_finishes() {
        let mut rng = SplitMix64::seed(1);
        let mut sounds: Vec<Sound> = Vec::new();
        let mut cursor_commands: Vec<CursorCommand> = Vec::new();
        let b = ctx_bounds();
        let mut goose = GooseEntity::new();
        let mut task = WanderTask::new();
        // First run sets a target inside bounds and arms the dwell timer.
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        assert!(!task.run(&mut goose, &mut ctx));
        assert!(goose.target_pos.x >= b.min.x && goose.target_pos.x <= b.max.x);
        assert!(goose.target_pos.y >= b.min.y && goose.target_pos.y <= b.max.y);
        assert_eq!(goose.current_speed, goose.parameters.walk_speed);
        // Well past the max dwell it reports finished.
        let mut ctx = base_ctx(
            (MAX_WANDERING_TIME + 1.0) as f64,
            &mut rng,
            &mut sounds,
            &mut cursor_commands,
        );
        assert!(task.run(&mut goose, &mut ctx));
    }

    /// Run `WanderTask` through `iters` forced arrivals and count the spontaneous honks.
    fn wander_arrival_honks(calm: bool, seed: u64, iters: usize) -> usize {
        let mut rng = SplitMix64::seed(seed);
        let mut sounds: Vec<Sound> = Vec::new();
        let mut cursor_commands: Vec<CursorCommand> = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = WanderTask::new();
        let mut now = 0.0;
        for _ in 0..iters {
            // Snap onto the current target so the next run sees an arrival.
            goose.position = goose.target_pos;
            let mut ctx = base_ctx(now, &mut rng, &mut sounds, &mut cursor_commands);
            ctx.calm = calm;
            task.run(&mut goose, &mut ctx);
            now += 0.1;
        }
        sounds.iter().filter(|s| **s == Sound::honk()).count()
    }

    #[test]
    fn calm_suppresses_spontaneous_honks() {
        let seed = 12_345;
        let noisy = wander_arrival_honks(false, seed, 80);
        assert!(noisy > 0, "control: an un-calm goose honks on arrivals");
        let calm = wander_arrival_honks(true, seed, 80);
        assert_eq!(
            calm, 0,
            "a calm (post-pat) goose suppresses spontaneous honks"
        );
    }

    #[test]
    fn wander_reasserts_speed_each_run() {
        let mut rng = SplitMix64::seed(99);
        let mut sounds: Vec<Sound> = Vec::new();
        let mut cursor_commands: Vec<CursorCommand> = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = WanderTask::new();
        // First run arms the task and sets walk speed.
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        task.run(&mut goose, &mut ctx);
        // Simulate a hyper burst having left charge-tier speed on the goose.
        goose.current_speed = 999.0;
        goose.current_acceleration = 999.0;
        let mut ctx = base_ctx(1.0, &mut rng, &mut sounds, &mut cursor_commands);
        task.run(&mut goose, &mut ctx);
        assert_eq!(
            goose.current_speed, goose.parameters.walk_speed,
            "wander should restore walk speed after a hyper burst"
        );
        assert_eq!(
            goose.current_acceleration,
            goose.parameters.acceleration_normal
        );
    }

    #[test]
    fn hyper_sets_charge_tier_and_finishes() {
        let mut rng = SplitMix64::seed(4);
        let mut sounds: Vec<Sound> = Vec::new();
        let mut cursor_commands: Vec<CursorCommand> = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = HyperTask::new();
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        assert!(!task.run(&mut goose, &mut ctx), "still hyper at t=0");
        assert_eq!(goose.current_speed, goose.parameters.charge_speed);
        assert_eq!(
            goose.current_acceleration,
            goose.parameters.acceleration_charged
        );
        // Well past the burst it reports finished.
        let mut ctx = base_ctx(
            HYPER_DURATION + 0.1,
            &mut rng,
            &mut sounds,
            &mut cursor_commands,
        );
        assert!(
            task.run(&mut goose, &mut ctx),
            "hyper ends after its duration"
        );
    }

    #[test]
    fn hyper_honks_excitedly_on_enter() {
        let mut rng = SplitMix64::seed(8);
        let mut sounds: Vec<Sound> = Vec::new();
        let mut cursor_commands: Vec<CursorCommand> = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = HyperTask::new();
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        task.run(&mut goose, &mut ctx);
        assert!(
            sounds.contains(&Sound::high_honk()),
            "clicking the goose makes it honk"
        );
    }

    #[test]
    fn mouse_steal_default_drag_time_matches_hyper_burst() {
        assert_eq!(
            MouseStealOptions::default().succ_time as f64,
            HYPER_DURATION
        );
    }

    #[test]
    fn first_ux_walks_in_then_finishes_after_intro() {
        let mut rng = SplitMix64::seed(2);
        let mut sounds: Vec<Sound> = Vec::new();
        let mut cursor_commands: Vec<CursorCommand> = Vec::new();
        let b = ctx_bounds();
        let center = (b.min + b.max) * 0.5;
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(center.x, b.max.y + 60.0); // start off-stage
        let mut task = FirstUxTask::new();

        // Before arriving at centre, never finished.
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.target_pos, center);

        // Snap to centre → the intro pause arms; still not finished until it elapses.
        goose.position = center;
        let mut ctx = base_ctx(1.0, &mut rng, &mut sounds, &mut cursor_commands);
        assert!(!task.run(&mut goose, &mut ctx));
        let mut ctx = base_ctx(
            (1.0 + FIRST_WANDER_TIME + 0.1) as f64,
            &mut rng,
            &mut sounds,
            &mut cursor_commands,
        );
        assert!(task.run(&mut goose, &mut ctx));
    }

    #[test]
    fn edge_wrap_teleports_only_after_hidden_and_stages_the_opposite_entry_offscreen() {
        let mut rng = SplitMix64::seed(21);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(500.0, 400.0);
        let mut task = EdgeWrapTask::new();

        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        assert!(!task.run(&mut goose, &mut ctx));
        let exit_target = goose.target_pos;
        assert!(!ctx.layout.contains(exit_target));

        goose.position = exit_target;
        ctx.goose_visible = true;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(
            goose.position, exit_target,
            "a still-visible goose never wraps"
        );

        ctx.goose_visible = false;
        assert!(!task.run(&mut goose, &mut ctx));
        assert!(!ctx.layout.contains(goose.position));
        assert!(ctx.layout.contains(goose.target_pos));
        let passage = task.passage.expect("chosen passage");
        assert_eq!(
            passage.exit_edge.direction().opposite(),
            ctx.layout.nearest_exposed_edge(goose.position).direction(),
            "the hidden entry is staged at an opposite-facing edge"
        );
    }

    #[test]
    fn deliberate_excursion_waits_until_hidden_and_returns_from_its_own_entry_without_wrap() {
        let mut rng = SplitMix64::seed(22);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(500.0, 400.0);
        let exit = Vec2::new(-220.0, 400.0);
        let entry = Vec2::new(500.0, -220.0);
        let return_target = Vec2::new(500.0, 220.0);
        let mut task = ExcursionTask::new(ExcursionKind::Errand, exit, entry, return_target, 1.0);

        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        goose.position = exit;
        ctx.goose_visible = true;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(task.state, ExcursionState::Depart);

        ctx.goose_visible = false;
        assert!(!task.run(&mut goose, &mut ctx));
        assert!(matches!(task.state, ExcursionState::Away { .. }));
        ctx.now = 1.1;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(
            goose.position, entry,
            "an errand returns from its staged edge"
        );
        assert_eq!(goose.target_pos, return_target);
    }

    #[test]
    fn graceful_exit_never_completes_as_a_roaming_task_or_reenters() {
        let mut rng = SplitMix64::seed(23);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(500.0, 400.0);
        let mut task = GracefulExitTask::new();
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);

        assert!(!task.run(&mut goose, &mut ctx));
        goose.position = goose.target_pos;
        ctx.goose_visible = false;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.velocity, Vec2::ZERO);
        assert_eq!(goose.target_pos, goose.position);
    }

    #[test]
    fn collect_task_ends_only_for_a_matching_dead_request() {
        for origin in [
            CollectWindowCloseOrigin::User,
            CollectWindowCloseOrigin::Program,
        ] {
            let mut rng = SplitMix64::seed(24);
            let mut sounds = Vec::new();
            let mut cursor_commands = Vec::new();
            let mut goose = GooseEntity::new();
            let mut task = CollectWindowTask::forced(CollectWindowKind::Note);
            let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
            ctx.collect_window = collect_options(1, 0);
            assert!(!task.run(&mut goose, &mut ctx));
            let request = match ctx.collect_window_commands.as_slice() {
                [CollectWindowCommand::Spawn { request, .. }] => *request,
                other => panic!("unexpected commands: {other:?}"),
            };
            ctx.collect_window_snapshot = Some(collect_close_snapshot(
                request,
                CollectWindowKind::Note,
                origin,
            ));
            assert!(task.run(&mut goose, &mut ctx));
        }
    }

    #[test]
    fn annoyed_reaction_is_visible_and_audible_only_when_allowed() {
        let mut rng = SplitMix64::seed(25);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(500.0, 400.0);
        let starting_direction = goose.direction;
        let mut task = AnnoyedReactionTask::new(true);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.position, Vec2::new(500.0, 400.0));
        assert_ne!(goose.direction, starting_direction);
        assert!(goose.extending_neck);
        assert_eq!(&*ctx.sounds, &[Sound::high_honk()]);

        let mut quiet = AnnoyedReactionTask::new(false);
        ctx.sounds.clear();
        assert!(!quiet.run(&mut goose, &mut ctx));
        assert!(ctx.sounds.is_empty());
    }

    #[test]
    fn nab_finishes_without_cursor_capability() {
        let mut rng = SplitMix64::seed(10);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = NabMouseTask::new();
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.pointer = Pointer {
            pos: goose.rig.beak_tip,
            present: true,
            left_down: false,
        };
        ctx.mouse_steal.enabled = true;
        ctx.mouse_steal.warp_supported = false;

        assert!(task.run(&mut goose, &mut ctx));
        assert!(ctx.cursor_commands.is_empty());
        assert!(ctx.sounds.is_empty());
    }

    #[test]
    fn nab_seeks_live_pointer_at_charge_speed() {
        let mut rng = SplitMix64::seed(11);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = NabMouseTask::new();
        let pointer = Vec2::new(700.0, 500.0);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.pointer = Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        };
        ctx.mouse_steal = MouseStealOptions::with_backend_support(true);

        assert!(!task.run(&mut goose, &mut ctx));
        assert!(
            Vec2::distance(
                goose.target_pos + (goose.rig.beak_tip - goose.position),
                pointer
            ) < 0.01
        );
        assert_eq!(goose.current_speed, goose.parameters.charge_speed);
        assert_eq!(
            goose.current_acceleration,
            goose.parameters.acceleration_charged
        );
        assert!(ctx.cursor_commands.is_empty(), "not grabbed yet");
    }

    #[test]
    fn nab_grabs_with_one_bite_and_cursor_warp() {
        let mut rng = SplitMix64::seed(12);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = NabMouseTask::new();
        let pointer = goose.rig.beak_tip + Vec2::new(3.0, 0.0);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.pointer = Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        };
        ctx.mouse_steal = MouseStealOptions::with_backend_support(true);

        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(&*ctx.sounds, &[Sound::Bite]);
        assert_eq!(&*ctx.cursor_commands, &[CursorCommand::WarpTo(pointer)]);

        ctx.sounds.clear();
        ctx.cursor_commands.clear();
        ctx.now = 0.5;
        ctx.pointer.pos = pointer;
        assert!(!task.run(&mut goose, &mut ctx));
        assert!(
            ctx.sounds.is_empty(),
            "the bite sound should play only when the cursor is first grabbed"
        );
        assert_eq!(ctx.cursor_commands.len(), 1);
    }

    #[test]
    fn nab_drag_preserves_beak_cursor_offset_and_times_out() {
        let mut rng = SplitMix64::seed(13);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = NabMouseTask::new();
        let offset = Vec2::new(9.0, -4.0);
        let pointer = goose.rig.beak_tip + offset;
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.pointer = Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        };
        ctx.mouse_steal = MouseStealOptions::with_backend_support(true);

        task.run(&mut goose, &mut ctx);
        ctx.cursor_commands.clear();
        ctx.sounds.clear();

        goose.rig.beak_tip = goose.rig.beak_tip + Vec2::new(25.0, 10.0);
        let expected = goose.rig.beak_tip + offset;
        ctx.pointer.pos = expected;
        ctx.now = 0.25;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(&*ctx.cursor_commands, &[CursorCommand::WarpTo(expected)]);

        ctx.cursor_commands.clear();
        ctx.pointer.pos = expected;
        ctx.now = ctx.mouse_steal.succ_time as f64 + 0.01;
        assert!(task.run(&mut goose, &mut ctx), "nab ends after succ_time");
    }

    #[test]
    fn nab_drag_retargets_like_hyper_when_it_arrives() {
        let mut rng = SplitMix64::seed(15);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = NabMouseTask::new();
        let offset = Vec2::new(6.0, 2.0);
        let pointer = goose.rig.beak_tip + offset;
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.pointer = Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        };
        ctx.mouse_steal = MouseStealOptions::with_backend_support(true);

        assert!(!task.run(&mut goose, &mut ctx));
        let first_target = match task.state {
            NabState::DraggingMouseAway { target, .. } => target,
            NabState::SeekingMouse => panic!("nab should be dragging after the grab"),
        };

        ctx.cursor_commands.clear();
        ctx.sounds.clear();
        goose.position = first_target;
        goose.target_pos = first_target;
        ctx.pointer.pos = goose.rig.beak_tip + offset;
        ctx.now = 0.25;

        assert!(!task.run(&mut goose, &mut ctx));
        let second_target = match task.state {
            NabState::DraggingMouseAway { target, .. } => target,
            NabState::SeekingMouse => panic!("nab should still be dragging after retarget"),
        };
        assert_ne!(
            second_target, first_target,
            "dragging should retarget like hyper instead of pulling in one straight line"
        );
        assert_eq!(goose.target_pos, second_target);
        assert_eq!(
            &*ctx.cursor_commands,
            &[CursorCommand::WarpTo(goose.rig.beak_tip + offset)]
        );
    }

    #[test]
    fn nab_drops_when_cursor_is_pulled_far_away() {
        let mut rng = SplitMix64::seed(14);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = NabMouseTask::new();
        let pointer = goose.rig.beak_tip;
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.pointer = Pointer {
            pos: pointer,
            present: true,
            left_down: false,
        };
        ctx.mouse_steal = MouseStealOptions::with_backend_support(true);

        task.run(&mut goose, &mut ctx);
        ctx.cursor_commands.clear();
        ctx.pointer.pos = pointer + Vec2::new(ctx.mouse_steal.drop_distance + 20.0, 0.0);
        ctx.now = 0.25;

        assert!(
            task.run(&mut goose, &mut ctx),
            "manual pull-away drops the cursor"
        );
        assert!(ctx.cursor_commands.is_empty());
    }

    #[test]
    fn perch_ride_finishes_without_window_watch_capability() {
        let mut rng = SplitMix64::seed(16);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = PerchRideTask::new();
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.dragged_window = Some(dragged_window(Vec2::new(500.0, 200.0)));

        assert!(task.run(&mut goose, &mut ctx));
    }

    #[test]
    fn perch_ride_seeks_window_anchor_at_run_speed() {
        let mut rng = SplitMix64::seed(17);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = PerchRideTask::new();
        let anchor = Vec2::new(800.0, 120.0);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.foreign_window = ForeignWindowOptions::with_backend_support(true, false);
        ctx.dragged_window = Some(dragged_window(anchor));

        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.target_pos, anchor);
        assert_eq!(goose.current_speed, goose.parameters.run_speed);
        assert_eq!(
            goose.current_acceleration,
            goose.parameters.acceleration_normal
        );
    }

    #[test]
    fn perch_ride_abandons_when_drag_releases_before_arrival() {
        let mut rng = SplitMix64::seed(18);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = PerchRideTask::new();
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.foreign_window = ForeignWindowOptions::with_backend_support(true, false);
        ctx.dragged_window = Some(dragged_window(Vec2::new(900.0, 120.0)));
        assert!(!task.run(&mut goose, &mut ctx));

        ctx.dragged_window = None;
        ctx.now = 0.25;
        assert!(task.run(&mut goose, &mut ctx));
    }

    #[test]
    fn perch_ride_pins_to_moving_anchor_after_arrival() {
        let mut rng = SplitMix64::seed(19);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = PerchRideTask::new();
        let first_anchor = Vec2::new(-400.0, -20.0);
        goose.position = first_anchor + Vec2::new(2.0, 2.0);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.foreign_window = ForeignWindowOptions::with_backend_support(true, false);
        ctx.dragged_window = Some(dragged_window(first_anchor));

        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.position, first_anchor);
        assert_eq!(goose.velocity, Vec2::ZERO);

        let moved_anchor = Vec2::new(-360.0, -16.0);
        ctx.dragged_window = Some(dragged_window(moved_anchor));
        ctx.now = 0.25;
        goose.velocity = Vec2::new(40.0, 5.0);
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.position, moved_anchor);
        assert_eq!(goose.target_pos, moved_anchor);
        assert_eq!(goose.velocity, Vec2::ZERO);
    }

    #[test]
    fn collect_window_finishes_without_capable_content() {
        let mut rng = SplitMix64::seed(20);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = CollectWindowTask::forced(CollectWindowKind::Note);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);

        assert!(task.run(&mut goose, &mut ctx));
        assert!(ctx.collect_window_commands.is_empty());
    }

    #[test]
    fn collect_window_spawns_forced_note_payload() {
        let mut rng = SplitMix64::seed(21);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = CollectWindowTask::forced(CollectWindowKind::Note);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.collect_window = collect_options(2, 0);

        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(ctx.collect_window_commands.len(), 1);
        match ctx.collect_window_commands[0] {
            CollectWindowCommand::Spawn {
                payload: CollectWindowPayload::Note { index },
                ..
            } => assert!(index < 2),
            other => panic!("unexpected collect command: {other:?}"),
        }
    }

    #[test]
    fn collect_window_drags_note_then_focuses_and_types_in_order() {
        let mut rng = SplitMix64::seed(22);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = CollectWindowTask::forced(CollectWindowKind::Note);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.collect_window = collect_options(1, 0);

        assert!(!task.run(&mut goose, &mut ctx));
        let (request, payload) = match ctx.collect_window_commands[0] {
            CollectWindowCommand::Spawn { request, payload } => (request, payload),
            other => panic!("unexpected collect command: {other:?}"),
        };
        ctx.collect_window_commands.clear();
        // The final admitted note fills the backend. It must still be delivered
        // and receive its text, even though no further prop can be started.
        ctx.collect_window.capacity_available = false;

        let rect = Rect {
            min: Vec2::new(200.0, 100.0),
            max: Vec2::new(500.0, 300.0),
        };
        ctx.collect_window_snapshot =
            Some(collect_snapshot(request, CollectWindowKind::Note, rect));
        ctx.now = 0.1;
        let pickup = Vec2::new(350.0, 200.0);
        let expected_body_target = ctx
            .layout
            .clamp_point(beak_locomotion_target(&goose, pickup));
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(goose.target_pos, expected_body_target);
        assert_eq!(
            goose.target_pos + (goose.rig.beak_tip - goose.position),
            pickup,
            "collect locomotion must aim the beak, not the body, at the prop"
        );

        ctx.collect_window_commands.clear();
        goose.rig.beak_tip = Vec2::new(350.0, 200.0);
        ctx.now = 0.2;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(
            ctx.collect_window_commands.as_slice(),
            &[CollectWindowCommand::SetPassthrough {
                id: crate::collect_window::CollectWindowId(99),
                passthrough: true
            }]
        );

        ctx.collect_window_commands.clear();
        goose.position = (ctx.bounds.min + ctx.bounds.max) * 0.5;
        goose.rig.beak_tip = goose.position;
        ctx.now = 0.3;
        assert!(!task.run(&mut goose, &mut ctx));
        assert!(matches!(
            ctx.collect_window_commands.as_slice(),
            [CollectWindowCommand::Move { .. }]
        ));

        ctx.collect_window_commands.clear();
        ctx.now = 0.4;
        assert!(!task.run(&mut goose, &mut ctx));
        assert_eq!(
            ctx.collect_window_commands.as_slice(),
            &[
                CollectWindowCommand::SetPassthrough {
                    id: crate::collect_window::CollectWindowId(99),
                    passthrough: false
                },
                CollectWindowCommand::Focus {
                    id: crate::collect_window::CollectWindowId(99)
                },
                CollectWindowCommand::TypeNote {
                    id: crate::collect_window::CollectWindowId(99),
                    note_index: match payload {
                        CollectWindowPayload::Note { index } => index,
                        CollectWindowPayload::Meme { .. } => panic!("expected note"),
                    }
                },
            ],
            "release must restore clickability, focus, then type"
        );
    }

    #[test]
    fn collect_window_placement_only_delivers_without_invented_desktop_movement() {
        for kind in [CollectWindowKind::Note, CollectWindowKind::Meme] {
            let mut rng = SplitMix64::seed(64);
            let mut sounds = Vec::new();
            let mut cursors = Vec::new();
            let mut goose = GooseEntity::new();
            goose.position = Vec2::new(480.0, 320.0);
            let mut task = CollectWindowTask::forced(kind);
            let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursors);
            ctx.collect_window = collect_options(1, 1);
            ctx.collect_window.capabilities.move_window = false;
            ctx.collect_window.capabilities.set_passthrough = false;
            assert!(!task.run(&mut goose, &mut ctx));
            let request = match ctx.collect_window_commands[0] {
                CollectWindowCommand::Spawn { request, .. } => request,
                other => panic!("expected spawn, got {other:?}"),
            };
            ctx.collect_window_commands.clear();
            ctx.collect_window.capacity_available = false;
            // Wayland can supply local dimensions, but no global position.
            // This deliberately distant rectangle must never become a target.
            ctx.collect_window_snapshot = Some(collect_snapshot(
                request,
                kind,
                Rect::new(Vec2::new(-9000.0, -8000.0), Vec2::new(-8500.0, -7600.0)),
            ));
            assert!(!task.run(&mut goose, &mut ctx));
            assert_eq!(goose.target_pos, goose.position);
            assert!(!task.run(&mut goose, &mut ctx));
            if kind == CollectWindowKind::Note {
                assert!(matches!(
                    ctx.collect_window_commands.as_slice(),
                    [
                        CollectWindowCommand::Focus { .. },
                        CollectWindowCommand::TypeNote { .. }
                    ]
                ));
            } else {
                assert!(ctx.collect_window_commands.is_empty());
            }
            ctx.collect_window_commands.clear();
            ctx.now = COLLECT_VISIBLE_DWELL + 0.1;
            assert!(task.run(&mut goose, &mut ctx));
            assert!(ctx
                .collect_window_commands
                .iter()
                .all(|command| matches!(command, CollectWindowCommand::Close { .. })));
            assert_eq!(
                ctx.collect_window_commands.len(),
                usize::from(kind == CollectWindowKind::Meme)
            );
            assert!(ctx.cursor_commands.is_empty());
        }
    }

    #[test]
    fn collect_window_closes_meme_after_visible_dwell() {
        let mut rng = SplitMix64::seed(23);
        let mut sounds = Vec::new();
        let mut cursor_commands = Vec::new();
        let mut goose = GooseEntity::new();
        let mut task = CollectWindowTask::forced(CollectWindowKind::Meme);
        let mut ctx = base_ctx(0.0, &mut rng, &mut sounds, &mut cursor_commands);
        ctx.collect_window = collect_options(0, 1);

        assert!(!task.run(&mut goose, &mut ctx));
        let request = match ctx.collect_window_commands[0] {
            CollectWindowCommand::Spawn { request, .. } => request,
            other => panic!("unexpected collect command: {other:?}"),
        };
        ctx.collect_window_commands.clear();
        let rect = Rect {
            min: Vec2::new(200.0, 100.0),
            max: Vec2::new(500.0, 300.0),
        };
        ctx.collect_window_snapshot =
            Some(collect_snapshot(request, CollectWindowKind::Meme, rect));
        task.run(&mut goose, &mut ctx); // wait -> run
        ctx.collect_window_commands.clear();
        goose.rig.beak_tip = Vec2::new(350.0, 200.0);
        task.run(&mut goose, &mut ctx); // run -> dragging
        ctx.collect_window_commands.clear();
        goose.position = (ctx.bounds.min + ctx.bounds.max) * 0.5;
        goose.rig.beak_tip = goose.position;
        task.run(&mut goose, &mut ctx); // dragging -> release
        ctx.collect_window_commands.clear();
        task.run(&mut goose, &mut ctx); // release once
        ctx.collect_window_commands.clear();
        ctx.now += COLLECT_VISIBLE_DWELL + 0.1;

        assert!(task.run(&mut goose, &mut ctx));
        assert_eq!(
            ctx.collect_window_commands.as_slice(),
            &[CollectWindowCommand::Close {
                id: crate::collect_window::CollectWindowId(99)
            }]
        );
    }
}
