//! Remote-control toys (ADR 0059): a ball, a stick and bread crumbs the goose plays with.
//!
//! Toys are placed and moved only through remote control. The goose kicks balls it runs into,
//! fetches sticks back to the pointer, and eats crumbs. The art is the Desktop Goose ToysMod and
//! BreadCrumbs mod images listed in `THIRD_PARTY_ASSETS.md`; simulation stays platform-free.

use crate::entity::GooseEntity;
use crate::layout::DesktopLayout;
use crate::math::Vec2;
use crate::time::DT;

/// Bounded so a status frame listing every toy fits the 128-byte IPC limit.
pub const MAX_TOYS: usize = 5;
/// Seconds a toy stays before fading away.
pub const TOY_LIFETIME: f64 = 180.0;
pub const TOY_FADE_SECONDS: f64 = 3.0;
pub const BALL_RADIUS: f32 = 20.0;
/// Covers the largest drawn toy (crumbs) plus rotation and antialiasing.
pub const TOY_RENDER_RADIUS: f32 = 34.0;
const BALL_FRICTION: f32 = 1.4;
const BALL_MAX_SPEED: f32 = 900.0;
const BALL_BOUNCE: f32 = 0.7;
const KICK_REACH: f32 = 18.0;
const KICK_MIN_GOOSE_SPEED: f32 = 20.0;
const KICK_COOLDOWN: f64 = 0.3;
const CARRY_OFFSET: Vec2 = Vec2::new(0.0, 4.0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToyKind {
    Ball,
    Stick,
    Crumbs,
}

impl ToyKind {
    /// One-letter wire code used by the local IPC protocol.
    pub fn code(self) -> char {
        match self {
            Self::Ball => 'B',
            Self::Stick => 'S',
            Self::Crumbs => 'C',
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        match code {
            "B" => Some(Self::Ball),
            "S" => Some(Self::Stick),
            "C" => Some(Self::Crumbs),
            _ => None,
        }
    }

    /// Name used by the relay protocol and panel.
    pub fn name(self) -> &'static str {
        match self {
            Self::Ball => "ball",
            Self::Stick => "stick",
            Self::Crumbs => "crumbs",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "ball" => Some(Self::Ball),
            "stick" => Some(Self::Stick),
            "crumbs" => Some(Self::Crumbs),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ToyId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Toy {
    pub id: ToyId,
    pub kind: ToyKind,
    pub position: Vec2,
    pub velocity: Vec2,
    /// Distance rolled, for the ball's animation frame.
    pub rolled: f32,
    pub created_at: f64,
    /// Held in the goose's beak (sticks only).
    pub carried: bool,
    pub last_kicked: Option<f64>,
}

impl Toy {
    /// Opacity in `0..=1`, fading out over the last seconds of the toy's lifetime.
    pub fn opacity(&self, now: f64) -> f32 {
        let remaining = self.created_at + TOY_LIFETIME - now;
        (remaining / TOY_FADE_SECONDS).clamp(0.0, 1.0) as f32
    }
}

#[derive(Debug, Clone, Default)]
pub struct ToyState {
    toys: Vec<Toy>,
    next_id: u64,
}

impl ToyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn toys(&self) -> &[Toy] {
        &self.toys
    }

    pub fn is_empty(&self) -> bool {
        self.toys.is_empty()
    }

    pub fn get(&self, id: ToyId) -> Option<&Toy> {
        self.toys.iter().find(|toy| toy.id == id)
    }

    pub fn clear(&mut self) {
        self.toys.clear();
    }

    /// Place a toy at `position` (already clamped by the caller); `None` when full.
    pub fn place(&mut self, now: f64, kind: ToyKind, position: Vec2) -> Option<ToyId> {
        if self.toys.len() >= MAX_TOYS {
            return None;
        }
        self.next_id += 1;
        let id = ToyId(self.next_id);
        self.toys.push(Toy {
            id,
            kind,
            position,
            velocity: Vec2::ZERO,
            rolled: 0.0,
            created_at: now,
            carried: false,
            last_kicked: None,
        });
        Some(id)
    }

    /// Move a toy that is not in the goose's beak. Returns `false` when it is gone or carried.
    pub fn move_toy(&mut self, id: ToyId, position: Vec2) -> bool {
        match self
            .toys
            .iter_mut()
            .find(|toy| toy.id == id && !toy.carried)
        {
            Some(toy) => {
                toy.position = position;
                toy.velocity = Vec2::ZERO;
                true
            }
            None => false,
        }
    }

    pub fn remove(&mut self, id: ToyId) {
        self.toys.retain(|toy| toy.id != id);
    }

    pub fn pick_up(&mut self, id: ToyId) {
        if let Some(toy) = self.toys.iter_mut().find(|toy| toy.id == id) {
            toy.carried = true;
            toy.velocity = Vec2::ZERO;
        }
    }

    pub fn drop_toy(&mut self, id: ToyId, position: Vec2) {
        if let Some(toy) = self.toys.iter_mut().find(|toy| toy.id == id) {
            toy.carried = false;
            toy.position = position;
        }
    }

    /// Drop anything the goose holds (its play errand ended or was interrupted).
    pub fn release_all(&mut self) {
        for toy in &mut self.toys {
            toy.carried = false;
        }
    }

    /// Advance one fixed step: expire, carry, kick and roll toys inside the real monitors.
    pub fn tick(&mut self, now: f64, layout: &DesktopLayout, goose: &GooseEntity) {
        self.toys.retain(|toy| toy.opacity(now) > 0.0);
        let goose_speed = goose.velocity.magnitude();
        for toy in &mut self.toys {
            if toy.carried {
                toy.position = goose.rig.beak_tip + CARRY_OFFSET;
                continue;
            }
            if toy.kind == ToyKind::Ball {
                let cooled = toy.last_kicked.is_none_or(|at| now - at >= KICK_COOLDOWN);
                let offset = toy.position - goose.position;
                if cooled
                    && goose_speed > KICK_MIN_GOOSE_SPEED
                    && offset.magnitude() < BALL_RADIUS + KICK_REACH
                {
                    let away = if offset.magnitude() > f32::EPSILON {
                        offset.normalize()
                    } else {
                        goose.velocity.normalize()
                    };
                    toy.velocity = clamp_speed(goose.velocity * 1.5 + away * 120.0);
                    toy.last_kicked = Some(now);
                }
            }
            if toy.velocity == Vec2::ZERO {
                continue;
            }
            let next = toy.position + toy.velocity * DT;
            let inside = layout.clamp_point(next);
            if inside.x != next.x {
                toy.velocity.x = -toy.velocity.x * BALL_BOUNCE;
            }
            if inside.y != next.y {
                toy.velocity.y = -toy.velocity.y * BALL_BOUNCE;
            }
            toy.rolled += Vec2::distance(toy.position, inside);
            toy.position = inside;
            toy.velocity = toy.velocity * (-BALL_FRICTION * DT).exp();
            if toy.velocity.magnitude() < 4.0 {
                toy.velocity = Vec2::ZERO;
            }
        }
    }
}

fn clamp_speed(velocity: Vec2) -> Vec2 {
    let speed = velocity.magnitude();
    if speed > BALL_MAX_SPEED {
        velocity * (BALL_MAX_SPEED / speed)
    } else {
        velocity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::Rect;

    fn layout() -> DesktopLayout {
        DesktopLayout::single(Rect::new(Vec2::ZERO, Vec2::new(1000.0, 800.0)))
    }

    #[test]
    fn placement_is_bounded_and_ids_are_unique() {
        let mut toys = ToyState::new();
        let ids: Vec<_> = (0..MAX_TOYS)
            .map(|_| {
                toys.place(0.0, ToyKind::Ball, Vec2::new(10.0, 10.0))
                    .unwrap()
            })
            .collect();
        assert!(toys.place(0.0, ToyKind::Stick, Vec2::ZERO).is_none());
        for (index, id) in ids.iter().enumerate() {
            assert!(ids[index + 1..].iter().all(|other| other != id));
        }
    }

    #[test]
    fn kind_codes_and_names_round_trip() {
        for kind in [ToyKind::Ball, ToyKind::Stick, ToyKind::Crumbs] {
            assert_eq!(ToyKind::from_code(&kind.code().to_string()), Some(kind));
            assert_eq!(ToyKind::from_name(kind.name()), Some(kind));
        }
        assert_eq!(ToyKind::from_name("laser"), None);
    }

    #[test]
    fn a_running_goose_kicks_the_ball_which_rolls_slows_and_stays_on_screen() {
        let mut toys = ToyState::new();
        let id = toys
            .place(0.0, ToyKind::Ball, Vec2::new(980.0, 400.0))
            .unwrap();
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(965.0, 400.0);
        goose.velocity = Vec2::new(400.0, 0.0);
        toys.tick(0.0, &layout(), &goose);
        let kicked = *toys.get(id).unwrap();
        assert!(kicked.last_kicked.is_some());
        assert!(kicked.velocity.x > 0.0);

        goose.position = Vec2::new(100.0, 100.0);
        goose.velocity = Vec2::ZERO;
        for step in 1..(120 * 10) {
            toys.tick(step as f64 * DT as f64, &layout(), &goose);
            let toy = toys.get(id).unwrap();
            assert!(layout().contains(toy.position));
        }
        let rested = toys.get(id).unwrap();
        assert_eq!(rested.velocity, Vec2::ZERO);
        assert!(rested.rolled > 0.0);
        assert!(rested.position.x < 1000.0, "bounced off the right edge");
    }

    #[test]
    fn carried_toys_follow_the_beak_and_cannot_be_moved() {
        let mut toys = ToyState::new();
        let id = toys
            .place(0.0, ToyKind::Stick, Vec2::new(10.0, 10.0))
            .unwrap();
        toys.pick_up(id);
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(300.0, 300.0);
        toys.tick(0.0, &layout(), &goose);
        assert_eq!(
            toys.get(id).unwrap().position,
            goose.rig.beak_tip + CARRY_OFFSET
        );
        assert!(!toys.move_toy(id, Vec2::ZERO));
        toys.release_all();
        assert!(toys.move_toy(id, Vec2::ZERO));
    }

    #[test]
    fn toys_fade_and_expire() {
        let mut toys = ToyState::new();
        let id = toys
            .place(0.0, ToyKind::Crumbs, Vec2::new(10.0, 10.0))
            .unwrap();
        let mut goose = GooseEntity::new();
        goose.position = Vec2::new(500.0, 500.0);
        let toy = *toys.get(id).unwrap();
        assert_eq!(toy.opacity(0.0), 1.0);
        assert!(toy.opacity(TOY_LIFETIME - 1.0) < 1.0);
        toys.tick(TOY_LIFETIME, &layout(), &goose);
        assert!(toys.is_empty());
    }
}
