//! Clean-room continuous projected goose renderer (ADR 0041).
//! Native antialiasing with optional bounded supersampling for exported art.

mod canvas;
mod geom;
mod leaf_art;
mod leaves;
mod projected;
mod svg;
mod toys;

use crate::autumn::{AutumnLeafColor, AutumnState};
use crate::footmarks::{FootMarkTiming, FootMarks};
use crate::math::Vec2;
use crate::rig::{GoosePose, Rig};
use crate::toys::ToyState;
pub use canvas::DamageCanvas;
use geom::{disc, ellipse, paint};
use std::cell::RefCell;
pub use svg::render_rig_svg;
use tiny_skia::{Color, FilterQuality, Pixmap, PixmapPaint, Transform};

/// Goose raster supersample factor (rendered at 2x, composited down at 0.5x).
pub const GOOSE_SUPERSAMPLE: f32 = 2.0;

const MUD: (u8, u8, u8) = (0x5a, 0x40, 0x28);

thread_local! {
    /// Platform threads reuse the same bounded canvas implementation as native presentation.
    static LAYER_SCRATCH: RefCell<DamageCanvas> = RefCell::new(DamageCanvas::default());
}

/// User-customizable goose palette — six compatible configurable tones.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderPalette {
    /// Base body/head/neck tone.
    pub goose_white: (u8, u8, u8),
    /// Soft shading: throat, underbody, behind-wing, tail underside.
    pub goose_shade: (u8, u8, u8),
    /// Wing accent, blended with the base for restrained shading.
    pub goose_wing: (u8, u8, u8),
    /// Beak top, legs, feet.
    pub goose_orange: (u8, u8, u8),
    /// Beak underside/mouth-line, nostril, far leg.
    pub goose_orange_dark: (u8, u8, u8),
    /// Thin contrast outline around body-toned forms.
    pub goose_outline: (u8, u8, u8),
}

impl Default for RenderPalette {
    fn default() -> Self {
        Self {
            goose_white: (0xfc, 0xfc, 0xf6),
            goose_shade: (0xcf, 0xd6, 0xcc),
            goose_wing: (0x8c, 0x9c, 0x90),
            goose_orange: (0xfc, 0x79, 0x27),
            goose_orange_dark: (0xd1, 0x55, 0x1b),
            goose_outline: (0x9c, 0xa8, 0x9c),
        }
    }
}

impl RenderPalette {
    /// Build a palette from the three legacy config tones (M15 files predate the
    /// six-tone palette): the missing tones are derived so old configs stay coherent.
    pub fn from_legacy(
        goose_white: (u8, u8, u8),
        goose_orange: (u8, u8, u8),
        goose_outline: (u8, u8, u8),
    ) -> Self {
        let d = Self::default();
        Self {
            goose_white,
            goose_shade: mix(goose_white, goose_outline, 0.55),
            goose_wing: d.goose_wing,
            goose_orange,
            goose_orange_dark: darken(goose_orange),
            goose_outline,
        }
    }
}

fn darken(rgb: (u8, u8, u8)) -> (u8, u8, u8) {
    (
        (rgb.0 as f32 * 0.85) as u8,
        (rgb.1 as f32 * 0.73) as u8,
        (rgb.2 as f32 * 0.55) as u8,
    )
}

fn mix(a: (u8, u8, u8), b: (u8, u8, u8), t: f32) -> (u8, u8, u8) {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

/// Which side of the goose the Autumn leaves render on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutumnRenderLayer {
    BelowGoose,
    AboveGoose,
}

fn leaf_palette_index(color: AutumnLeafColor) -> usize {
    match color {
        AutumnLeafColor::Gold => 0,
        AutumnLeafColor::Orange => 1,
        AutumnLeafColor::Red => 2,
        AutumnLeafColor::Brown => 3,
    }
}

/// Render remote-control toys. Toys on the ground go in `BelowGoose`; a toy in the goose's beak
/// goes in `AboveGoose`.
pub fn render_toys(
    pixmap: &mut Pixmap,
    toys: &ToyState,
    now: f64,
    origin: Vec2,
    layer: AutumnRenderLayer,
) {
    for toy in toys.toys() {
        if toy.carried == (layer == AutumnRenderLayer::AboveGoose) {
            toys::render(pixmap, toy, now, origin);
        }
    }
}

