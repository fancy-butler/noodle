// Noodle — Accordion mode: the whole window becomes a button accordion.
// Computer keys are the buttons, Space is the bellows, F-keys the registers.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// A child module of `ui`, so it can use App's fields and the ui helpers.

use super::*;
use crate::accordion::{self as acc, Bass, Instrument, Kind};
use crate::engine::AccMsg;

pub struct AccState {
    pub insts: Vec<Instrument>,
    /// computer key → (engine id, what it plays) while held
    down: HashMap<Key, Held>,
    mouse: Option<Held>,
    pub push: bool,
    /// bellows opening for the picture, 0 closed … 1 open
    ext: f32,
    /// Stradella window centre (pitch class)
    pub center: u8,
}

#[derive(Clone, Copy, PartialEq)]
enum Held {
    Treble { r: usize, i: usize },
    DiaBass { r: usize, i: usize },
    Strad { col: u8, row: usize },
}

impl AccState {
    pub fn new() -> Self {
        AccState { insts: acc::instruments(), down: HashMap::new(), mouse: None, push: false, ext: 0.5, center: 0 }
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

const PUSH_COL: Color32 = Color32::from_rgb(0xff, 0x9f, 0x1c);
const PULL_COL: Color32 = Color32::from_rgb(0x2e, 0xc4, 0xb6);

/// readable text on a case colour
fn ink_on(c: [u8; 3], alpha: u8) -> Color32 {
    let l = 0.3 * c[0] as f32 + 0.59 * c[1] as f32 + 0.11 * c[2] as f32;
    if l > 140.0 {
        Color32::from_black_alpha(alpha)
    } else {
        Color32::from_white_alpha(alpha)
    }
}

/// Sharps, except on rows in flat keys (the row's home note is F, B♭, E♭ or A♭).
fn spell(n: u8, row_home: u8) -> &'static str {
    if matches!(row_home % 12, 5 | 10 | 3 | 8) {
        acc::NAMES_FLAT[(n % 12) as usize]
    } else {
        acc::NAMES_SHARP[(n % 12) as usize]
    }
}

fn is_black(n: u8) -> bool {
    matches!(n % 12, 1 | 3 | 6 | 8 | 10)
}

/// x offset (in key widths) of each computer-key row, like a real keyboard
fn row_x0(kind: Kind, r: usize) -> f32 {
    match kind {
        Kind::Diatonic => [0.75, 1.5, 1.0][r.min(2)],
        Kind::Chromatic => [1.25, 0.75, 0.5, 0.0, -0.5][r.min(4)],
    }
}

impl App {
    fn acc_inst(&self) -> &Instrument {
        let a = self.acc.as_ref().unwrap();
        &a.insts[self.s.acc_inst.min(a.insts.len() - 1)]
    }

    pub(super) fn enter_accordion(&mut self) {
        if self.acc.is_none() {
            self.acc = Some(AccState::new());
        }
        self.acc_mode = true;
        if self.game_mode {
            self.exit_game();
        }
        self.acc_setup();
    }

    pub(super) fn exit_accordion(&mut self) {
        self.acc_release_all();
        self.acc_mode = false;
        let _ = self.tx.send(Cmd::Acc(AccMsg::Setup(None)));
    }

    fn acc_setup(&mut self) {
        let (reg, bass, tuning) = {
            let i = self.acc_inst();
            (i.registers[self.s.acc_reg.min(i.registers.len() - 1)], i.bass_ranks, i.tuning)
        };
        self.acc_release_all();
        let _ = self.tx.send(Cmd::Acc(AccMsg::Setup(Some((reg, bass, tuning)))));
        let push = self.acc.as_ref().map(|a| a.push).unwrap_or(false);
        let _ = self.tx.send(Cmd::Acc(AccMsg::Bellows(push)));
    }

    fn acc_release_all(&mut self) {
        let Some(a) = self.acc.as_mut() else { return };
        let held: Vec<Held> = a.down.drain().map(|(_, h)| h).chain(a.mouse.take()).collect();
        for h in held {
            self.acc_play(h, false);
        }
    }

