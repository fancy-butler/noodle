// Noodle — original instrument drawings (bagpipe icons, shruti box), drawn in code.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Flat icon style: solid tapered wooden tubes with ivory ferrules, a smooth bag
// in one accent colour. Each icon keeps what tells the instruments apart:
// blowpipe vs bellows, number/placement of drones, regulators, keys, bag size.

use eframe::egui::epaint::{CubicBezierShape, QuadraticBezierShape};
use eframe::egui::{Align2, Color32, FontId, Painter, Pos2, Rect, Shape, Stroke, StrokeKind, Vec2};

const WOOD: Color32 = Color32::from_rgb(0x6b, 0x43, 0x28);
const WOOD_DK: Color32 = Color32::from_rgb(0x45, 0x2a, 0x18);
const IVORY: Color32 = Color32::from_rgb(0xec, 0xe4, 0xd2);
const CORD: Color32 = Color32::from_rgb(0xc9, 0xa2, 0x4a);
const BRASS: Color32 = Color32::from_rgb(0xb8, 0x93, 0x4a);

struct Pen<'a> {
    p: &'a Painter,
    r: Rect,
    u: f32, // 1 unit = 1% of the icon
}

impl<'a> Pen<'a> {
    fn at(&self, x: f32, y: f32) -> Pos2 {
        Pos2::new(self.r.left() + x / 100.0 * self.r.width(), self.r.top() + y / 100.0 * self.r.height())
    }
    fn dir(&self, a: (f32, f32), b: (f32, f32)) -> Vec2 {
        (self.at(b.0, b.1) - self.at(a.0, a.1)).normalized()
    }
    /// solid tapered tube
    fn tube(&self, a: (f32, f32), b: (f32, f32), wa: f32, wb: f32, col: Color32) {
        let pa = self.at(a.0, a.1);
        let pb = self.at(b.0, b.1);
        let d = (pb - pa).normalized();
        let n = Vec2::new(-d.y, d.x);
        let (ha, hb) = (wa * self.u * 0.5, wb * self.u * 0.5);
        self.p.add(Shape::convex_polygon(vec![pa + n * ha, pb + n * hb, pb - n * hb, pa - n * ha], col, Stroke::NONE));
    }
    fn ring(&self, a: (f32, f32), b: (f32, f32), t: f32, w: f32) {
        let d = self.dir(a, b);
        let x = a.0 + (b.0 - a.0) * t;
        let y = a.1 + (b.1 - a.1) * t;
        self.tube((x, y), (x + d.x * 2.2, y + d.y * 2.2), w, w, IVORY);
    }
    fn drone(&self, a: (f32, f32), b: (f32, f32), w: f32, bell: bool) {
        self.tube(a, b, w, w * 0.8, WOOD);
        self.ring(a, b, 0.0, w * 1.35);
        self.ring(a, b, 0.48, w * 1.25);
        if bell {
            let d = self.dir(a, b);
            let tip = (b.0 + d.x * 5.0, b.1 + d.y * 5.0);
            self.tube(b, tip, w * 0.9, w * 2.4, WOOD_DK);
            self.tube(tip, (tip.0 + d.x * 1.6, tip.1 + d.y * 1.6), w * 2.5, w * 2.5, IVORY);
        } else {
            self.ring(a, b, 0.97, w * 1.5);
        }
    }
    fn chanter(&self, top: (f32, f32), bot: (f32, f32), w: f32, holes: usize, keyed: bool) {
        self.tube(top, bot, w * 0.75, w * 1.25, WOOD);
        self.ring(top, bot, 0.0, w * 1.4);
        let pa = self.at(top.0, top.1);
        let pb = self.at(bot.0, bot.1);
        let d = (pb - pa).normalized();
        self.tube(bot, (bot.0 + d.x * 3.5, bot.1 + d.y * 3.5), w * 1.3, w * 2.1, IVORY);
        for i in 0..holes {
            let t = 0.18 + i as f32 / holes as f32 * 0.68;
            let c = pa + (pb - pa) * t;
            self.p.circle_filled(c, w * self.u * 0.22, WOOD_DK);
            if keyed && i % 2 == 0 {
                let n = Vec2::new(-d.y, d.x);
                self.p.line_segment([c + n * w * self.u * 0.7, c + n * w * self.u * 1.5], Stroke::new(self.u, IVORY));
            }
        }
    }
    fn blowpipe(&self, from: (f32, f32), to: (f32, f32)) {
        self.tube(from, to, 2.6, 2.0, WOOD);
        self.ring(from, to, 0.0, 3.4);
        self.ring(from, to, 0.95, 2.8);
    }
    fn bellows(&self, c: (f32, f32), to_bag: (f32, f32)) {
        let leather = Color32::from_rgb(0x3b, 0x2a, 0x22);
        self.p.add(Shape::convex_polygon(
            vec![self.at(c.0 - 11.0, c.1 - 5.0), self.at(c.0 + 11.0, c.1 - 2.0), self.at(c.0 + 11.0, c.1 + 2.0), self.at(c.0 - 11.0, c.1 + 5.0)],
            leather,
            Stroke::NONE,
        ));
        self.tube((c.0 - 12.0, c.1 - 5.5), (c.0 + 12.0, c.1 - 2.2), 1.6, 1.6, WOOD);
        self.tube((c.0 - 12.0, c.1 + 5.5), (c.0 + 12.0, c.1 + 2.2), 1.6, 1.6, WOOD);
        let a = self.at(c.0 + 11.0, c.1);
        let b = self.at(to_bag.0, to_bag.1);
        let ctrl = Pos2::new(a.x + 4.0 * self.u, b.y + 6.0 * self.u);
        self.p.add(QuadraticBezierShape::from_points_stroke([a, ctrl, b], false, Color32::TRANSPARENT, Stroke::new(1.8 * self.u, Color32::from_gray(150))));
    }
    fn bag(&self, c: (f32, f32), w: f32, h: f32, tilt: f32, col: Color32) {
        let (s, co) = tilt.sin_cos();
        let rot = |x: f32, y: f32| self.at(c.0 + x * co - y * s, c.1 + x * s + y * co);
        let top = [rot(-w * 0.95, -h * 0.15), rot(-w * 0.4, -h * 1.05), rot(w * 0.9, -h * 1.1), rot(w * 1.0, h * 0.05)];
        let bot = [rot(w * 1.0, h * 0.05), rot(w * 0.95, h * 1.15), rot(-w * 0.45, h * 1.0), rot(-w * 0.95, -h * 0.15)];
        let mut pts: Vec<Pos2> = CubicBezierShape::from_points_stroke(top, false, Color32::TRANSPARENT, Stroke::NONE).flatten(Some(0.2));
        pts.extend(CubicBezierShape::from_points_stroke(bot, false, Color32::TRANSPARENT, Stroke::NONE).flatten(Some(0.2)));
        self.p.add(Shape::convex_polygon(pts.clone(), col, Stroke::NONE));
        let n = pts.len();
        let hl: Vec<Pos2> = pts[n / 10..n / 2].to_vec();
        self.p.add(Shape::line(hl, Stroke::new(1.3 * self.u, Color32::from_white_alpha(38))));
    }
    fn cord(&self, a: (f32, f32), b: (f32, f32), sag: f32) {
        let pa = self.at(a.0, a.1);
        let pb = self.at(b.0, b.1);
        let ctrl = Pos2::new((pa.x + pb.x) / 2.0, (pa.y + pb.y) / 2.0 + sag * self.u);
        self.p.add(QuadraticBezierShape::from_points_stroke([pa, ctrl, pb], false, Color32::TRANSPARENT, Stroke::new(1.1 * self.u, CORD)));
    }
}