/// Render built-in Autumn leaves. Call `BelowGoose` before the goose and `AboveGoose` after it.
pub fn render_autumn_leaves(
    pixmap: &mut Pixmap,
    autumn: &AutumnState,
    now: f64,
    origin: Vec2,
    goose_pos: Vec2,
    layer: AutumnRenderLayer,
) {
    for pile in autumn.piles() {
        let spawn = pile.spawn_scale(now);
        let fade = pile.fade_out(now);
        let alpha_scale = 1.0 - fade;
        if layer == AutumnRenderLayer::BelowGoose && pile.kicked_at.is_none() {
            let radius = pile.radius * spawn;
            let center = pile.position - origin;
            ellipse(
                pixmap,
                center,
                radius,
                radius * 0.6,
                &paint((0x31, 0x25, 0x1a), (48.0 * alpha_scale) as u8),
            );
        }

        for (index, leaf) in pile.leaves.iter().enumerate() {
            let world = pile.position + leaf.screen_offset() * spawn;
            let leaf_above_goose = world.y >= goose_pos.y;
            if matches!(layer, AutumnRenderLayer::AboveGoose) != leaf_above_goose {
                continue;
            }
            let alpha = (225.0 * alpha_scale) as u8;
            if alpha == 0 {
                continue;
            }
            let center = world - origin;
            let color = leaf_palette_index(leaf.color);
            // Stable variation is visual only: preserve the simulation's random stream and
            // the stronger velocity-dependent burst when the goose charges through a pile.
            // Position-derived tumble settles continuously instead of snapping back on landing.
            let turn = leaf.planar.magnitude() * 0.025 + leaf.z * 0.015;
            let angle = index as f32 * 2.399_963 + pile.id.0 as f32 + turn;
            let size = 0.89 + ((index / 16) % 5) as f32 * 0.055;
            let variant = (index / 4 + pile.id.0 as usize) % 8;
            leaves::render(pixmap, center, angle, size, variant, color, alpha);
        }
    }
}

/// Render the muddy footprints into `pixmap` (call before the goose so it sits on top).
pub fn render_footmarks(pixmap: &mut Pixmap, marks: &FootMarks, now: f64, origin: Vec2) {
    render_footmarks_with_timing(pixmap, marks, now, origin, FootMarkTiming::default());
}

/// Render muddy footprints with runtime timing from config.
pub fn render_footmarks_with_timing(
    pixmap: &mut Pixmap,
    marks: &FootMarks,
    now: f64,
    origin: Vec2,
    timing: FootMarkTiming,
) {
    for (mark, scale) in marks.active_with_timing(now, timing) {
        let c = mark.position - origin;
        let alpha = (180.0 * scale) as u8;
        disc(pixmap, c, 3.5 * scale, &paint(MUD, alpha));
    }
}

/// Render the rising/fading heart particles (M6 pat-streak; call after the goose so
/// hearts float on top).
pub fn render_hearts(pixmap: &mut Pixmap, hearts: &crate::hearts::Hearts, now: f64, origin: Vec2) {
    const HEART: (u8, u8, u8) = (0xff, 0x5a, 0x7a);
    const LOBE: f32 = 3.4;
    for (pos, alpha) in hearts.active(now) {
        let a = (alpha * 255.0) as u8;
        if a == 0 {
            continue;
        }
        let p = paint(HEART, a);
        let c = pos - origin;
        disc(pixmap, c + Vec2::new(-LOBE * 0.85, -LOBE * 0.45), LOBE, &p);
        disc(pixmap, c + Vec2::new(LOBE * 0.85, -LOBE * 0.45), LOBE, &p);
        let mut pb = tiny_skia::PathBuilder::new();
        let a1 = c + Vec2::new(-LOBE * 1.75, -LOBE * 0.1);
        let b1 = c + Vec2::new(LOBE * 1.75, -LOBE * 0.1);
        let d1 = c + Vec2::new(0.0, LOBE * 1.9);
        pb.move_to(a1.x, a1.y);
        pb.line_to(b1.x, b1.y);
        pb.line_to(d1.x, d1.y);
        pb.close();
        if let Some(path) = pb.finish() {
            geom::fill(pixmap, &path, &p);
        }
    }
}