    fn held_id(h: Held) -> u16 {
        match h {
            Held::Treble { r, i } => 1 + (r * 40 + i) as u16,
            Held::DiaBass { r, i } => 300 + (r * 20 + i) as u16,
            Held::Strad { col, row } => 500 + (col as u16) * 8 + row as u16,
        }
    }

    /// Start or stop what a button plays, in the current bellows direction.
    fn acc_play(&mut self, h: Held, on: bool) {
        let push = self.acc.as_ref().map(|a| a.push).unwrap_or(false);
        let id = Self::held_id(h);
        let msg = match h {
            Held::Treble { r, i } => {
                let Some(b) = self.acc_inst().rows.get(r).and_then(|row| row.get(i)).copied() else { return };
                AccMsg::Treble { id, note: if push { b.push } else { b.pull }, on }
            }
            Held::DiaBass { r, i } => {
                let Bass::Diatonic(rows) = &self.acc_inst().bass else { return };
                let Some(&(p, q)) = rows.get(r).and_then(|x| x.get(i)) else { return };
                let (notes, chord) = acc::bass_notes(if push { p } else { q });
                let mut n3 = [0u8; 3];
                for (k, n) in notes.iter().take(3).enumerate() {
                    n3[k] = *n;
                }
                AccMsg::Bass { id, notes: n3, n: notes.len().min(3) as u8, chord, on }
            }
            Held::Strad { col, row } => {
                let (notes, chord) = acc::stradella_notes(col, row);
                let mut n3 = [0u8; 3];
                for (k, n) in notes.iter().take(3).enumerate() {
                    n3[k] = *n;
                }
                AccMsg::Bass { id, notes: n3, n: notes.len().min(3) as u8, chord, on }
            }
        };
        let _ = self.tx.send(Cmd::Acc(msg));
    }

    fn acc_bellows(&mut self, push: bool) {
        let Some(a) = self.acc.as_mut() else { return };
        if a.push == push {
            return;
        }
        a.push = push;
        let _ = self.tx.send(Cmd::Acc(AccMsg::Bellows(push)));
        // bisonoric buttons change note with the bellows
        let diatonic = self.acc_inst().kind == Kind::Diatonic;
        let a = self.acc.as_ref().unwrap();
        let held: Vec<Held> = a.down.values().copied().chain(a.mouse).collect();
        for h in held {
            let flips = match h {
                Held::Treble { .. } => diatonic,
                Held::DiaBass { .. } => true,
                Held::Strad { .. } => false,
            };
            if flips {
                self.acc_play(h, false);
                self.acc_play(h, true);
            }
        }
    }

    fn acc_key_to_held(&self, key: Key, m: egui::Modifiers) -> Option<Held> {
        let inst = self.acc_inst();
        for (r, row) in inst.rows.iter().enumerate() {
            for i in 0..row.len() {
                if acc::treble_key(inst, r, i) == Some(key) {
                    return Some(Held::Treble { r, i });
                }
            }
        }
        let bk = acc::bass_keys(inst);
        let k = bk.iter().position(|x| *x == key)?;
        match &inst.bass {
            Bass::Diatonic(rows) => {
                // first bass row on the first keys, second row after it
                let mut k2 = k;
                for (r, row) in rows.iter().enumerate() {
                    if k2 < row.len() {
                        return Some(Held::DiaBass { r, i: k2 });
                    }
                    k2 -= row.len();
                }
                None
            }
            Bass::Stradella => {
                // pairs of keys per column: bass, chord. Shift = counterbass / minor, Ctrl = 7th, both = dim
                let cols = acc::fifths(self.acc.as_ref()?.center, bk.len() / 2);
                let col = *cols.get(k / 2)?;
                let row = if k % 2 == 0 {
                    if m.shift { 0 } else { 1 }
                } else {
                    match (m.shift, m.ctrl || m.command) {
                        (true, true) => 5,
                        (false, true) => 4,
                        (true, false) => 3,
                        _ => 2,
                    }
                };
                Some(Held::Strad { col, row })
            }
        }
    }