/// Bagpipe icon for pipes::PIPES[idx], fitted into `r`.
pub fn draw(p: &Painter, r: Rect, idx: usize) {
    let r = Rect::from_center_size(r.center(), Vec2::splat(r.width().min(r.height())));
    let pen = Pen { p, r, u: r.width() / 100.0 };
    match idx {
        0 => {
            // Great Highland: mouth-blown, 2 tenors + long bass over the shoulder, cords
            pen.drone((52.0, 40.0), (66.0, 12.0), 3.6, false);
            pen.drone((55.0, 42.0), (73.0, 15.0), 3.6, false);
            pen.drone((58.0, 44.0), (90.0, 3.0), 4.4, false);
            pen.cord((66.0, 12.0), (90.0, 3.0), 9.0);
            pen.bag((46.0, 55.0), 22.0, 13.0, -0.4, Color32::from_rgb(0xa3, 0x1f, 0x2b));
            pen.blowpipe((34.0, 45.0), (13.0, 22.0));
            pen.chanter((33.0, 64.0), (22.0, 92.0), 3.6, 7, false);
        }
        1 => {
            // Scottish smallpipes: bellows, 3 short drones across the arm
            pen.drone((52.0, 44.0), (88.0, 27.0), 2.9, false);
            pen.drone((53.0, 47.0), (84.0, 34.0), 2.9, false);
            pen.drone((54.0, 50.0), (79.0, 41.0), 2.9, false);
            pen.bag((44.0, 56.0), 17.0, 11.0, -0.3, Color32::from_rgb(0x23, 0x5c, 0x3a));
            pen.bellows((70.0, 80.0), (52.0, 62.0));
            pen.chanter((33.0, 63.0), (25.0, 90.0), 3.1, 7, false);
        }
        2 => {
            // Border pipes: bellows + standing drones
            pen.drone((52.0, 41.0), (64.0, 15.0), 3.2, false);
            pen.drone((55.0, 43.0), (71.0, 18.0), 3.2, false);
            pen.drone((58.0, 45.0), (85.0, 8.0), 3.9, false);
            pen.cord((64.0, 15.0), (85.0, 8.0), 7.0);
            pen.bag((46.0, 56.0), 20.0, 12.0, -0.35, Color32::from_rgb(0x2b, 0x45, 0x7e));
            pen.bellows((72.0, 82.0), (54.0, 63.0));
            pen.chanter((33.0, 65.0), (23.0, 92.0), 3.4, 7, false);
        }
        3 => {
            // Uilleann: drones + keyed regulators across the lap, chanter to the knee, bellows
            for (i, len) in [56.0f32, 46.0, 37.0].iter().enumerate() {
                let y = 48.0 + i as f32 * 5.0;
                pen.drone((72.0, y), (72.0 - len, y - 2.0), 2.7, false);
            }
            for (i, len) in [34.0f32, 28.0, 22.0].iter().enumerate() {
                let y = 66.0 + i as f32 * 5.0;
                pen.tube((72.0, y), (72.0 - len, y), 2.5, 2.5, Color32::from_rgb(0x5a, 0x38, 0x22));
                for k in 0..3 {
                    let x = 66.0 - k as f32 * 8.0;
                    if x > 72.0 - len {
                        pen.tube((x, y - 2.6), (x - 4.0, y - 2.6), 1.2, 1.2, IVORY);
                    }
                }
            }
            pen.bag((78.0, 48.0), 11.0, 8.0, 0.25, Color32::from_rgb(0x7a, 0x22, 0x30));
            pen.bellows((84.0, 26.0), (76.0, 40.0));
            pen.chanter((74.0, 56.0), (80.0, 95.0), 3.2, 8, true);
        }
        4 => {
            // Northumbrian: four slim drones, small keyed chanter, bellows
            for i in 0..4 {
                let o = i as f32 * 3.0;
                pen.drone((50.0 + o * 0.6, 43.0 + o), (90.0 - o * 1.2, 22.0 + o * 2.4), 2.2, false);
            }
            pen.bag((42.0, 56.0), 15.0, 10.0, -0.25, Color32::from_rgb(0x3a, 0x3a, 0x44));
            pen.bellows((70.0, 82.0), (50.0, 62.0));
            pen.chanter((32.0, 62.0), (27.0, 88.0), 2.8, 8, true);
        }
        5 | 6 => {
            // Galician / Asturian gaita: big ronco with a bell over the shoulder
            let galician = idx == 5;
            pen.drone((56.0, 41.0), (88.0, 9.0), 4.6, true);
            if galician {
                pen.drone((51.0, 42.0), (62.0, 25.0), 2.4, false);
            }
            let cover = if galician { Color32::from_rgb(0x1f, 0x4f, 0xa0) } else { Color32::from_rgb(0x22, 0x6e, 0x40) };
            pen.bag((46.0, 56.0), 22.0, 13.5, -0.3, cover);
            pen.blowpipe((34.0, 45.0), (13.0, 23.0));
            pen.chanter((33.0, 65.0), (23.0, 93.0), 3.6, 7, false);
        }
        7 => {
            // Breton biniou kozh: tiny bag, one short drone
            pen.drone((53.0, 45.0), (74.0, 24.0), 3.0, false);
            pen.bag((47.0, 55.0), 12.0, 8.0, -0.3, Color32::from_rgb(0x2a, 0x2a, 0x32));
            pen.blowpipe((40.0, 49.0), (24.0, 32.0));
            pen.chanter((40.0, 61.0), (35.0, 80.0), 3.0, 6, false);
        }
        9 => {
            // Medieval market pipes: hide bag, two drones in separate stocks, bells
            pen.drone((50.0, 41.0), (64.0, 14.0), 3.4, true);
            pen.drone((58.0, 45.0), (88.0, 6.0), 4.2, true);
            pen.bag((46.0, 56.0), 22.0, 14.0, -0.35, Color32::from_rgb(0x9a, 0x7a, 0x52));
            pen.blowpipe((34.0, 45.0), (13.0, 22.0));
            pen.chanter((33.0, 65.0), (23.0, 93.0), 3.8, 7, false);
        }
        10 => {
            // Hümmelchen: small bag, two short drones in one stock
            pen.drone((51.0, 45.0), (76.0, 26.0), 2.6, false);
            pen.drone((53.0, 48.0), (81.0, 34.0), 2.6, false);
            pen.bag((44.0, 56.0), 15.0, 10.0, -0.3, Color32::from_rgb(0x84, 0x64, 0x42));
            pen.blowpipe((36.0, 48.0), (19.0, 30.0));
            pen.chanter((35.0, 62.0), (29.0, 86.0), 3.0, 7, false);
        }
        _ => {
            // Bulgarian gaida: whole goatskin bag, one long drone hanging back, horn bell
            pen.drone((62.0, 56.0), (90.0, 90.0), 3.6, true);
            pen.bag((48.0, 52.0), 24.0, 17.0, -0.15, Color32::from_rgb(0x8a, 0x6a, 0x4a));
            pen.blowpipe((32.0, 42.0), (13.0, 20.0));
            pen.chanter((34.0, 65.0), (27.0, 90.0), 3.4, 8, false);
            pen.tube((27.0, 90.0), (25.0, 97.0), 4.0, 7.0, Color32::from_rgb(0xcf, 0xbf, 0x98));
        }
    }
}