/// Render sleepy-mood Z particles above the goose.
pub fn render_sleepies(
    pixmap: &mut Pixmap,
    sleepies: &crate::mood::ZParticles,
    now: f64,
    origin: Vec2,
) {
    for (pos, alpha) in sleepies.active(now) {
        let a = (alpha * 220.0) as u8;
        if a == 0 {
            continue;
        }
        let c = pos - origin;
        let p = paint((0x88, 0x99, 0xaa), a);
        let mut pb = tiny_skia::PathBuilder::new();
        pb.move_to(c.x - 4.0, c.y - 5.0);
        pb.line_to(c.x + 4.0, c.y - 5.0);
        pb.line_to(c.x - 4.0, c.y + 5.0);
        pb.line_to(c.x + 4.0, c.y + 5.0);
        if let Some(path) = pb.finish() {
            geom::stroke(pixmap, &path, &p, 1.8);
        }
    }
}

/// Render the projected goose, using a retained canvas when supersampling is requested.
fn render_rig_layer(
    pixmap: &mut Pixmap,
    rig: &Rig,
    origin: Vec2,
    palette: RenderPalette,
    supersample: f32,
) {
    let ss = supersample.max(1.0);
    if (ss - 1.0).abs() <= f32::EPSILON {
        projected::paint_goose(pixmap, rig, origin, ss, &palette);
        return;
    }
    let bb = rig.bounding_box();
    let w = (bb.width() * ss).ceil() as u32;
    let h = (bb.height() * ss).ceil() as u32;
    LAYER_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();
        let layer = scratch.prepare(w, h).expect("bounded renderer canvas");
        projected::paint_goose(layer, rig, bb.min, ss, &palette);
        let inv = 1.0 / ss;
        let transform = Transform::from_scale(inv, inv)
            .post_translate(bb.min.x - origin.x, bb.min.y - origin.y);
        pixmap.draw_pixmap(
            0,
            0,
            layer.as_ref(),
            &PixmapPaint {
                quality: if ss > 1.0 {
                    FilterQuality::Bilinear
                } else {
                    // At 1x there is no downsampling to filter. Nearest preserves the
                    // antialiased tiny-skia pixels and avoids a second bilinear pass.
                    FilterQuality::Nearest
                },
                ..PixmapPaint::default()
            },
            transform,
            None,
        );
    });
}

/// Render the complete projected pose.
pub fn render_pose_with_palette(
    pixmap: &mut Pixmap,
    pose: &GoosePose,
    origin: Vec2,
    palette: RenderPalette,
) {
    render_pose_with_palette_at_scale(pixmap, pose, origin, palette, GOOSE_SUPERSAMPLE);
}

/// Render the full pose at an explicit raster scale. Native backends may select the
/// 1x antialiased path when their compositor already performs a second image copy;
/// golden/reference renders retain [`GOOSE_SUPERSAMPLE`].
pub fn render_pose_with_palette_at_scale(
    pixmap: &mut Pixmap,
    pose: &GoosePose,
    origin: Vec2,
    palette: RenderPalette,
    supersample: f32,
) {
    render_rig_layer(pixmap, &pose.primary, origin, palette, supersample);
}

/// Render one goose view with the default palette (tests/tools).
pub fn render_rig(pixmap: &mut Pixmap, rig: &Rig, origin: Vec2) {
    render_rig_with_palette(pixmap, rig, origin, RenderPalette::default());
}

/// Render one goose view with an explicit palette.
pub fn render_rig_with_palette(
    pixmap: &mut Pixmap,
    rig: &Rig,
    origin: Vec2,
    palette: RenderPalette,
) {
    render_rig_layer(pixmap, rig, origin, palette, GOOSE_SUPERSAMPLE);
}

/// Render one goose view into a fresh transparent pixmap at an arbitrary vector scale
/// (px per world px) with **no** downsampling — crisp output for tools/marketing/site
/// assets. `origin` is the world point mapped to the pixmap's top-left; the pixmap is
/// `width`×`height` *world units* large before scaling.
pub fn render_rig_scaled(
    rig: &Rig,
    origin: Vec2,
    world_width: f32,
    world_height: f32,
    scale: f32,
    palette: RenderPalette,
) -> Option<Pixmap> {
    let w = (world_width * scale).ceil() as u32;
    let h = (world_height * scale).ceil() as u32;
    let mut pixmap = Pixmap::new(w.max(1), h.max(1))?;
    pixmap.fill(Color::TRANSPARENT);
    projected::paint_goose(&mut pixmap, rig, origin, scale, &palette);
    Some(pixmap)
}

