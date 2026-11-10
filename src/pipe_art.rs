// Noodle — original vector drawings of each bagpipe type (drawn in code, no images).
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Each drawing shows what makes that instrument recognisable: how it's blown
// (blowpipe vs bellows), how many drones and where they sit, the bag, the chanter.

use eframe::egui::{Color32, Painter, Pos2, Rect, Shape, Stroke, Vec2};

const WOOD: Color32 = Color32::from_rgb(0x5a, 0x37, 0x1f);
const WOOD_LT: Color32 = Color32::from_rgb(0x8a, 0x5a, 0x35);
const MOUNT: Color32 = Color32::from_rgb(0xd8, 0xd2, 0xc4);
const LEATHER: Color32 = Color32::from_rgb(0x4a, 0x30, 0x22);

struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    s: f32,
}

impl<'a> Pen<'a> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.r.left() + x * self.r.width(), self.r.top() + y * self.r.height())
    }
    fn pipe(&self, a: (f32, f32), b: (f32, f32), w: f32, col: Color32) {
        self.p.line_segment([self.at(a.0, a.1), self.at(b.0, b.1)], Stroke::new(w * self.s, col));
    }
    /// a drone: wooden tube with silver ferrules and a cap
    fn drone(&self, a: (f32, f32), b: (f32, f32), w: f32, bell: bool) {
        self.pipe(a, b, w, WOOD);
        for t in [0.33f32, 0.66] {
            let x = a.0 + (b.0 - a.0) * t;
            let y = a.1 + (b.1 - a.1) * t;
            self.p.circle_filled(self.at(x, y), w * 0.75 * self.s, MOUNT);
        }
        let end = self.at(b.0, b.1);
        if bell {
            self.p.circle_filled(end, w * 1.5 * self.s, WOOD_LT);
            self.p.circle_filled(end, w * 0.8 * self.s, MOUNT);
        } else {
            self.p.circle_filled(end, w * 0.9 * self.s, MOUNT);
        }
    }
    fn ellipse(&self, c: (f32, f32), rx: f32, ry: f32, rot: f32, fill: Color32, stroke: Color32) -> Vec<Pos2> {
        let (sn, cs) = rot.sin_cos();
        let pts: Vec<Pos2> = (0..40)
            .map(|i| {
                let a = i as f32 / 40.0 * std::f32::consts::TAU;
                let x = rx * a.cos();
                let y = ry * a.sin();
                self.at(c.0 + x * cs - y * sn, c.1 + x * sn + y * cs)
            })
            .collect();
        self.p.add(Shape::convex_polygon(pts.clone(), fill, Stroke::new(1.2 * self.s, stroke)));
        pts
    }
    fn chanter(&self, top: (f32, f32), bot: (f32, f32), w: f32, holes: usize, keys: bool) {
        self.pipe(top, bot, w, WOOD);
        // bell
        self.p.circle_filled(self.at(bot.0, bot.1), w * 0.95 * self.s, WOOD_LT);
        for i in 0..holes {
            let t = 0.2 + i as f32 / holes as f32 * 0.65;
            let x = top.0 + (bot.0 - top.0) * t;
            let y = top.1 + (bot.1 - top.1) * t;
            self.p.circle_filled(self.at(x, y), 0.9 * self.s, Color32::from_gray(20));
            if keys && i % 2 == 1 {
                self.p.rect_filled(Rect::from_center_size(self.at(x + 0.03, y), Vec2::new(4.0 * self.s, 2.0 * self.s)), 1.0, MOUNT);
            }
        }
        self.p.circle_filled(self.at(top.0, top.1), w * 0.8 * self.s, MOUNT); // stock ferrule
    }
    fn blowpipe(&self, from: (f32, f32), to: (f32, f32)) {
        self.pipe(from, to, 2.4, WOOD);
        self.p.circle_filled(self.at(to.0, to.1), 1.8 * self.s, MOUNT);
    }
    fn bellows(&self, at: (f32, f32), bag: (f32, f32)) {
        let pts = vec![self.at(at.0, at.1 - 0.06), self.at(at.0 + 0.2, at.1 - 0.02), self.at(at.0 + 0.2, at.1 + 0.03), self.at(at.0, at.1 + 0.07)];
        self.p.add(Shape::convex_polygon(pts, LEATHER, Stroke::new(1.0 * self.s, WOOD_LT)));
        // boards
        self.p.line_segment([self.at(at.0, at.1 - 0.06), self.at(at.0 + 0.2, at.1 - 0.02)], Stroke::new(2.0 * self.s, WOOD_LT));
        self.p.line_segment([self.at(at.0, at.1 + 0.07), self.at(at.0 + 0.2, at.1 + 0.03)], Stroke::new(2.0 * self.s, WOOD_LT));
        // wind tube to the bag
        self.pipe((at.0 + 0.2, at.1), bag, 1.6, Color32::from_gray(150));
    }
}