// ---------------------------------------------------------------------------
// Shruti box: wooden case, inset panel, 13 flip levers (Sa … Sa').
// Lower row = the 8 natural notes, upper row = the 5 komal/tivra notes —
// laid out like a keyboard's white and black keys.

/// semitone (0..=12) for each lever, lower row first then upper row
pub const SHRUTI_LEVERS: [u8; 13] = [0, 2, 4, 5, 7, 9, 11, 12, 1, 3, 6, 8, 10];

/// Geometry of the 13 lever hit-areas inside the box drawn in `r`.
pub fn shruti_lever_rects(r: Rect) -> [Rect; 13] {
    let panel = shruti_panel(r);
    let mut out = [Rect::NOTHING; 13];
    let lw = panel.width() / 9.6;
    let lh = panel.height() * 0.40;
    for i in 0..8 {
        let cx = panel.left() + lw * (0.9 + i as f32 * 1.1);
        out[i] = Rect::from_center_size(Pos2::new(cx, panel.top() + panel.height() * 0.70), Vec2::new(lw * 0.8, lh));
    }
    // upper row sits between the lower levers, like black keys (no lever between Ga–Ma and Ni–Sa')
    let gaps = [0.5f32, 1.5, 3.5, 4.5, 5.5];
    for (j, g) in gaps.iter().enumerate() {
        let cx = panel.left() + lw * (0.9 + g * 1.1);
        out[8 + j] = Rect::from_center_size(Pos2::new(cx, panel.top() + panel.height() * 0.27), Vec2::new(lw * 0.8, lh));
    }
    out
}

