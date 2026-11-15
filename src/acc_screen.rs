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
        let mut events = std::mem::take(&mut self.acc_events);
        if !ctx.input(|i| i.focused) {
            self.acc_release_all();
            return;
        }
        if ctx.wants_keyboard_input() {
            // typing in a text box (e.g. tune search): let it have the keys
            events.clear();
        }
        // nothing on screen should hold keyboard focus in accordion mode
        if let Some(id) = ctx.memory(|m| m.focused()) {
            if !ctx.wants_keyboard_input() {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
        for ev in events {
            let egui::Event::Key { key, pressed, repeat, modifiers, .. } = ev else { continue };
            if std::env::var("NOODLE_KEYDBG").is_ok() {
                eprintln!("key {key:?} {pressed}");
            }
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
                    Key::Backtick => {
                        self.acc_drone_toggle();
                        continue;
                    }
                    Key::Tab => {
                        let n = self.acc_inst().registers.len();
                        self.s.acc_reg = if modifiers.shift { (self.s.acc_reg + n - 1) % n } else { (self.s.acc_reg + 1) % n };
                        let r = self.acc_inst().registers[self.s.acc_reg];
                        let _ = self.tx.send(Cmd::Acc(AccMsg::Register(r)));
                        continue;
                    }
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

    fn acc_drone_toggle(&mut self) {
        if (self.s.drone_mode == 1 || self.s.drone_mode == 2) && self.drone_running {
            self.s.acc_drone_last = self.s.drone_mode;
            self.set_drone_mode(0);
        } else {
            let m = if self.s.acc_drone_last == 1 { 1 } else { 2 };
            self.set_drone_mode(m);
            if m == 2 && self.s.shruti_switches == 0 {
                self.s.shruti_switches = (1 << 0) | (1 << 7);
            }
        }
    }

    /// Drones for accordion mode: big, simple, in the strip above the keys.
    fn acc_drone_panel(&mut self, ui: &mut egui::Ui, strip: Rect) {
        let p = ui.painter_at(strip);
        p.rect_filled(strip, 12.0, card_bg());
        let mut c = ui.new_child(egui::UiBuilder::new().max_rect(strip.shrink2(Vec2::new(14.0, 8.0))).layout(egui::Layout::left_to_right(egui::Align::Center)));
        let ui = &mut c;
        ui.spacing_mut().item_spacing = Vec2::new(8.0, 6.0);
        let mode = self.s.drone_mode;
        let on = (mode == 1 || mode == 2) && self.drone_running;
        // big on/off, also the ` key
        let (txt, col) = if on { ("DRONE ON", GOOD) } else { ("DRONE OFF", card_hi()) };
        let b = egui::Button::new(RichText::new(txt).size(17.0).strong().color(if on { Color32::BLACK } else { fg() })).fill(col).min_size(Vec2::new(130.0, 60.0)).corner_radius(10.0);
        if ui.add(b).on_hover_text("Drone on/off (the ` key, left of 1)").clicked() {
            self.acc_drone_toggle();
        }
        ui.vertical(|ui| {
            ui.label(RichText::new("` key").size(11.0).color(text_dim()));
            ui.horizontal(|ui| {
                for (m, n) in [(1u8, "Pipes"), (2u8, "Shruti")] {
                    let sel = mode == m || (!on && self.s.acc_drone_last == m);
                    if ui.selectable_label(sel, RichText::new(n).size(15.0)).clicked() {
                        self.s.acc_drone_last = m;
                        if on {
                            self.set_drone_mode(m);
                            if m == 2 && self.s.shruti_switches == 0 {
                                self.s.shruti_switches = (1 << 0) | (1 << 7);
                            }
                        }
                    }
                }
            });
        });
        ui.separator();
        // key
        ui.vertical(|ui| {
            ui.label(RichText::new("Key").size(11.0).color(text_dim()));
            egui::ComboBox::from_id_salt("acc_drone_key").selected_text(RichText::new(ROOTS[self.s.scale_root].0).size(20.0).strong()).width(58.0).height(500.0).show_ui(ui, |ui| {
                for (i, r) in ROOTS.iter().enumerate() {
                    ui.selectable_value(&mut self.s.scale_root, i, RichText::new(r.0).size(16.0));
                }
            });
        });
        ui.separator();
        let kind = if on { mode } else if self.s.acc_drone_last == 1 { 1 } else { 2 };
        if kind == 1 {
            // which bagpipe, and each drone on/off
            let pidx = self.s.pipe_idx.min(crate::pipes::PIPES.len() - 1);
            let pp = &crate::pipes::PIPES[pidx];
            ui.vertical(|ui| {
                ui.label(RichText::new("Bagpipe").size(11.0).color(text_dim()));
                let mut pick = None;
                egui::ComboBox::from_id_salt("acc_pipe").selected_text(RichText::new(pp.name).size(15.0)).width(220.0).height(500.0).show_ui(ui, |ui| {
                    for (i, q) in crate::pipes::PIPES.iter().enumerate() {
                        if ui.selectable_label(i == pidx, q.name).on_hover_text(q.note).clicked() {
                            pick = Some(i);
                        }
                    }
                });
                if let Some(i) = pick {
                    self.select_pipe(i);
                }
            });
            if self.s.pipe_drones.len() != pp.drones.len() {
                self.s.pipe_drones = pp.drones.iter().map(|d| d.2).collect();
            }
            let tonic = self.p.pipe_tonic.load(Relaxed);
            ui.vertical(|ui| {
                ui.label(RichText::new("Drones (click to switch on/off)").size(11.0).color(text_dim()));
                ui.horizontal(|ui| {
                    for (i, d) in pp.drones.iter().enumerate() {
                        let don = self.s.pipe_drones[i];
                        let nn = theory::note_name_oct((tonic as i32 + d.0 as i32).clamp(0, 127) as u8);
                        let t = RichText::new(format!("{}\n{}", d.1, nn)).size(14.0).color(if don { Color32::BLACK } else { fg() });
                        if ui.add(egui::Button::new(t).fill(if don { GOOD } else { card_hi() }).min_size(Vec2::new(76.0, 46.0)).corner_radius(8.0)).clicked() {
                            self.s.pipe_drones[i] = !don;
                        }
                    }
                });
            });
        } else {
            // shruti: a lever per note of the octave
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Shruti reeds (click to open/close)").size(11.0).color(text_dim()));
                    let mut preset = None;
                    egui::ComboBox::from_id_salt("acc_shruti").selected_text("Presets").width(110.0).show_ui(ui, |ui| {
                        for (i, (n, _)) in crate::pipes::SHRUTI.iter().enumerate() {
                            if ui.selectable_label(false, *n).clicked() {
                                preset = Some(i);
                            }
                        }
                    });
                    if let Some(i) = preset {
                        self.s.shruti_switches = crate::pipes::SHRUTI[i].1.iter().fold(0u16, |m, r| m | (1 << *r));
                    }
                    ui.selectable_value(&mut self.s.shruti_low, true, "Low");
                    ui.selectable_value(&mut self.s.shruti_low, false, "Mid");
                });
                let sp = self.speller();
                let root = ROOTS[self.s.scale_root].1;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for semi in 0..=12u8 {
                        let son = self.s.shruti_switches & (1 << semi) != 0;
                        let name = sp.name((root + semi) % 12);
                        let t = RichText::new(if semi == 12 { format!("{name}'") } else { name }).size(14.0).strong().color(if son { Color32::BLACK } else { fg() });
                        if ui.add(egui::Button::new(t).fill(if son { GOOD } else { card_hi() }).min_size(Vec2::new(38.0, 34.0)).corner_radius(6.0)).clicked() {
                            self.s.shruti_switches ^= 1 << semi;
                        }
                    }
                });
            });
        }
        ui.separator();
        ui.vertical(|ui| {
            ui.label(RichText::new("Volume").size(11.0).color(text_dim()));
            let mut lv = self.s.drone_level;
            if ui.add(egui::Slider::new(&mut lv, 0.0..=1.0).show_value(false)).changed() {
                self.s.drone_level = lv;
            }
        });
    }

    pub(super) fn acc_screen(&mut self, ui: &mut egui::Ui, rect: Rect) {
        if self.acc.is_none() {
            self.acc = Some(AccState::new());
        }
        // ---- one header line ----
        let mut new_inst: Option<usize> = None;
        let mut leave = false;
        let top = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 34.0));
        let mut bar = ui.new_child(egui::UiBuilder::new().max_rect(top).layout(egui::Layout::left_to_right(egui::Align::Center)));
        {
            let ui = &mut bar;
            let (cur_name, verified, source, nint, reg) = {
                let i = self.acc_inst();
                (i.name, i.verified, i.source, self.acc.as_ref().unwrap().insts.len(), i.registers[self.s.acc_reg.min(i.registers.len() - 1)].name)
            };
            egui::ComboBox::from_id_salt("acc_inst").selected_text(RichText::new(cur_name).size(17.0).strong()).width(260.0).show_ui(ui, |ui| {
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
            ui.label(RichText::new(reg).size(15.0).color(YOURS));
            if verified {
                ui.label(RichText::new("✔").color(GOOD)).on_hover_text(format!("Layout: {source}"));
            } else {
                ui.label(RichText::new("⚠ unverified").color(Color32::from_rgb(0xff, 0xb0, 0x40))).on_hover_text(source);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("✕ Leave (Esc)").clicked() {
                    leave = true;
                }
                ui.selectable_value(&mut self.s.acc_round, false, "Keys");
                ui.selectable_value(&mut self.s.acc_round, true, "Buttons");
                ui.label(RichText::new("Look").size(12.0).color(text_dim()));
                ui.separator();
                ui.toggle_value(&mut self.s.acc_space_pull, "Space = pull").on_hover_text("Off: hold Space to push, let go to pull. On: the other way round.");
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

        // ---- the keyboard, exactly as it sits in front of you ----
        let strip = Rect::from_min_size(Pos2::new(rect.left(), top.bottom() + 6.0), Vec2::new(rect.width(), 92.0));
        self.acc_drone_panel(ui, strip);
        let area = Rect::from_min_max(Pos2::new(rect.left(), strip.bottom() + 8.0), rect.max);
        let width_u = 15.3;
        let height_u = 6.75;
        let u = (area.width() / width_u).min(area.height() / height_u).min(96.0);
        let ox = area.center().x - width_u * u / 2.0;
        let oy = area.top() + (area.height() - height_u * u) / 2.0;
        let p = ui.painter_at(rect);
        let push = self.acc.as_ref().unwrap().push;
        let dir_col = if push { PUSH_COL } else { PULL_COL };
        let mods = ui.input(|i| i.modifiers);
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let down = ui.input(|i| i.pointer.primary_down());
        let (hint_now, hint_next) = self.acc_hints();
        let live: Vec<u8> = (0..128u8).filter(|k| self.held[0][*k as usize] > 0).collect();
        let (bisonoric, strad, nregs, cur_reg) = {
            let i = self.acc_inst();
            (i.kind == Kind::Diatonic, matches!(i.bass, Bass::Stradella), i.registers.len(), self.s.acc_reg.min(i.registers.len() - 1))
        };
        let cap_col = card_hi();
        let dead_col = card_bg();
        let ink = fg();
        let key_rect = |x: f32, row_y: f32, w: f32, h: f32| Rect::from_min_size(Pos2::new(ox + x * u, oy + row_y * u), Vec2::new(w * u - u * 0.08, h * u - u * 0.08));
        let round = self.s.acc_round;
        let look = self.acc_inst().look;
        let dead = |r: Rect, label: &str| {
            if !round {
                p.rect_filled(r, u * 0.12, dead_col);
            }
            if !label.is_empty() && !round {
                p.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(u * 0.17), text_dim());
            }
        };
        // a button: round with a shadow and a sheen on the accordion look, a key cap otherwise
        let cap = |r: Rect, fill: Color32| {
            if round {
                let rad = r.width().min(r.height()) * 0.47;
                let c = r.center();
                p.circle_filled(c + Vec2::new(0.0, rad * 0.1), rad, Color32::from_black_alpha(110));
                p.circle_filled(c, rad, fill);
                p.circle_stroke(c, rad, Stroke::new(1.2, Color32::from_black_alpha(90)));
                p.circle_filled(c - Vec2::new(rad * 0.3, rad * 0.35), rad * 0.22, Color32::from_white_alpha(28));
            } else {
                p.rect_filled(r, u * 0.12, fill);
            }
        };
        let ring = |r: Rect, grow: f32, w: f32, col: Color32| {
            if round {
                p.circle_stroke(r.center(), r.width().min(r.height()) * 0.47 + grow, Stroke::new(w, col));
            } else {
                p.rect_stroke(r.expand(grow), u * 0.15, Stroke::new(w, col), egui::StrokeKind::Outside);
            }
        };
        // where the labels sit on a button: big note, the other-direction note, the computer key
        let spots = |kr: Rect| -> (Pos2, Pos2, Pos2, Align2, Align2) {
            if round {
                (kr.center() - Vec2::new(0.0, u * 0.02), kr.center() + Vec2::new(0.0, u * 0.27), kr.center() - Vec2::new(0.0, u * 0.3), Align2::CENTER_CENTER, Align2::CENTER_CENTER)
            } else {
                (kr.center() + Vec2::new(0.0, u * 0.04), kr.right_top() + Vec2::new(-u * 0.1, u * 0.12), kr.left_bottom() + Vec2::new(u * 0.1, -u * 0.12), Align2::RIGHT_CENTER, Align2::LEFT_CENTER)
            }
        };
        if round {
            // the cases: treble board behind the treble rows, bass board behind the Z row
            let nrows = self.acc_inst().rows.len();
            let (top_y, left_x) = if nrows >= 3 { (0.86, 0.9) } else { (1.86, 1.4) };
            let treble = Rect::from_min_max(Pos2::new(ox + left_x * u, oy + top_y * u), Pos2::new(ox + 13.6 * u, oy + 3.96 * u));
            p.rect_filled(treble, u * 0.25, rgb(look.body));
            p.rect_stroke(treble, u * 0.25, Stroke::new(3.0, rgb(look.accent)), egui::StrokeKind::Inside);
            let bass = Rect::from_min_max(Pos2::new(ox + 2.08 * u, oy + 3.92 * u), Pos2::new(ox + 12.28 * u, oy + 4.96 * u));
            p.rect_filled(bass, u * 0.25, rgb(look.body2).linear_multiply(0.85));
            p.rect_stroke(bass, u * 0.25, Stroke::new(3.0, rgb(look.accent)), egui::StrokeKind::Inside);
        }
        // helper keys (Esc, Tab, Shift, Ctrl) say what they do here
        let fy = 0.0;
        dead(key_rect(0.0, fy, 1.0, 0.75), "Esc\nexit");
        let mut press: Option<Held> = None;
        let mut reg_click: Option<usize> = None;
        // F-row: registers
        for n in 0..12 {
            let x = 1.4 + n as f32 + (n / 4) as f32 * 0.35;
            let r = key_rect(x, fy, 1.0, 0.75);
            if n >= nregs {
                dead(r, "");
                continue;
            }
            let on = n == cur_reg;
            p.rect_filled(r, u * 0.12, if on { Color32::from_rgb(0xf4, 0xee, 0xe0) } else { cap_col });
            let rr = self.acc_inst().registers[n];
            let c = Pos2::new(r.center().x, r.top() + r.height() * 0.42);
            let ink2 = if on { Color32::BLACK } else { ink };
            let cr = r.height() * 0.26;
            p.circle_stroke(c, cr, Stroke::new(1.2, ink2));
            for rank in rr.ranks {
                let d = match rank {
                    crate::reed::Rank::H => Vec2::new(0.0, -cr * 0.6),
                    crate::reed::Rank::M => Vec2::ZERO,
                    crate::reed::Rank::MFlat => Vec2::new(-cr * 0.5, 0.0),
                    crate::reed::Rank::MSharp => Vec2::new(cr * 0.5, 0.0),
                    crate::reed::Rank::L => Vec2::new(0.0, cr * 0.6),
                };
                p.circle_filled(c + d, (cr * 0.2).max(1.5), ink2);
            }
            let short = rr.name.split(" · ").next().unwrap_or(rr.name);
            p.text(Pos2::new(r.center().x, r.bottom() - r.height() * 0.14), Align2::CENTER_CENTER, format!("F{} {}", n + 1, short), FontId::proportional((u * 0.15).max(9.0)), ink2);
            if let Some(pp) = pointer {
                if down && r.contains(pp) {
                    reg_click = Some(n);
                }
            }
        }
        // rows: numbers, Q, A, Z — with the keyboard's real stagger
        let rows_y = [0.95, 1.95, 2.95, 3.95];
        dead(key_rect(0.0, rows_y[0], 1.0, 1.0), "`\ndrone");
        dead(key_rect(13.0, rows_y[0], 2.0, 1.0), "");
        dead(key_rect(0.0, rows_y[1], 1.5, 1.0), "Tab\nnext\nregister");
        dead(key_rect(13.5, rows_y[1], 1.5, 1.0), "");
        dead(key_rect(0.0, rows_y[2], 1.75, 1.0), "");
        dead(key_rect(12.75, rows_y[2], 2.25, 1.0), "");
        dead(key_rect(0.0, rows_y[3], 2.25, 1.0), if strad { "Shift\ncounterbass\n/ minor" } else { "" });
        dead(key_rect(12.25, rows_y[3], 2.75, 1.0), "");
        dead(key_rect(0.0, 4.95, 1.4, 1.0), if strad { "Ctrl\n7th\n(+Shift dim)" } else { "" });
        let held_now: Vec<Held> = {
            let a = self.acc.as_ref().unwrap();
            a.down.values().copied().chain(a.mouse).collect()
        };
        // treble rows
        {
            let inst = self.acc_inst();
            let row_x = [1.75, 1.5, 1.0];
            let row_y = [rows_y[2], rows_y[1], rows_y[0]];
            for (r, row) in inst.rows.iter().enumerate().take(3) {
                let home = if bisonoric { row.get(2).map(|b| b.push).unwrap_or(0) } else { 0 };
                for (i, b) in row.iter().enumerate() {
                    let Some(k) = acc::treble_key(inst, r, i) else { continue };
                    let kr = key_rect(row_x[r] + i as f32, row_y[r], 1.0, 1.0);
                    let held = held_now.contains(&Held::Treble { r, i });
                    let sharp = !bisonoric && is_black(b.push);
                    let fill = if held {
                        dir_col
                    } else if sharp {
                        Color32::from_rgb(0x1a, 0x1a, 0x1e)
                    } else {
                        Color32::from_rgb(0xf2, 0xee, 0xe4)
                    };
                    let fill = if round && !held && !sharp { rgb(look.button) } else { fill };
                    cap(kr, fill);
                    let ink3 = if held || !sharp { Color32::from_rgb(0x18, 0x18, 0x18) } else { Color32::WHITE };
                    if bisonoric {
                        let (big, small) = if push { (b.push, b.pull) } else { (b.pull, b.push) };
                        let (m, sm, _, sa, _) = spots(kr);
                        p.text(m, Align2::CENTER_CENTER, spell(big, home), FontId::proportional(u * if round { 0.36 } else { 0.42 }), ink3);
                        p.text(sm, sa, spell(small, home), FontId::proportional(u * if round { 0.17 } else { 0.2 }), ink3.linear_multiply(0.5));
                    } else {
                        p.text(spots(kr).0, Align2::CENTER_CENTER, spell(b.push, 0), FontId::proportional(u * if round { 0.38 } else { 0.42 }), ink3);
                        if b.push % 12 == 0 {
                            ring(kr, -u * 0.06, 2.0, Color32::from_rgb(0xd0, 0x40, 0x40));
                        }
                    }
                    let (_, _, kp, _, ka) = spots(kr);
                    p.text(kp, ka, acc::key_label(k), FontId::monospace(u * if round { 0.13 } else { 0.16 }), ink3.linear_multiply(0.45));
                    if matches!(k, Key::F | Key::J) && !round {
                        p.line_segment([kr.center_bottom() + Vec2::new(-u * 0.12, -u * 0.1), kr.center_bottom() + Vec2::new(u * 0.12, -u * 0.1)], Stroke::new(2.0, ink3.linear_multiply(0.6)));
                    }
                    // hints: song / MIDI notes now (solid ring), next note (dashed)
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
                            ring(kr, u * 0.05, 3.0, if d { PUSH_COL } else { PULL_COL });
                        }
                    }
                    if let Some(n) = hint_next {
                        if let Some(d) = wants(n) {
                            let col = if d { PUSH_COL } else { PULL_COL };
                            let e = kr.expand(u * 0.09);
                            if round {
                                let rr = kr.width().min(kr.height()) * 0.47 + u * 0.1;
                                for t in 0..12 {
                                    let a0 = t as f32 / 12.0 * std::f32::consts::TAU;
                                    p.line_segment([kr.center() + Vec2::angled(a0) * rr, kr.center() + Vec2::angled(a0 + 0.28) * rr], Stroke::new(2.5, col));
                                }
                            }
                            for t in 0..(if round { 0 } else { 8 }) {
                                let f0 = t as f32 / 8.0;
                                let f1 = f0 + 0.06;
                                for (a0, a1) in [(e.left_top(), e.right_top()), (e.left_bottom(), e.right_bottom())] {
                                    p.line_segment([a0 + (a1 - a0) * f0, a0 + (a1 - a0) * f1], Stroke::new(2.5, col));
                                }
                            }
                        }
                    }
                    if let Some(pp) = pointer {
                        if down && kr.contains(pp) {
                            press = Some(Held::Treble { r, i });
                        }
                    }
                }
            }
            // unused keys on the number row for 2-row boxes
            for r in inst.rows.len()..3 {
                let n = if r == 2 { 12 } else { 12 };
                for i in 0..n {
                    dead(key_rect(row_x[r] + i as f32, row_y[r], 1.0, 1.0), "");
                }
            }
        }
        // Z row: the left hand
        {
            let inst = self.acc_inst();
            let center = self.acc.as_ref().unwrap().center;
            for (k_i, key) in acc::ZROW.iter().enumerate() {
                let kr = key_rect(2.25 + k_i as f32, rows_y[3], 1.0, 1.0);
                let Some(h) = self.acc_key_to_held(*key, mods) else {
                    dead(kr, "");
                    continue;
                };
                let held = held_now.iter().any(|x| match (x, &h) {
                    (Held::Strad { col: c1, .. }, Held::Strad { col: c2, .. }) => c1 == c2 && self.acc.as_ref().unwrap().down.get(key).is_some(),
                    _ => *x == h,
                });
                let base = Color32::from_rgb(0xd9, 0xc8, 0xa6);
                cap(kr, if held { dir_col } else if round { rgb(look.button) } else { base });
                let ink3 = Color32::from_rgb(0x18, 0x18, 0x18);
                let lab = |s: acc::BassSnd| match s {
                    acc::BassSnd::Note(pc) => acc::NAMES[pc as usize].to_string(),
                    acc::BassSnd::Major(pc) => format!("{} maj", acc::NAMES[pc as usize]),
                    acc::BassSnd::Minor(pc) => format!("{} min", acc::NAMES[pc as usize]),
                };
                match (h, &inst.bass) {
                    (Held::DiaBass { r, i }, Bass::Diatonic(rows)) => {
                        let (pu, pl) = rows[r][i];
                        let (big, small) = if push { (pu, pl) } else { (pl, pu) };
                        let (m, sm, _, sa, _) = spots(kr);
                        p.text(m, Align2::CENTER_CENTER, lab(big), FontId::proportional(u * if matches!(big, acc::BassSnd::Note(_)) { 0.38 } else { 0.24 }), ink3);
                        p.text(sm, sa, lab(small), FontId::proportional(u * if round { 0.14 } else { 0.18 }), ink3.linear_multiply(0.5));
                    }
                    (Held::Strad { col, row }, _) => {
                        let (m, sm, _, sa, _) = spots(kr);
                        p.text(m, Align2::CENTER_CENTER, acc::stradella_label(col, row), FontId::proportional(u * if row <= 1 { 0.38 } else { 0.24 }), ink3);
                        p.text(sm, sa, acc::STRADELLA_ROWS[row].to_lowercase(), FontId::proportional(u * 0.13), ink3.linear_multiply(0.5));
                    }
                    _ => {}
                }
                let (_, _, kp, _, ka) = spots(kr);
                p.text(kp, ka, acc::key_label(*key), FontId::monospace(u * if round { 0.13 } else { 0.16 }), ink3.linear_multiply(0.45));
                if let Some(pp) = pointer {
                    if down && kr.contains(pp) {
                        press = Some(h);
                    }
                }
            }
            if strad && !round {
                let r = key_rect(12.25, rows_y[3], 2.75, 1.0);
                p.text(r.center(), Align2::CENTER_CENTER, format!("← → move basses\n(centre {})", acc::NAMES[center as usize]), FontId::proportional(u * 0.15), text_dim());
            }
        }
        // the space bar is the bellows
        {
            let r = key_rect(3.6, 4.95, 7.2, 1.0);
            if round {
                // bellows: folds in the case colour, edged in the direction colour
                p.rect_filled(r, u * 0.08, rgb(look.bellows));
                let folds = 16;
                for k in 1..folds {
                    let x = r.left() + r.width() * k as f32 / folds as f32;
                    p.line_segment([Pos2::new(x, r.top()), Pos2::new(x, r.bottom())], Stroke::new(if k % 2 == 0 { 2.0 } else { 1.0 }, mix(rgb(look.bellows), Color32::WHITE, if k % 2 == 0 { 0.35 } else { 0.12 })));
                }
                p.rect_stroke(r, u * 0.08, Stroke::new(4.0, dir_col), egui::StrokeKind::Inside);
                p.rect_filled(Rect::from_center_size(r.center(), Vec2::new(u * 2.6, u * 0.8)), u * 0.12, dir_col);
            } else {
                p.rect_filled(r, u * 0.14, dir_col);
            }
            p.text(r.center(), Align2::CENTER_CENTER, if push { "◀ PUSH ▶" } else { "◀ PULL ▶" }, FontId::proportional(u * 0.4), Color32::from_rgb(0x10, 0x10, 0x10));
            if round {
                let extra = if strad { "   ·   Shift: counterbass / minor   ·   Ctrl: 7th   ·   both: dim   ·   ← → move basses" } else { "" };
                p.text(r.center_bottom() + Vec2::new(0.0, u * 0.3), Align2::CENTER_CENTER, format!("` drone   ·   Tab: next register   ·   Esc: leave{extra}"), FontId::proportional((u * 0.16).max(11.0)), text_dim());
            }
            let hint = if self.s.acc_space_pull { "hold = pull · let go = push" } else { "hold = push · let go = pull" };
            p.text(r.center_bottom() - Vec2::new(0.0, u * 0.14), Align2::CENTER_CENTER, hint, FontId::proportional(u * 0.14), Color32::from_rgb(0x10, 0x10, 0x10));
        }

        if let Some(n) = reg_click {
            if n != self.s.acc_reg {
                self.s.acc_reg = n;
                let r = self.acc_inst().registers[n];
                let _ = self.tx.send(Cmd::Acc(AccMsg::Register(r)));
            }
        }
        // mouse playing
        let cur = self.acc.as_ref().unwrap().mouse;
        if !down || press != cur {
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
}