    /// Keyboard in accordion mode. Returns true when it handled the input.
    pub(super) fn acc_keys(&mut self, ctx: &egui::Context) {
        if !ctx.input(|i| i.focused) {
            self.acc_release_all();
            return;
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            let egui::Event::Key { key, pressed, repeat, modifiers, .. } = ev else { continue };
            if repeat {
                continue;
            }
            if key == Key::Space {
                let push = pressed != self.s.acc_space_pull;
                self.acc_bellows(push);
                continue;
            }
            if pressed {
                let fkeys = [Key::F1, Key::F2, Key::F3, Key::F4, Key::F5, Key::F6, Key::F7, Key::F8, Key::F9, Key::F10, Key::F11, Key::F12];
                if let Some(n) = fkeys.iter().position(|k| *k == key) {
                    if n < self.acc_inst().registers.len() {
                        self.s.acc_reg = n;
                        let r = self.acc_inst().registers[n];
                        let _ = self.tx.send(Cmd::Acc(AccMsg::Register(r)));
                    }
                    continue;
                }
                match key {
                    Key::Escape => {
                        self.exit_accordion();
                        return;
                    }
                    Key::ArrowLeft | Key::ArrowRight => {
                        let step = if key == Key::ArrowLeft { 5 } else { 7 };
                        if let Some(a) = self.acc.as_mut() {
                            a.center = (a.center + step) % 12;
                        }
                        continue;
                    }
                    Key::PageUp | Key::PageDown => {
                        let n = self.acc.as_ref().map(|a| a.insts.len()).unwrap_or(1);
                        self.s.acc_inst = if key == Key::PageDown { (self.s.acc_inst + 1) % n } else { (self.s.acc_inst + n - 1) % n };
                        self.s.acc_reg = 0;
                        self.acc_setup();
                        continue;
                    }
                    _ => {}
                }
                if self.acc.as_ref().map(|a| a.down.contains_key(&key)).unwrap_or(true) {
                    continue;
                }
                if let Some(h) = self.acc_key_to_held(key, modifiers) {
                    self.acc.as_mut().unwrap().down.insert(key, h);
                    self.acc_play(h, true);
                }
            } else if let Some(h) = self.acc.as_mut().and_then(|a| a.down.remove(&key)) {
                self.acc_play(h, false);
            }
        }
    }

    /// Notes to show as hints: what's sounding in the song (melody) and what comes next.
    fn acc_hints(&self) -> (Vec<u8>, Option<u8>) {
        let Some(song) = self.song.as_ref() else { return (Vec::new(), None) };
        let pos = self.p.song_pos.get();
        let lead = (0..16u8).find(|c| song.notes_per_ch[*c as usize] > 0 && *c != 9).unwrap_or(0);
        let now: Vec<u8> = song.notes.iter().filter(|n| n.ch == lead && n.start <= pos + 0.02 && n.end > pos + 0.02).map(|n| n.key).collect();
        let next = song.notes.iter().filter(|n| n.ch == lead && n.start > pos + 0.03).map(|n| n.key).next();
        (now, next)
    }