fn shruti_panel(r: Rect) -> Rect {
    let inner = r.shrink2(Vec2::new(r.width() * 0.06, r.height() * 0.12));
    Rect::from_min_max(Pos2::new(inner.left() + inner.width() * 0.08, inner.top() + inner.height() * 0.12), Pos2::new(inner.right() - inner.width() * 0.08, inner.bottom() - inner.height() * 0.22))
}

/// Draw the shruti box. `on` = bit per semitone 0..=12; `labels[semi]` text under each lever.
pub fn draw_shruti(p: &Painter, r: Rect, on: u16, labels: &[String; 13], hover: Option<usize>, ink: Color32) {
    let case_col = Color32::from_rgb(0x8a, 0x55, 0x2e);
    let lid = Color32::from_rgb(0x74, 0x45, 0x24);
    // handle
    let hc = Pos2::new(r.center().x, r.top() + r.height() * 0.05);
    p.add(QuadraticBezierShape::from_points_stroke(
        [hc - Vec2::new(r.width() * 0.09, -r.height() * 0.04), hc - Vec2::new(0.0, r.height() * 0.06), hc + Vec2::new(r.width() * 0.09, r.height() * 0.04)],
        false,
        Color32::TRANSPARENT,
        Stroke::new(3.0, BRASS),
    ));
    let body = r.shrink2(Vec2::new(r.width() * 0.04, r.height() * 0.08));
    p.rect_filled(body, 6.0, case_col);
    let inner = body.shrink(body.width() * 0.025);
    p.rect_filled(inner, 3.0, lid);
    // wood grain
    for i in 0..7 {
        let y = inner.top() + inner.height() * (0.1 + i as f32 * 0.13);
        p.line_segment([Pos2::new(inner.left() + 4.0, y), Pos2::new(inner.right() - 4.0, y + (i % 3) as f32)], Stroke::new(0.8, Color32::from_black_alpha(28)));
    }
    let panel = shruti_panel(r);
    p.rect_filled(panel, 3.0, Color32::from_rgb(0x5e, 0x36, 0x1c));
    p.rect_stroke(panel, 3.0, Stroke::new(1.0, Color32::from_black_alpha(80)), StrokeKind::Inside);
    for (x, y) in [(0.03f32, 0.08f32), (0.97, 0.08), (0.03, 0.92), (0.97, 0.92), (0.5, 0.06), (0.5, 0.94)] {
        p.circle_filled(Pos2::new(panel.left() + panel.width() * x, panel.top() + panel.height() * y), 2.2, BRASS);
    }
    // bellows valve (brass quadrant), bottom right
    let q = Pos2::new(inner.right() - inner.width() * 0.07, inner.bottom() - inner.height() * 0.1);
    p.circle_filled(q, inner.height() * 0.1, Color32::from_rgb(0x9a, 0x7a, 0x3a));
    p.circle_filled(q, inner.height() * 0.035, Color32::from_rgb(0x30, 0x24, 0x10));

    let rects = shruti_lever_rects(r);
    for (i, lr) in rects.iter().enumerate() {
        let semi = SHRUTI_LEVERS[i] as usize;
        let is_on = on & (1 << semi) != 0;
        // slot
        p.rect_filled(Rect::from_center_size(lr.center(), Vec2::new(lr.width() * 0.22, lr.height() * 0.5)), 1.0, Color32::from_rgb(0x1e, 0x10, 0x08));
        // lever: upright when closed, swung sideways when open (that reed sounds)
        let pivot = Pos2::new(lr.center().x, lr.bottom() - lr.height() * 0.12);
        let ang: f32 = if is_on { -0.62 } else { 0.0 };
        let len = lr.height() * 0.86;
        let w = lr.width() * 0.62;
        let d = Vec2::new(ang.sin(), -ang.cos());
        let n = Vec2::new(-d.y, d.x);
        let tip = pivot + d * len;
        let col = if is_on { Color32::from_rgb(0xff, 0xf3, 0xd6) } else { IVORY.gamma_multiply(0.85) };
        p.add(Shape::convex_polygon(vec![pivot + n * w * 0.45, tip + n * w * 0.5, tip - n * w * 0.5, pivot - n * w * 0.45], col, Stroke::new(1.0, Color32::from_black_alpha(90))));
        p.circle_filled(tip, w * 0.5, col);
        p.circle_filled(pivot, w * 0.28, BRASS);
        if is_on {
            p.circle_filled(tip, w * 0.75, Color32::from_rgba_unmultiplied(0xff, 0xc8, 0x50, 40));
        }
        if hover == Some(i) {
            p.rect_stroke(lr.expand(2.0), 4.0, Stroke::new(1.5, Color32::from_rgb(0xff, 0xd2, 0x3d)), StrokeKind::Outside);
        }
        let ly = if i < 8 { r.bottom() - r.height() * 0.035 } else { panel.top() - r.height() * 0.04 };
        p.text(Pos2::new(lr.center().x, ly), Align2::CENTER_CENTER, &labels[semi], FontId::proportional((r.height() * 0.075).clamp(9.0, 13.0)), if is_on { Color32::from_rgb(0xff, 0xd2, 0x3d) } else { ink });
    }
}