/// Draw pipe type `idx` (index into pipes::PIPES) inside `r`.
pub fn draw(p: &Painter, r: Rect, idx: usize) {
    let pen = Pen { p, r, s: (r.width().min(r.height()) / 110.0).max(0.4) };
    match idx {
        // Great Highland: mouth-blown, tartan bag, 2 tenors + tall bass over the shoulder
        0 => {
            pen.drone((0.50, 0.44), (0.66, 0.08), 3.2, false);
            pen.drone((0.53, 0.45), (0.76, 0.12), 3.2, false);
            pen.drone((0.56, 0.46), (0.95, 0.02), 4.0, false);
            pen.pipe((0.66, 0.08), (0.95, 0.02), 1.0, Color32::from_rgb(0x1a, 0x5a, 0x2e)); // drone cords
            let bag = pen.ellipse((0.45, 0.55), 0.24, 0.15, -0.35, Color32::from_rgb(0x9e, 0x1b, 0x24), Color32::from_rgb(0x30, 0x10, 0x10));
            // tartan stripes, kept inside the bag outline
            for i in 0..5 {
                let t = 0.2 + i as f32 * 0.15;
                let a = bag[(i * 3) % bag.len()];
                let b = bag[(20 + i * 3) % bag.len()];
                let mid = Pos2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t);
                let _ = mid;
                p.line_segment([a, b], Stroke::new(1.2 * pen.s, Color32::from_rgb(0x14, 0x3c, 0x22).gamma_multiply(0.8)));
            }
            pen.blowpipe((0.32, 0.46), (0.12, 0.22));
            pen.chanter((0.34, 0.66), (0.26, 0.98), 3.4, 7, false);
        }
        // Scottish smallpipes: bellows, small bag, 3 short drones in a common stock over the arm
        1 => {
            pen.bellows((0.62, 0.80), (0.52, 0.62));
            pen.drone((0.48, 0.45), (0.86, 0.26), 2.4, false);
            pen.drone((0.49, 0.47), (0.82, 0.33), 2.4, false);
            pen.drone((0.50, 0.49), (0.76, 0.40), 2.4, false);
            pen.ellipse((0.42, 0.56), 0.18, 0.12, -0.25, Color32::from_rgb(0x1d, 0x4a, 0x2f), Color32::from_rgb(0x0c, 0x24, 0x16));
            pen.chanter((0.34, 0.65), (0.28, 0.93), 2.8, 7, false);
        }
        // Border: bellows, highland-style standing drones but shorter
        2 => {
            pen.bellows((0.66, 0.82), (0.52, 0.62));
            pen.drone((0.50, 0.44), (0.64, 0.14), 2.8, false);
            pen.drone((0.53, 0.45), (0.73, 0.17), 2.8, false);
            pen.drone((0.56, 0.46), (0.88, 0.08), 3.4, false);
            pen.ellipse((0.44, 0.56), 0.21, 0.13, -0.3, Color32::from_rgb(0x2a, 0x3e, 0x6e), Color32::from_rgb(0x10, 0x18, 0x30));
            pen.chanter((0.34, 0.66), (0.27, 0.97), 3.2, 7, false);
        }
        // Uilleann: bellows, drones + regulators lying across the lap, chanter down to the knee
        3 => {
            pen.bellows((0.70, 0.40), (0.62, 0.48));
            pen.ellipse((0.66, 0.56), 0.13, 0.09, 0.2, Color32::from_rgb(0x6a, 0x22, 0x2c), Color32::from_rgb(0x30, 0x0e, 0x12));
            for (i, len) in [0.62f32, 0.52, 0.42].iter().enumerate() {
                let y = 0.62 + i as f32 * 0.045;
                pen.drone((0.62, y), (0.62 - len, y - 0.02), 2.2, false);
            }
            // regulators with keys
            for (i, len) in [0.36f32, 0.30, 0.24].iter().enumerate() {
                let y = 0.77 + i as f32 * 0.045;
                pen.pipe((0.62, y), (0.62 - len, y), 2.2, WOOD);
                for k in 0..3 {
                    let x = 0.58 - k as f32 * 0.07;
                    p.rect_filled(Rect::from_center_size(pen.at(x, y - 0.02), Vec2::new(5.0 * pen.s, 2.0 * pen.s)), 1.0, MOUNT);
                }
            }
            pen.chanter((0.70, 0.60), (0.74, 0.98), 3.0, 8, true);
        }
        // Northumbrian: bellows, 4 slim drones in one stock, small keyed chanter
        4 => {
            pen.bellows((0.64, 0.82), (0.50, 0.62));
            for i in 0..4 {
                let o = i as f32 * 0.025;
                pen.drone((0.48 + o, 0.45 + o * 0.5), (0.90 - o * 2.0, 0.22 + o * 2.5), 1.8, false);
            }
            pen.ellipse((0.40, 0.56), 0.16, 0.11, -0.2, Color32::from_rgb(0x2f, 0x2f, 0x35), Color32::from_rgb(0x10, 0x10, 0x14));
            pen.chanter((0.33, 0.64), (0.29, 0.90), 2.4, 8, true);
        }
        // Galician gaita: mouth-blown, one big ronco over the shoulder (+ small ronquilla), fringed cover
        5 | 6 => {
            let galician = idx == 5;
            pen.drone((0.55, 0.42), (0.96, 0.04), 4.2, true);
            if galician {
                pen.drone((0.50, 0.44), (0.62, 0.26), 2.0, false);
            }
            let cover = if galician { Color32::from_rgb(0x1f, 0x4e, 0x9a) } else { Color32::from_rgb(0x1e, 0x6a, 0x3a) };
            let bag = pen.ellipse((0.45, 0.56), 0.23, 0.15, -0.25, cover, Color32::from_rgb(0x0a, 0x1a, 0x30));
            // fringe along the bottom of the cover
            for (i, pt) in bag.iter().enumerate() {
                if i % 2 == 0 && pt.y > pen.at(0.0, 0.58).y {
                    p.line_segment([*pt, *pt + Vec2::new(0.0, 5.0 * pen.s)], Stroke::new(1.0 * pen.s, Color32::from_rgb(0xe0, 0xb0, 0x30)));
                }
            }
            pen.blowpipe((0.32, 0.46), (0.12, 0.24));
            pen.chanter((0.34, 0.67), (0.27, 0.98), 3.4, 7, false);
        }
        // Breton biniou kozh: tiny, mouth-blown, short chanter, one short drone
        7 => {
            pen.drone((0.52, 0.46), (0.74, 0.22), 2.4, false);
            pen.ellipse((0.47, 0.56), 0.12, 0.09, -0.25, Color32::from_rgb(0x1c, 0x1c, 0x22), Color32::from_rgb(0x60, 0x60, 0x70));
            pen.blowpipe((0.40, 0.50), (0.24, 0.32));
            pen.chanter((0.42, 0.63), (0.38, 0.84), 2.6, 6, false);
        }
        // Medieval market pipes: plain sheepskin bag, two drones in separate stocks over the shoulder
        9 => {
            pen.drone((0.50, 0.42), (0.70, 0.10), 3.0, true);
            pen.drone((0.58, 0.46), (0.95, 0.04), 3.8, true);
            let bag = pen.ellipse((0.45, 0.56), 0.24, 0.16, -0.3, Color32::from_rgb(0xa8, 0x86, 0x5c), Color32::from_rgb(0x5a, 0x40, 0x24));
            // stitched seam
            for (i, pt) in bag.iter().enumerate() {
                if i % 3 == 0 && i < 20 {
                    p.circle_filled(*pt + Vec2::new(0.0, 3.0 * pen.s), 0.8 * pen.s, Color32::from_rgb(0x3a, 0x28, 0x14));
                }
            }
            pen.blowpipe((0.32, 0.46), (0.12, 0.22));
            pen.chanter((0.34, 0.67), (0.27, 0.98), 3.6, 7, false);
        }
        // Hümmelchen: small bag, two short drones in one stock
        10 => {
            pen.drone((0.50, 0.46), (0.74, 0.26), 2.2, false);
            pen.drone((0.52, 0.48), (0.80, 0.34), 2.2, false);
            pen.ellipse((0.44, 0.57), 0.15, 0.10, -0.25, Color32::from_rgb(0x8a, 0x6a, 0x48), Color32::from_rgb(0x4a, 0x34, 0x20));
            pen.blowpipe((0.36, 0.50), (0.20, 0.32));
            pen.chanter((0.37, 0.64), (0.32, 0.88), 2.6, 7, false);
        }
        // Bulgarian gaida: whole goatskin bag, one long drone hanging back, chanter with a horn bell
        _ => {
            pen.drone((0.64, 0.56), (0.98, 0.94), 3.4, true);
            let bag = pen.ellipse((0.47, 0.54), 0.25, 0.18, -0.15, Color32::from_rgb(0x7a, 0x5a, 0x3e), Color32::from_rgb(0x3a, 0x26, 0x18));
            // fur
            for (i, pt) in bag.iter().enumerate() {
                if i % 2 == 1 {
                    let c = pen.at(0.47, 0.54);
                    let d = (*pt - c) * 0.12;
                    p.line_segment([*pt, *pt + d], Stroke::new(1.0 * pen.s, Color32::from_rgb(0x5a, 0x40, 0x2a)));
                }
            }
            for k in 0..14 {
                let x = 0.30 + (k * 37 % 100) as f32 / 100.0 * 0.32;
                let y = 0.44 + (k * 53 % 100) as f32 / 100.0 * 0.2;
                p.circle_filled(pen.at(x, y), 0.9 * pen.s, Color32::from_rgb(0x9a, 0x7a, 0x5a));
            }
            pen.blowpipe((0.30, 0.44), (0.12, 0.20));
            pen.chanter((0.34, 0.66), (0.28, 0.94), 3.2, 8, false);
            p.circle_filled(pen.at(0.28, 0.97), 4.0 * pen.s, Color32::from_rgb(0xc8, 0xb8, 0x90)); // horn bell
        }
    }
}