/// Convenience for tests/tools: allocate a `width`×`height` transparent pixmap and
/// render the goose so its bounding box is centred. Returns `None` if allocation fails.
pub fn render_centered(width: u32, height: u32, rig: &Rig) -> Option<Pixmap> {
    let mut pixmap = Pixmap::new(width, height)?;
    pixmap.fill(Color::TRANSPARENT);
    let bb = rig.bounding_box();
    let bb_center = (bb.min + bb.max) * 0.5;
    let origin = bb_center - Vec2::new(width as f32 * 0.5, height as f32 * 0.5);
    render_rig(&mut pixmap, rig, origin);
    Some(pixmap)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rig::Rig;

    #[test]
    fn renders_some_opaque_pixels() {
        let pixmap = render_centered(256, 256, &Rig::default()).expect("alloc");
        let opaque = pixmap
            .data()
            .chunks_exact(4)
            .filter(|px| px[3] > 200)
            .count();
        assert!(
            opaque > 500,
            "expected a visible goose, got {opaque} opaque px"
        );
    }

    #[test]
    fn each_palette_tone_changes_pixels() {
        let rig = Rig::default();
        let base = render_centered(256, 256, &rig).expect("alloc");
        let tones: [fn(&mut RenderPalette); 6] = [
            |p| p.goose_white = (0x20, 0x60, 0xff),
            |p| p.goose_shade = (0x20, 0xff, 0x60),
            |p| p.goose_wing = (0xff, 0x20, 0x60),
            |p| p.goose_orange = (0x10, 0x10, 0xa0),
            |p| p.goose_orange_dark = (0xa0, 0x10, 0x10),
            |p| p.goose_outline = (0x00, 0x00, 0x00),
        ];
        for (i, tweak) in tones.iter().enumerate() {
            let mut pal = RenderPalette::default();
            tweak(&mut pal);
            let mut custom = Pixmap::new(256, 256).expect("alloc");
            custom.fill(Color::TRANSPARENT);
            let bb = rig.bounding_box();
            let origin = (bb.min + bb.max) * 0.5 - Vec2::new(128.0, 128.0);
            render_rig_with_palette(&mut custom, &rig, origin, pal);
            assert_ne!(base.data(), custom.data(), "tone {i} had no visible effect");
        }
    }

    #[test]
    fn legacy_palette_derives_missing_tones() {
        let p =
            RenderPalette::from_legacy((0xff, 0xff, 0xff), (0xff, 0xa5, 0x00), (0xd3, 0xd3, 0xd3));
        assert_eq!(p.goose_white, (0xff, 0xff, 0xff));
        assert_eq!(p.goose_orange, (0xff, 0xa5, 0x00));
        assert_ne!(p.goose_shade, p.goose_white);
        assert_ne!(p.goose_orange_dark, p.goose_orange);
        assert_eq!(p.goose_wing, RenderPalette::default().goose_wing);
    }

    #[test]
    fn every_heading_is_opaque_and_inside_independent_pixel_bounds() {
        for heading in (0..360).step_by(3) {
            let rig = Rig::update(Vec2::new(128.0, 150.0), heading as f32, 1.0, 0.0);
            let mut pixmap = Pixmap::new(256, 256).unwrap();
            render_rig(&mut pixmap, &rig, Vec2::ZERO);
            let bounds = rig.bounding_box();
            let mut opaque = 0;
            for (i, pixel) in pixmap.data().chunks_exact(4).enumerate() {
                if pixel[3] == 0 {
                    continue;
                }
                assert!(
                    pixel[..3].iter().all(|&c| c <= pixel[3]),
                    "premultiplied channels"
                );
                assert!(
                    bounds.contains(Vec2::new((i % 256) as f32, (i / 256) as f32)),
                    "heading {heading}: nontransparent pixel escaped bounds"
                );
                if pixel[3] == 255 {
                    opaque += 1;
                }
            }
            assert!(opaque > 1_000, "heading {heading}: body became transparent");
        }
    }

    #[test]
    fn walking_feet_remain_visible_through_full_cycles_at_every_heading() {
        use crate::{
            entity::{ParametersTable, SpeedTier},
            rig::{RigAnim, RigInput},
            time::DT,
        };
        let parameters = ParametersTable::default();
        for tier in [SpeedTier::Walk, SpeedTier::Run, SpeedTier::Charge] {
            for heading in (0..360).step_by(30) {
                let velocity = Vec2::from_angle_degrees(heading as f32) * parameters.speed(tier);
                let mut center = Vec2::ZERO;
                let mut anim = RigAnim::new(center, heading as f32);
                for tick in 0..240 {
                    center = center + velocity * DT;
                    let rig = anim
                        .update(&RigInput {
                            center,
                            direction_deg: heading as f32,
                            neck_target: 0.45,
                            speed: parameters.speed(tier),
                            velocity,
                            step_time: parameters.step_time(tier),
                            now: f64::from(tick) * f64::from(DT),
                            dt: DT,
                        })
                        .primary;
                    anim.feet.drain_plants(|_| {});
                    if tick % 4 != 0 {
                        continue;
                    }
                    let origin = rig.ground - Vec2::new(64.0, 96.0);
                    let mut pixels = Pixmap::new(128, 128).unwrap();
                    render_rig(&mut pixels, &rig, origin);
                    // The bill is above this band. Count rendered feet, including their
                    // antialiased edges, so a complete body cannot pass while floating.
                    let below = (rig.body_center.y - origin.y + 15.0) as usize;
                    let feet = pixels
                        .data()
                        .chunks_exact(4)
                        .enumerate()
                        .filter(|(i, p)| {
                            i / 128 >= below
                                && p[3] > 160
                                && p[0] > p[1].saturating_add(50)
                                && p[1] > p[2].saturating_add(30)
                        })
                        .count();
                    assert!(
                        feet >= 4,
                        "{tier:?} heading {heading} tick {tick}: only {feet} foot pixels"
                    );
                }
            }
        }
    }

    #[test]
    fn supersampled_rendering_reuses_bounded_transparent_storage() {
        LAYER_SCRATCH.with(|scratch| *scratch.borrow_mut() = DamageCanvas::default());
        let mut pixels = Pixmap::new(256, 256).unwrap();
        for frame in 0..720 {
            pixels.fill(Color::TRANSPARENT);
            let rig = Rig::update(Vec2::new(128.0, 170.0), frame as f32, 0.5, 0.0);
            render_rig(&mut pixels, &rig, Vec2::ZERO);
        }
        LAYER_SCRATCH.with(|scratch| {
            let scratch = scratch.borrow();
            assert!(scratch.allocations() <= 4);
            assert!(scratch.retained_bytes() <= 256 * 288 * 4);
        });
    }

    #[test]
    fn neck_is_connected_and_opaque_between_body_and_head_at_every_heading() {
        for heading in (0..360).step_by(15) {
            for neck in [0.0, 0.5, 1.0] {
                let rig = Rig::update(Vec2::new(128.0, 190.0), heading as f32, neck, 0.0);
                let mut pixels = Pixmap::new(256, 256).unwrap();
                render_rig(&mut pixels, &rig, Vec2::ZERO);
                for step in 0..=20 {
                    let t = step as f32 / 20.0;
                    let u = 1.0 - t;
                    let spine = rig.neck_base * (u * u * u)
                        + rig.neck_c1 * (3.0 * u * u * t)
                        + rig.neck_c2 * (3.0 * u * t * t)
                        + rig.neck_head * (t * t * t);
                    let index = (spine.y.round() as usize * 256 + spine.x.round() as usize) * 4;
                    assert_eq!(
                        pixels.data()[index + 3],
                        255,
                        "neck gap at {heading} / {neck}"
                    );
                }
            }
        }
    }

    #[test]
    fn renders_a_visible_heart() {
        use crate::hearts::Hearts;
        let mut hearts = Hearts::new();
        hearts.add(Vec2::new(64.0, 64.0), 0.0);
        let mut pixmap = Pixmap::new(128, 128).expect("alloc");
        pixmap.fill(Color::TRANSPARENT);
        render_hearts(&mut pixmap, &hearts, 0.0, Vec2::ZERO);
        let pink = pixmap
            .data()
            .chunks_exact(4)
            .filter(|px| px[3] > 120 && px[0] > 150 && px[1] < 170 && px[2] < 210)
            .count();
        assert!(pink > 15, "expected a visible heart, got {pink} pink px");
    }

    #[test]
    fn no_hearts_draws_nothing() {
        use crate::hearts::Hearts;
        let hearts = Hearts::new();
        let mut pixmap = Pixmap::new(64, 64).expect("alloc");
        pixmap.fill(Color::TRANSPARENT);
        render_hearts(&mut pixmap, &hearts, 0.0, Vec2::ZERO);
        let opaque = pixmap.data().chunks_exact(4).filter(|px| px[3] > 0).count();
        assert_eq!(opaque, 0, "no hearts → nothing drawn");
    }

    #[test]
    fn renders_visible_sleepy_particle() {
        use crate::mood::ZParticles;
        let mut z = ZParticles::new();
        z.add(Vec2::new(64.0, 64.0), 0.0);
        let mut pixmap = Pixmap::new(128, 128).expect("alloc");
        pixmap.fill(Color::TRANSPARENT);
        render_sleepies(&mut pixmap, &z, 0.0, Vec2::ZERO);
        let opaque = pixmap.data().chunks_exact(4).filter(|px| px[3] > 0).count();
        assert!(opaque > 5, "expected a visible Z particle");
    }

    #[test]
    fn toy_play_renders_every_damaged_frame_like_the_runtime() {
        use crate::toys::ToyKind;
        use crate::world::World;

        let mut world = World::new(
            crate::math::Rect::new(Vec2::ZERO, Vec2::new(1280.0, 720.0)),
            907,
        );
        while world.current_task() == "first_ux" {
            world.tick();
        }
        world
            .place_toy(ToyKind::Ball, Vec2::new(640.0, 360.0))
            .unwrap();
        world
            .place_toy(ToyKind::Crumbs, Vec2::new(200.0, 500.0))
            .unwrap();
        world
            .place_toy(ToyKind::Stick, Vec2::new(1000.0, 200.0))
            .unwrap();
        let mut drawn_carried = false;
        for frame in 0..(120 * 30) {
            world.tick();
            if frame % 2 != 0 {
                continue;
            }
            let Some(dirty) = world.visual_bounds() else {
                continue;
            };
            let width = dirty.width().ceil().max(1.0) as u32;
            let height = dirty.height().ceil().max(1.0) as u32;
            let mut canvas = Pixmap::new(width, height).expect("bounded damage canvas");
            for layer in [AutumnRenderLayer::BelowGoose, AutumnRenderLayer::AboveGoose] {
                render_toys(&mut canvas, world.toys(), world.now(), dirty.min, layer);
            }
            drawn_carried |= world.toys().toys().iter().any(|toy| toy.carried);
        }
        assert!(drawn_carried, "the fetched stick was drawn in the beak");
    }

    #[test]
    fn renders_visible_autumn_leaves() {
        use crate::autumn::AutumnState;
        use crate::entity::GooseEntity;
        use crate::math::Rect;
        use crate::rng::SplitMix64;

        let mut autumn = AutumnState::new();
        let goose = GooseEntity::new();
        let mut rng = SplitMix64::seed(99);
        autumn.tick(
            0.0,
            true,
            Rect {
                min: Vec2::ZERO,
                max: Vec2::new(128.0, 128.0),
            },
            &goose,
            &mut rng,
        );
        autumn.tick(
            10.1,
            true,
            Rect {
                min: Vec2::ZERO,
                max: Vec2::new(128.0, 128.0),
            },
            &goose,
            &mut rng,
        );

        let mut pixmap = Pixmap::new(128, 128).expect("alloc");
        pixmap.fill(Color::TRANSPARENT);
        render_autumn_leaves(
            &mut pixmap,
            &autumn,
            10.1,
            Vec2::ZERO,
            Vec2::new(64.0, 64.0),
            AutumnRenderLayer::BelowGoose,
        );
        render_autumn_leaves(
            &mut pixmap,
            &autumn,
            10.1,
            Vec2::ZERO,
            Vec2::new(64.0, 64.0),
            AutumnRenderLayer::AboveGoose,
        );

        let leafish = pixmap
            .data()
            .chunks_exact(4)
            .filter(|px| px[3] > 80 && px[0] > px[2] && px[1] > 40)
            .count();
        assert!(leafish > 30, "expected visible Autumn leaves");
    }

    #[test]
    fn empty_outside_the_goose() {
        let pixmap = render_centered(256, 256, &Rig::default()).expect("alloc");
        let w = pixmap.width() as usize;
        let h = pixmap.height() as usize;
        let data = pixmap.data();
        for &(x, y) in &[(0usize, 0usize), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            let idx = (y * w + x) * 4;
            assert_eq!(data[idx + 3], 0, "corner ({x},{y}) should be transparent");
        }
    }
}