// ---------------------------------------------------------------------------
// Harmonium: bellows lid at the back, keyboard (lights up what's sounding),
// and a row of pull-out stop knobs along the front.

pub const HARM_STOPS: [&str; 7] = ["Bass", "Male", "Female", "Sa", "Pa", "Ma", "Sa'"];
const HARM_LO: u8 = 48; // keyboard drawn from C3…C6

fn harm_front(r: Rect) -> Rect {
    Rect::from_min_max(Pos2::new(r.left() + r.width() * 0.04, r.top() + r.height() * 0.56), Pos2::new(r.right() - r.width() * 0.04, r.bottom() - r.height() * 0.11))
}

pub fn harmonium_stop_rects(r: Rect) -> [Rect; 7] {
    let f = harm_front(r);
    let mut out = [Rect::NOTHING; 7];
    for (i, o) in out.iter_mut().enumerate() {
        let cx = f.left() + f.width() * (0.09 + i as f32 * 0.137);
        *o = Rect::from_center_size(Pos2::new(cx, f.top() + f.height() * 0.34), Vec2::splat(f.height() * 0.50));
    }
    out
}

/// `stops` bit i = HARM_STOPS[i] pulled out; `lit[k]` = MIDI key k sounding.
pub fn draw_harmonium(p: &Painter, r: Rect, stops: u8, lit: &[bool; 128], hover: Option<usize>, ink: Color32) {
    let wood = Color32::from_rgb(0xc8, 0x8a, 0x2c);
    let wood_dk = Color32::from_rgb(0x9a, 0x62, 0x18);
    let base = Color32::from_rgb(0x6a, 0x1c, 0x1c);
    // plinth
    p.rect_filled(Rect::from_min_max(Pos2::new(r.left() + r.width() * 0.02, r.bottom() - r.height() * 0.12), Pos2::new(r.right() - r.width() * 0.02, r.bottom())), 3.0, base);
    // body
    let body = Rect::from_min_max(Pos2::new(r.left() + r.width() * 0.03, r.top() + r.height() * 0.16), Pos2::new(r.right() - r.width() * 0.03, r.bottom() - r.height() * 0.11));
    p.rect_filled(body, 4.0, wood);
    for i in 0..6 {
        let y = body.top() + body.height() * (0.12 + i as f32 * 0.16);
        p.line_segment([Pos2::new(body.left() + 3.0, y), Pos2::new(body.right() - 3.0, y)], Stroke::new(0.7, Color32::from_black_alpha(22)));
    }
    // bellows lid (back), with the dark bellows fold under it
    let lid = Rect::from_min_max(Pos2::new(r.left() + r.width() * 0.07, r.top()), Pos2::new(r.right() - r.width() * 0.07, r.top() + r.height() * 0.16));
    p.rect_filled(lid.translate(Vec2::new(0.0, 3.0)), 3.0, Color32::from_rgb(0x24, 0x1c, 0x40));
    p.rect_filled(lid, 3.0, wood_dk.lerp_to_gamma(wood, 0.5));
    // keyboard: 3 octaves C3…C6
    let kb = Rect::from_min_max(Pos2::new(body.left() + body.width() * 0.05, body.top() + body.height() * 0.06), Pos2::new(body.right() - body.width() * 0.05, body.top() + body.height() * 0.40));
    p.rect_filled(kb.expand(2.0), 2.0, Color32::from_rgb(0xb0, 0x20, 0x20));
    let whites: Vec<u8> = (HARM_LO..=HARM_LO + 36).filter(|k| !matches!(k % 12, 1 | 3 | 6 | 8 | 10)).collect();
    let ww = kb.width() / whites.len() as f32;
    let glow = Color32::from_rgb(0xff, 0xb3, 0x47);
    for (i, k) in whites.iter().enumerate() {
        let kr = Rect::from_min_size(Pos2::new(kb.left() + i as f32 * ww, kb.top()), Vec2::new(ww - 1.0, kb.height()));
        p.rect_filled(kr, 1.5, if lit[*k as usize] { glow } else { Color32::from_rgb(0xf4, 0xf1, 0xea) });
    }
    for (i, k) in whites.iter().enumerate() {
        let b = k + 1;
        if matches!(b % 12, 1 | 3 | 6 | 8 | 10) && b <= HARM_LO + 36 {
            let x = kb.left() + (i as f32 + 1.0) * ww;
            let br = Rect::from_center_size(Pos2::new(x, kb.top() + kb.height() * 0.31), Vec2::new(ww * 0.6, kb.height() * 0.62));
            p.rect_filled(br, 1.5, if lit[b as usize] { glow } else { Color32::from_rgb(0x1a, 0x1a, 0x1e) });
        }
    }
    // front panel with stops
    let f = harm_front(r);
    p.rect_filled(f, 3.0, wood.lerp_to_gamma(Color32::WHITE, 0.06));
    for (i, sr) in harmonium_stop_rects(r).iter().enumerate() {
        let on = stops & (1 << i) != 0;
        let c = sr.center();
        let rad = sr.width() * 0.32;
        // pulled stops stick out: bigger, brighter, with a shadow under them
        p.circle_filled(c, rad * 1.25, Color32::from_rgb(0xf2, 0xee, 0xe4)); // rosette
        if on {
            p.circle_filled(c + Vec2::new(1.5, 2.5), rad * 1.1, Color32::from_black_alpha(70));
            p.circle_filled(c, rad * 1.1, Color32::from_rgb(0xe8, 0xec, 0xf2));
            p.circle_filled(c - Vec2::new(rad * 0.3, rad * 0.3), rad * 0.35, Color32::WHITE);
            p.circle_stroke(c, rad * 1.1, Stroke::new(2.0, Color32::from_rgb(0xff, 0xd2, 0x3d)));
        } else {
            p.circle_filled(c, rad * 0.8, Color32::from_rgb(0xa8, 0xae, 0xb8));
            p.circle_filled(c - Vec2::new(rad * 0.2, rad * 0.2), rad * 0.25, Color32::from_rgb(0xe0, 0xe4, 0xea));
        }
        if hover == Some(i) {
            p.circle_stroke(c, rad * 1.45, Stroke::new(1.5, Color32::from_rgb(0xff, 0xd2, 0x3d)));
        }
        p.text(Pos2::new(c.x, f.bottom() - f.height() * 0.02), Align2::CENTER_BOTTOM, HARM_STOPS[i], FontId::proportional((r.height() * 0.06).clamp(8.5, 12.0)), if on { Color32::from_rgb(0x2a, 0x14, 0x00) } else { ink });
    }
}

