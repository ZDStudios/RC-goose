//! Raster art for remote-control toys. The PNGs are the Desktop Goose ToysMod and BreadCrumbs
//! images (see `THIRD_PARTY_ASSETS.md`), decoded once and drawn premultiplied.

use crate::math::Vec2;
use crate::toys::{Toy, ToyKind, BALL_RADIUS};
use std::sync::OnceLock;
use tiny_skia::{FilterQuality, Pixmap, PixmapPaint, Transform};

const BALL: [&[u8]; 3] = [
    include_bytes!("../../../../Assets/Toys/ball.png"),
    include_bytes!("../../../../Assets/Toys/ball2.png"),
    include_bytes!("../../../../Assets/Toys/ball3.png"),
];
const STICK: [&[u8]; 3] = [
    include_bytes!("../../../../Assets/Toys/stick.png"),
    include_bytes!("../../../../Assets/Toys/stick2.png"),
    include_bytes!("../../../../Assets/Toys/stick3.png"),
];
const CRUMBS: &[u8] = include_bytes!("../../../../Assets/Toys/crumbs.png");
/// Crumbs art is 85 px; draw it at roughly the ball's size.
const CRUMBS_SCALE: f32 = 0.55;

struct Sprites {
    ball: Vec<Pixmap>,
    stick: Vec<Pixmap>,
    crumbs: Pixmap,
}

fn sprites() -> &'static Sprites {
    static SPRITES: OnceLock<Sprites> = OnceLock::new();
    SPRITES.get_or_init(|| {
        let decode = |bytes: &[u8]| Pixmap::decode_png(bytes).expect("bundled toy PNG decodes");
        Sprites {
            ball: BALL.iter().map(|bytes| decode(bytes)).collect(),
            stick: STICK.iter().map(|bytes| decode(bytes)).collect(),
            crumbs: decode(CRUMBS),
        }
    })
}

/// Draw one toy centred on its position, relative to the canvas `origin`.
pub(super) fn render(pixmap: &mut Pixmap, toy: &Toy, now: f64, origin: Vec2) {
    let opacity = toy.opacity(now);
    if opacity <= 0.0 {
        return;
    }
    let sprites = sprites();
    let (image, scale) = match toy.kind {
        ToyKind::Ball => {
            // Three roll frames: advance one frame per third of the ball's circumference.
            let step = std::f32::consts::TAU * BALL_RADIUS / 3.0;
            let frame = (toy.rolled / step) as usize % sprites.ball.len();
            (&sprites.ball[frame], 1.0)
        }
        ToyKind::Stick => (&sprites.stick[toy.id.0 as usize % sprites.stick.len()], 1.0),
        ToyKind::Crumbs => (&sprites.crumbs, CRUMBS_SCALE),
    };
    let center = toy.position - origin;
    let transform = Transform::from_translate(center.x, center.y)
        .pre_scale(scale, scale)
        .pre_translate(
            -(image.width() as f32) / 2.0,
            -(image.height() as f32) / 2.0,
        );
    let paint = PixmapPaint {
        opacity,
        quality: if scale == 1.0 {
            FilterQuality::Nearest
        } else {
            FilterQuality::Bilinear
        },
        ..PixmapPaint::default()
    };
    pixmap.draw_pixmap(0, 0, image.as_ref(), &paint, transform, None);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toys::ToyState;

    #[test]
    fn every_bundled_toy_decodes_and_draws_inside_its_render_radius() {
        let sprites = sprites();
        assert_eq!(sprites.ball.len(), 3);
        assert_eq!(sprites.stick.len(), 3);
        let mut toys = ToyState::new();
        for kind in [ToyKind::Ball, ToyKind::Stick, ToyKind::Crumbs] {
            toys.place(0.0, kind, Vec2::new(40.0, 40.0));
        }
        for toy in toys.toys() {
            let size = (crate::toys::TOY_RENDER_RADIUS * 2.0) as u32 + 8;
            let mut pixmap = Pixmap::new(size, size).unwrap();
            render(
                &mut pixmap,
                toy,
                0.0,
                Vec2::new(40.0 - size as f32 / 2.0, 40.0 - size as f32 / 2.0),
            );
            let drawn = pixmap.pixels().iter().filter(|p| p.alpha() > 0).count();
            assert!(drawn > 100, "{:?} drew {drawn} pixels", toy.kind);
            let edge = pixmap.pixels().iter().enumerate().filter(|(i, p)| {
                let (x, y) = (*i as u32 % size, *i as u32 / size);
                p.alpha() > 0 && (x < 2 || y < 2 || x >= size - 2 || y >= size - 2)
            });
            assert_eq!(
                edge.count(),
                0,
                "{:?} stays inside its damage radius",
                toy.kind
            );
        }
    }
}