    pub(super) fn acc_screen(&mut self, ui: &mut egui::Ui, rect: Rect) {
        if self.acc.is_none() {
            self.acc = Some(AccState::new());
        }
        let now = now_s(&self.t0) as f32;
        let _ = now;
        // ---- top bar: instrument, registers, bellows ----
        let mut new_inst: Option<usize> = None;
        let mut new_reg: Option<usize> = None;
        let mut leave = false;
        let top = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 74.0));
        let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::top_down(egui::Align::Min)));
        {
            let ui = &mut bar;
            let (cur_name, verified, source, blurb, regs, nint) = {
                let i = self.acc_inst();
                (i.name, i.verified, i.source, i.blurb, i.registers, self.acc.as_ref().unwrap().insts.len())
            };
            ui.horizontal(|ui| {
                ui.label(RichText::new("ACCORDION").size(13.0).strong().color(YOURS));
                egui::ComboBox::from_id_salt("acc_inst").selected_text(RichText::new(cur_name).size(16.0).strong()).width(250.0).show_ui(ui, |ui| {
                    let a = self.acc.as_ref().unwrap();
                    let mut fam = "";
                    for (k, inst) in a.insts.iter().enumerate() {
                        if inst.family != fam {
                            fam = inst.family;
                            ui.label(RichText::new(fam).size(11.0).color(text_dim()));
                        }
                        if ui.selectable_label(k == self.s.acc_inst.min(nint - 1), inst.name).clicked() {
                            new_inst = Some(k);
                        }
                    }
                });
                if !verified {
                    ui.label(RichText::new("⚠ layout unverified").color(Color32::from_rgb(0xff, 0xb0, 0x40))).on_hover_text(source);
                } else {
                    ui.label(RichText::new("✔ layout sourced").size(11.0).color(GOOD)).on_hover_text(format!("Layout: {source}"));
                }
                ui.label(RichText::new(blurb).size(12.0).color(text_dim()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("✕ Leave accordion (Esc)").clicked() {
                        leave = true;
                    }
                    ui.toggle_value(&mut self.s.acc_space_pull, "Space = pull").on_hover_text("Off: hold Space to push (close) the bellows, let go to pull. On: the other way round.");
                    ui.toggle_value(&mut self.s.acc_names, "Note names").on_hover_text("Note names on the buttons (off: just the computer keys)");
                });
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Registers").size(11.0).color(text_dim()));
                for (n, r) in regs.iter().enumerate() {
                    let on = n == self.s.acc_reg.min(regs.len() - 1);
                    let (resp, p) = ui.allocate_painter(Vec2::new(118.0, 34.0), Sense::click());
                    let rr = resp.rect;
                    p.rect_filled(rr, 8.0, if on { Color32::from_rgb(0xf4, 0xee, 0xe0) } else { card_hi() });
                    // the classic register symbol: a circle with a dot per reed bank
                    let c = Pos2::new(rr.left() + 18.0, rr.center().y);
                    let ink = if on { Color32::BLACK } else { fg() };
                    p.circle_stroke(c, 12.0, Stroke::new(1.5, ink));
                    p.line_segment([c - Vec2::new(12.0, 4.0), c + Vec2::new(12.0, -4.0)], Stroke::new(0.8, ink));
                    p.line_segment([c - Vec2::new(12.0, -4.0), c + Vec2::new(12.0, 4.0)], Stroke::new(0.8, ink));
                    for rank in r.ranks {
                        let d = match rank {
                            crate::reed::Rank::H => Vec2::new(0.0, -8.0),
                            crate::reed::Rank::M => Vec2::new(0.0, 0.0),
                            crate::reed::Rank::MFlat => Vec2::new(-6.0, 0.0),
                            crate::reed::Rank::MSharp => Vec2::new(6.0, 0.0),
                            crate::reed::Rank::L => Vec2::new(0.0, 8.0),
                        };
                        p.circle_filled(c + d, 2.6, ink);
                    }
                    let name = r.name.split(" · ").next().unwrap_or(r.name);
                    p.text(Pos2::new(rr.left() + 36.0, rr.center().y - 6.0), egui::Align2::LEFT_CENTER, name, egui::FontId::proportional(12.0), ink);
                    p.text(Pos2::new(rr.left() + 36.0, rr.center().y + 8.0), egui::Align2::LEFT_CENTER, format!("F{}", n + 1), egui::FontId::proportional(10.0), if on { Color32::DARK_GRAY } else { text_dim() });
                    if resp.on_hover_text(r.name).clicked() {
                        new_reg = Some(n);
                    }
                }
            });
        }
        if leave {
            self.exit_accordion();
            return;
        }
        if let Some(k) = new_inst {
            self.s.acc_inst = k;
            self.s.acc_reg = 0;
            self.acc_setup();
        }
        if let Some(n) = new_reg {
            self.s.acc_reg = n;
            let r = self.acc_inst().registers[n];
            let _ = self.tx.send(Cmd::Acc(AccMsg::Register(r)));
        }

        // ---- the instrument ----
        let body = Rect::from_min_max(Pos2::new(rect.left(), top.bottom() + 6.0), Pos2::new(rect.right(), rect.bottom() - 30.0));
        let p = ui.painter_at(rect);
        let look = self.acc_inst().look;
        let (push, ext_now) = {
            let a = self.acc.as_mut().unwrap();
            // bellows picture: opens while pulling, closes while pushing
            let any = !a.down.is_empty() || a.mouse.is_some();
            if any {
                a.ext = (a.ext + if a.push { -0.006 } else { 0.006 }).clamp(0.05, 1.0);
            }
            (a.push, a.ext)
        };
        let strad = matches!(self.acc_inst().bass, Bass::Stradella);
        let bass_w = if strad { (body.width() * 0.3).clamp(260.0, 420.0) } else { (body.width() * 0.22).clamp(200.0, 300.0) };
        let bel_w = 50.0 + 90.0 * ext_now;
        let bass_r = Rect::from_min_size(body.min, Vec2::new(bass_w, body.height()));
        let bel_r = Rect::from_min_size(Pos2::new(bass_r.right(), body.top() + 10.0), Vec2::new(bel_w, body.height() - 20.0));
        let treb_r = Rect::from_min_max(Pos2::new(bel_r.right(), body.top()), body.max);
        // cases
        for r in [bass_r, treb_r] {
            Self::gradient(&p, r, rgb(look.body), rgb(look.body2));
            p.rect_stroke(r, 10.0, Stroke::new(2.0, rgb(look.accent)), egui::StrokeKind::Inside);
        }
        // bellows folds
        let folds = 9;
        p.rect_filled(bel_r, 2.0, rgb(look.bellows));
        let dir_col = if push { PUSH_COL } else { PULL_COL };
        for k in 0..=folds {
            let x = bel_r.left() + bel_r.width() * k as f32 / folds as f32;
            p.line_segment([Pos2::new(x, bel_r.top()), Pos2::new(x, bel_r.bottom())], Stroke::new(if k % 2 == 0 { 2.0 } else { 1.0 }, mix(rgb(look.bellows), Color32::WHITE, if k % 2 == 0 { 0.35 } else { 0.12 })));
        }
        for y in [bel_r.top() + 4.0, bel_r.bottom() - 4.0] {
            p.line_segment([Pos2::new(bel_r.left(), y), Pos2::new(bel_r.right(), y)], Stroke::new(3.0, rgb(look.accent)));
        }
        p.rect_filled(Rect::from_center_size(bel_r.center() - Vec2::new(0.0, 3.0), Vec2::new(54.0, 46.0)), 8.0, Color32::from_black_alpha(200));
        p.text(bel_r.center() - Vec2::new(0.0, 14.0), egui::Align2::CENTER_CENTER, if push { "PUSH" } else { "PULL" }, egui::FontId::proportional(15.0), dir_col);
        p.text(bel_r.center() + Vec2::new(0.0, 8.0), egui::Align2::CENTER_CENTER, if push { "▶ ◀" } else { "◀ ▶" }, egui::FontId::proportional(16.0), dir_col);
        p.text(Pos2::new(bel_r.center().x, bel_r.bottom() + 14.0), egui::Align2::CENTER_CENTER, "Space", egui::FontId::proportional(11.0), text_dim());

        let (hint_now, hint_next) = self.acc_hints();
        let live: Vec<u8> = (0..128u8).filter(|k| self.held[0][*k as usize] > 0).collect();
        let mut press: Option<Held> = None;
        let mut released = false;

        // ---- treble ----
        {
            let inst = self.acc_inst();
            let kind = inst.kind;
            let bisonoric = kind == Kind::Diatonic;
            let shown_rows = inst.rows.len();
            let max_x = (0..shown_rows).map(|r| row_x0(kind, r) + inst.rows[r].len() as f32).fold(0.0f32, f32::max);
            let min_x = (0..shown_rows).map(|r| row_x0(kind, r)).fold(f32::MAX, f32::min);
            let inner = treb_r.shrink2(Vec2::new(22.0, 26.0));
            let cell = (inner.width() / (max_x - min_x + 0.2)).min(inner.height() / (shown_rows as f32 + 0.4)).min(84.0);
            let rad = cell * 0.43;
            let grid_h = cell * shown_rows as f32;
            let y0 = inner.center().y + grid_h / 2.0 - cell / 2.0;
            let x0 = inner.left() + (inner.width() - (max_x - min_x) * cell) / 2.0 - min_x * cell;
            // grille panel behind the buttons
            p.rect_filled(Rect::from_min_max(Pos2::new(x0 + (min_x - 0.2) * cell, y0 - grid_h + cell * 0.3), Pos2::new(x0 + (max_x + 0.0) * cell, y0 + cell * 0.7)), 12.0, rgb(look.grille).linear_multiply(0.35));
            let a = self.acc.as_ref().unwrap();
            let held_t: Vec<(usize, usize)> = a.down.values().chain(a.mouse.iter()).filter_map(|h| if let Held::Treble { r, i } = h { Some((*r, *i)) } else { None }).collect();
            let pointer = ui.input(|i| i.pointer.interact_pos());
            let down = ui.input(|i| i.pointer.primary_down());
            for (r, row) in inst.rows.iter().enumerate() {
                let dup = kind == Kind::Chromatic && r >= inst.rows.len() - inst.dup_rows;
                let home = if bisonoric { row.get(2).map(|b| b.push).unwrap_or(0) } else { 0 };
                for (i, b) in row.iter().enumerate() {
                    let c = Pos2::new(x0 + (row_x0(kind, r) + i as f32 + 0.5) * cell, y0 - r as f32 * cell);
                    let held = held_t.contains(&(r, i));
                    let base = if !bisonoric && is_black(b.push) { Color32::from_rgb(0x22, 0x22, 0x26) } else { rgb(look.button) };
                    let base = if dup { base.linear_multiply(0.55) } else { base };
                    p.circle_filled(c + Vec2::new(0.0, 2.5), rad, Color32::from_black_alpha(90));
                    p.circle_filled(c, rad, if held { dir_col } else { base });
                    p.circle_stroke(c, rad, Stroke::new(1.0, Color32::from_black_alpha(120)));
                    let ink = if held { Color32::BLACK } else if !bisonoric && is_black(b.push) { Color32::WHITE } else { Color32::from_rgb(0x20, 0x20, 0x20) };
                    if bisonoric {
                        // push note on top, pull note below; the live direction is bold
                        p.line_segment([c - Vec2::new(rad * 0.75, 0.0), c + Vec2::new(rad * 0.75, 0.0)], Stroke::new(1.0, ink.linear_multiply(0.4)));
                        let (pa, pb) = (if push { 1.0 } else { 0.45 }, if push { 0.45 } else { 1.0 });
                        if self.s.acc_names {
                            p.text(c - Vec2::new(0.0, rad * 0.42), egui::Align2::CENTER_CENTER, spell(b.push, home), egui::FontId::proportional(rad * 0.52), ink.linear_multiply(pa));
                            p.text(c + Vec2::new(0.0, rad * 0.42), egui::Align2::CENTER_CENTER, spell(b.pull, home), egui::FontId::proportional(rad * 0.52), ink.linear_multiply(pb));
                        }
                    } else if self.s.acc_names {
                        p.text(c, egui::Align2::CENTER_CENTER, spell(b.push, 0), egui::FontId::proportional(rad * 0.62), ink);
                    }
                    if b.push % 12 == 0 && !bisonoric {
                        // C buttons get a ring, like the dimpled buttons on a real box
                        p.circle_stroke(c, rad * 0.82, Stroke::new(1.5, Color32::from_rgb(0xc0, 0x40, 0x40)));
                    }
                    if let Some(k) = acc::treble_key(inst, r, i) {
                        p.text(c + Vec2::new(rad * 0.8, -rad * 0.8), egui::Align2::CENTER_CENTER, acc::key_label(k), egui::FontId::monospace(11.0), ink_on(look.grille, 230));
                    }
                    // hints: song note now (solid) and next (dashed), live MIDI notes
                    let wants = |n: u8| -> Option<bool> {
                        if b.push == n {
                            Some(true)
                        } else if bisonoric && b.pull == n {
                            Some(false)
                        } else {
                            None
                        }
                    };
                    for &n in hint_now.iter().chain(live.iter()) {
                        if let Some(d) = wants(n) {
                            p.circle_stroke(c, rad + 3.5, Stroke::new(3.0, if d { PUSH_COL } else { PULL_COL }));
                        }
                    }
                    if let Some(n) = hint_next {
                        if let Some(d) = wants(n) {
                            let col = if d { PUSH_COL } else { PULL_COL };
                            for s in 0..12 {
                                let a0 = s as f32 / 12.0 * std::f32::consts::TAU;
                                let a1 = a0 + 0.3;
                                let rr = rad + 6.0;
                                p.line_segment([c + Vec2::angled(a0) * rr, c + Vec2::angled(a1) * rr], Stroke::new(2.0, col.linear_multiply(0.8)));
                            }
                        }
                    }
                    if let Some(pp) = pointer {
                        if down && (pp - c).length() < rad && !dup {
                            press = Some(Held::Treble { r, i });
                        }
                    }
                }
            }
            if !down {
                released = true;
            }
            p.text(Pos2::new(treb_r.left() + 14.0, treb_r.bottom() - 14.0), egui::Align2::LEFT_CENTER, if bisonoric { "Top of each button = push note · bottom = pull note" } else { "Same note both ways · rows of minor thirds, semitones on the diagonal" }, egui::FontId::proportional(11.0), ink_on(look.body2, 190));
        }

        // ---- bass ----
        {
            let inst = self.acc_inst();
            let a = self.acc.as_ref().unwrap();
            let held_b: Vec<Held> = a.down.values().copied().chain(a.mouse).filter(|h| !matches!(h, Held::Treble { .. })).collect();
            let inner = bass_r.shrink2(Vec2::new(16.0, 24.0));
            let pointer = ui.input(|i| i.pointer.interact_pos());
            let down = ui.input(|i| i.pointer.primary_down());
            let bk = acc::bass_keys(inst);
            match &inst.bass {
                Bass::Diatonic(rows) => {
                    let cols = rows.iter().map(|r| r.len()).max().unwrap_or(1);
                    let cell = (inner.width() / (rows.len() as f32 + 0.6)).min(inner.height() / cols as f32).min(70.0);
                    let rad = cell * 0.42;
                    let mut kpos = 0;
                    for (r, row) in rows.iter().enumerate() {
                        for (i, &(pu, pl)) in row.iter().enumerate() {
                            let c = Pos2::new(inner.center().x + (r as f32 - (rows.len() as f32 - 1.0) / 2.0) * cell + if r % 2 == 1 { 0.0 } else { 0.0 }, inner.top() + (i as f32 + 0.5 + r as f32 * 0.5) * cell);
                            let held = held_b.contains(&Held::DiaBass { r, i });
                            let lab = |s: acc::BassSnd| match s {
                                acc::BassSnd::Note(pc) => acc::NAMES[pc as usize].to_string(),
                                acc::BassSnd::Major(pc) => format!("{}+", acc::NAMES[pc as usize]),
                                acc::BassSnd::Minor(pc) => format!("{}m", acc::NAMES[pc as usize]),
                            };
                            p.circle_filled(c + Vec2::new(0.0, 2.0), rad, Color32::from_black_alpha(90));
                            p.circle_filled(c, rad, if held { dir_col } else { rgb(look.button) });
                            let ink = Color32::from_rgb(0x20, 0x20, 0x20);
                            p.text(c - Vec2::new(0.0, rad * 0.4), egui::Align2::CENTER_CENTER, lab(pu), egui::FontId::proportional(rad * 0.5), ink.linear_multiply(if push { 1.0 } else { 0.45 }));
                            p.text(c + Vec2::new(0.0, rad * 0.4), egui::Align2::CENTER_CENTER, lab(pl), egui::FontId::proportional(rad * 0.5), ink.linear_multiply(if push { 0.45 } else { 1.0 }));
                            if let Some(k) = bk.get(kpos) {
                                p.text(c + Vec2::new(-rad * 0.95, -rad * 0.8), egui::Align2::CENTER_CENTER, acc::key_label(*k), egui::FontId::monospace(11.0), ink_on(look.body, 220));
                            }
                            kpos += 1;
                            if let Some(pp) = pointer {
                                if down && (pp - c).length() < rad {
                                    press = Some(Held::DiaBass { r, i });
                                }
                            }
                        }
                    }
                }
                Bass::Stradella => {
                    let ncol = (bk.len() / 2).max(5);
                    let cols = acc::fifths(a.center, ncol);
                    let cell = ((inner.width() - 40.0) / (6.0 + 0.3 * ncol as f32)).min(inner.height() / (ncol as f32 + 1.6)).min(64.0);
                    let rad = cell * 0.4;
                    for (row, name) in ["Cb", "Bass", "Maj", "Min", "7th", "Dim"].iter().enumerate() {
                        p.text(Pos2::new(inner.left() + (row as f32 + 0.5) * cell, inner.top() + 0.5 * cell), egui::Align2::CENTER_CENTER, *name, egui::FontId::proportional(10.5), ink_on(look.body, 200));
                    }
                    for (ci, &col) in cols.iter().enumerate() {
                        for row in 0..6 {
                            let c = Pos2::new(inner.left() + (row as f32 + 0.5) * cell + ci as f32 * cell * 0.3, inner.top() + (ci as f32 + 1.4) * cell);
                            let held = held_b.contains(&Held::Strad { col, row });
                            let fill = if held {
                                dir_col
                            } else if row == 1 {
                                rgb(look.button)
                            } else {
                                rgb(look.button).linear_multiply(0.8)
                            };
                            p.circle_filled(c, rad, fill);
                            if row == 1 && col == 0 {
                                p.circle_stroke(c, rad * 0.7, Stroke::new(1.5, Color32::from_rgb(0xc0, 0x40, 0x40)));
                            }
                            p.text(c, egui::Align2::CENTER_CENTER, acc::stradella_label(col, row), if row == 1 { egui::FontId::proportional(rad * 0.7) } else { egui::FontId::proportional(rad * 0.55) }, Color32::from_rgb(0x20, 0x20, 0x20));
                            if let Some(pp) = pointer {
                                if down && (pp - c).length() < rad {
                                    press = Some(Held::Strad { col, row });
                                }
                            }
                        }
                        // key labels: bass key, chord key
                        if let (Some(k1), Some(k2)) = (bk.get(ci * 2), bk.get(ci * 2 + 1)) {
                            let y = inner.top() + (ci as f32 + 1.4) * cell;
                            p.text(Pos2::new(inner.right() - 4.0, y), egui::Align2::RIGHT_CENTER, format!("{} {}", acc::key_label(*k1), acc::key_label(*k2)), egui::FontId::monospace(11.0), ink_on(look.body, 220));
                        }
                    }
                    p.text(Pos2::new(bass_r.left() + 10.0, bass_r.bottom() - 30.0), egui::Align2::LEFT_CENTER, "Pairs of keys: bass, chord", egui::FontId::proportional(10.5), ink_on(look.body, 200));
                    p.text(Pos2::new(bass_r.left() + 10.0, bass_r.bottom() - 16.0), egui::Align2::LEFT_CENTER, "Shift: counterbass / minor · Ctrl: 7th · both: dim · ← → move", egui::FontId::proportional(10.5), ink_on(look.body, 200));
                }
            }
        }

        // mouse playing
        {
            let cur = self.acc.as_ref().unwrap().mouse;
            if released || press != cur {
                if let Some(h) = cur {
                    self.acc_play(h, false);
                }
                self.acc.as_mut().unwrap().mouse = None;
                if let Some(h) = press {
                    self.acc.as_mut().unwrap().mouse = Some(h);
                    self.acc_play(h, true);
                }
            }
        }

        // footer
        let reg = {
            let i = self.acc_inst();
            i.registers[self.s.acc_reg.min(i.registers.len() - 1)].name
        };
        p.text(Pos2::new(rect.left() + 4.0, rect.bottom() - 12.0), egui::Align2::LEFT_CENTER, format!("Hold Space to {} · F1–F{} registers ({reg}) · PgUp/PgDn change accordion · MIDI keyboard plays the treble and lights its buttons · load a tune to see which buttons come next", if self.s.acc_space_pull { "pull" } else { "push" }, self.acc_inst().registers.len()), egui::FontId::proportional(11.5), text_dim());
    }
}