// ---------------------------------------------------------------------------
// Fingering view: one instrument body, holes filled where a finger goes down.

/// Draw `chart` with `states` (None = note not on this instrument). `ghost` = faded (the "next" note).
pub fn draw_fingering(p: &Painter, r: Rect, chart: &crate::fingering::Chart, states: Option<&[u8]>, label: &str, ghost: bool, ink: Color32) {
    use crate::fingering::Hole;
    let a = if ghost { 0.45 } else { 1.0 };
    let body_w = (r.width() * 0.30).min(34.0);
    let top = r.top() + 22.0;
    let bot = r.bottom() - 10.0;
    let cx = r.center().x + body_w * 0.25;
    // body
    let body = Rect::from_min_max(Pos2::new(cx - body_w / 2.0, top), Pos2::new(cx + body_w / 2.0, bot));
    p.rect_filled(body, body_w * 0.35, WOOD.gamma_multiply(a));
    p.rect_filled(Rect::from_min_size(body.min - Vec2::new(2.0, 4.0), Vec2::new(body_w + 4.0, 8.0)), 3.0, IVORY.gamma_multiply(a));
    p.text(Pos2::new(r.center().x, r.top() + 2.0), Align2::CENTER_TOP, label, FontId::proportional(15.0), ink.gamma_multiply(a));
    // hole positions
    let n = chart.holes.len();
    let ends = chart.holes.iter().filter(|h| matches!(h.0, Hole::End | Hole::Key)).count();
    let rows = n - ends;
    let gap_extra = chart.breaks.len() as f32 * 0.6;
    let step = (bot - top - 26.0) / (rows as f32 + gap_extra);
    let rad = (step * 0.36).min(body_w * 0.36).max(3.0);
    let closed = Color32::from_rgb(0x15, 0x10, 0x0c).gamma_multiply(a);
    let open_fill = Color32::from_rgb(0xf6, 0xef, 0xdc).gamma_multiply(a);
    let finger = Color32::from_rgb(0xff, 0x8a, 0x3d).gamma_multiply(a);
    let mut y = top + 16.0;
    let mut row = 0usize;
    for (i, (kind, _name)) in chart.holes.iter().enumerate() {
        let st = states.and_then(|s| s.get(i)).copied();
        if matches!(kind, Hole::End | Hole::Key) {
            continue;
        }
        if chart.breaks.contains(&i) {
            y += step * 0.6;
        }
        let c = match kind {
            Hole::Thumb => Pos2::new(cx - body_w * 0.5 - rad * 1.6, y),
            _ => Pos2::new(cx, y),
        };
        let draw_hole = |c: Pos2, rr: f32, s: Option<u8>| {
            match s {
                Some(1) => {
                    p.circle_filled(c, rr * 1.15, finger);
                    p.circle_filled(c, rr * 0.8, closed);
                }
                Some(2) => {
                    p.circle_filled(c, rr, open_fill);
                    // half: left half covered
                    let pts: Vec<Pos2> = (0..=16).map(|k| {
                        let t = std::f32::consts::FRAC_PI_2 + k as f32 / 16.0 * std::f32::consts::PI;
                        c + Vec2::new(t.cos(), t.sin()) * rr * 1.1
                    }).collect();
                    p.add(Shape::convex_polygon(pts, finger, Stroke::NONE));
                }
                Some(_) => {
                    p.circle_filled(c, rr, open_fill);
                    p.circle_stroke(c, rr, Stroke::new(1.0, closed));
                }
                None => {
                    p.circle_filled(c, rr, Color32::from_gray(90).gamma_multiply(a));
                }
            }
        };
        match kind {
            Hole::Double => {
                draw_hole(c - Vec2::new(rad * 0.55, 0.0), rad * 0.55, st.map(|s| if s == 2 { 1 } else { s }));
                draw_hole(c + Vec2::new(rad * 0.55, 0.0), rad * 0.55, st.map(|s| if s == 2 { 0 } else { s }));
            }
            Hole::Thumb => {
                draw_hole(c, rad, st);
                p.text(c - Vec2::new(rad + 2.0, 0.0), Align2::RIGHT_CENTER, "T", FontId::proportional(9.5), ink.gamma_multiply(0.6 * a));
            }
            _ => draw_hole(c, rad, st),
        }
        y += step;
        row += 1;
    }
    let _ = row;
    // bottom end / key
    for (i, (kind, name)) in chart.holes.iter().enumerate() {
        let st = states.and_then(|s| s.get(i)).copied();
        match kind {
            Hole::End => {
                let c = Pos2::new(cx, bot + 2.0);
                let on = st == Some(1);
                p.add(Shape::convex_polygon(
                    (0..20).map(|k| {
                        let t = k as f32 / 20.0 * std::f32::consts::TAU;
                        c + Vec2::new(t.cos() * body_w * 0.55, t.sin() * 4.0)
                    }).collect(),
                    if on { finger } else { open_fill },
                    Stroke::new(1.0, closed),
                ));
                if on {
                    p.text(c + Vec2::new(body_w * 0.6 + 3.0, 0.0), Align2::LEFT_CENTER, if name.contains('×') { "×" } else { "knee" }, FontId::proportional(10.0), finger);
                }
            }
            Hole::Key => {
                let kc = Pos2::new(cx + body_w * 0.5 + 7.0, top + 16.0 + step * 1.6);
                let on = st == Some(1);
                p.rect_filled(Rect::from_center_size(kc, Vec2::new(8.0, 5.0)), 2.0, if on { finger } else { IVORY.gamma_multiply(a) });
                if on {
                    p.text(kc + Vec2::new(6.0, 0.0), Align2::LEFT_CENTER, *name, FontId::proportional(9.5), finger);
                }
            }
            _ => {}
        }
    }
    if states.is_none() {
        p.text(Pos2::new(cx, (top + bot) / 2.0), Align2::CENTER_CENTER, "—", FontId::proportional(20.0), ink.gamma_multiply(0.5));
    }
}
