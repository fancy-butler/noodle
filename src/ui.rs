// Noodle — the user interface.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use crate::drums::{KITS, PATTERNS};
use crate::game::{self, Game};
use crate::engine::*;
use crate::settings::Settings;
use crate::song::{self, Song};
use crate::sounds::{self, Loaded, PresetInfo, GM_FAMILIES};
use crate::theory::{self, Speller, ROOTS, SCALES};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, Align2, Color32, FontFamily, FontId, Key, Pos2, Rect, RichText, Sense, Stroke, StrokeKind, UiBuilder, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::Instant;

// ---------- palette ----------
static LIGHT: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
fn light() -> bool {
    LIGHT.load(Relaxed)
}
fn pick(dark: Color32, lite: Color32) -> Color32 {
    if light() {
        lite
    } else {
        dark
    }
}
fn bg() -> Color32 {
    pick(Color32::from_rgb(0x12, 0x14, 0x17), Color32::from_rgb(0xec, 0xee, 0xf1))
}
fn panel() -> Color32 {
    pick(Color32::from_rgb(0x19, 0x1c, 0x21), Color32::from_rgb(0xf7, 0xf8, 0xfa))
}
fn card_bg() -> Color32 {
    pick(Color32::from_rgb(0x21, 0x25, 0x2b), Color32::from_rgb(0xff, 0xff, 0xff))
}
fn card_hi() -> Color32 {
    pick(Color32::from_rgb(0x2b, 0x30, 0x38), Color32::from_rgb(0xe2, 0xe5, 0xea))
}
fn text_dim() -> Color32 {
    pick(Color32::from_rgb(0x8a, 0x93, 0xa0), Color32::from_rgb(0x5d, 0x66, 0x73))
}
/// main text / ink colour
fn fg() -> Color32 {
    pick(Color32::WHITE, Color32::from_rgb(0x16, 0x19, 0x1d))
}
/// theme-aware grey: dark-mode value v, mirrored for light mode
fn gray(v: u8) -> Color32 {
    if light() {
        Color32::from_gray(255u8.saturating_sub(v).saturating_add(12))
    } else {
        Color32::from_gray(v)
    }
}
fn deep() -> Color32 {
    pick(Color32::from_rgb(0x0d, 0x0f, 0x12), Color32::from_rgb(0xe4, 0xe7, 0xec))
}
const LIVE: Color32 = Color32::from_rgb(0xff, 0x8a, 0x3d);
const ARP: Color32 = Color32::from_rgb(0x3d, 0xd6, 0xff);
const LOOPC: Color32 = Color32::from_rgb(0xb4, 0x8c, 0xff);
const SONG: Color32 = Color32::from_rgb(0x6b, 0x84, 0xb0);
const YOURS: Color32 = Color32::from_rgb(0xff, 0xd2, 0x3d);
const GOOD: Color32 = Color32::from_rgb(0x5a, 0xd1, 0x7a);
const BAD: Color32 = Color32::from_rgb(0xff, 0x4d, 0x5e);

const SRC_COL: [Color32; 3] = [LIVE, ARP, LOOPC];


const KEYMAP: [(Key, u8); 17] = [
    (Key::A, 0),
    (Key::W, 1),
    (Key::S, 2),
    (Key::E, 3),
    (Key::D, 4),
    (Key::F, 5),
    (Key::T, 6),
    (Key::G, 7),
    (Key::Y, 8),
    (Key::H, 9),
    (Key::U, 10),
    (Key::J, 11),
    (Key::K, 12),
    (Key::O, 13),
    (Key::L, 14),
    (Key::P, 15),
    (Key::Semicolon, 16),
];

struct Bar {
    key: u8,
    src: u8,
    vel: u8,
    start: f64,
    end: Option<f64>,
}

pub struct Handles {
    pub p: Arc<Params>,
    pub tx: Sender<Cmd>,
    pub rx: Receiver<UiEvt>,
    pub devices: Arc<Mutex<Vec<String>>>,
    pub blocked: Arc<Mutex<Vec<String>>>,
    pub sample_rate: u32,
    pub audio_info: Result<String, String>,
    pub engine: crate::audio::SharedEngine,
    pub audio: Option<crate::audio::Audio>,
}

pub struct App {
    p: Arc<Params>,
    tx: Sender<Cmd>,
    rx: Receiver<UiEvt>,
    devices: Arc<Mutex<Vec<String>>>,
    blocked: Arc<Mutex<Vec<String>>>,
    sample_rate: u32,
    audio_info: Result<String, String>,
    engine: crate::audio::SharedEngine,
    audio: Option<crate::audio::Audio>,
    s: Settings,
    saved_json: String,
    last_save: f64,
    // sounds
    load_tx: Sender<Result<Loaded, String>>,
    load_rx: Receiver<Result<Loaded, String>>,
    loading: Option<String>,
    load_err: Option<String>,
    sf_paths: Vec<PathBuf>,
    sf_current: Option<PathBuf>,
    presets: Vec<PresetInfo>,
    category: Option<usize>, // None all, Some(0..16) family, Some(99) favourites
    // notes
    t0: Instant,
    bars: Vec<Bar>,
    held: [[u8; 128]; 3],
    keys_down: HashMap<Key, u8>,
    mouse_note: Option<u8>,
    // scale trainer
    scale_on: bool,
    walk: bool,
    walk_seq: Vec<u8>,
    walk_pos: usize,
    walk_laps: u32,
    wrong: [f64; 128],
    right: [f64; 128],
    // song
    song: Option<Arc<Song>>,
    song_files: Vec<PathBuf>,
    song_err: Option<String>,
    song_user: u16,
    song_wait: bool,
    // misc
    taps: Vec<f64>,
    music_font: bool,
    show_help: bool,
    // controller
    ml3_active: bool,
    ml3_seen: bool,
    cc_vals: [u8; 128],
    cc_seen: [f64; 128],
    bend: i16,
    pad_hit: [f64; 8],
    wind_active: bool,
    wind_name: String,
    scale_filter: String,
    style_done: bool,
    game_mode: bool,
    game: Option<Game>,
    show_results: bool,
    drone_running: bool,
    pipe_picker: bool,
}

fn now_s(t0: &Instant) -> f64 {
    t0.elapsed().as_secs_f64()
}

fn is_black(k: u8) -> bool {
    matches!(k % 12, 1 | 3 | 6 | 8 | 10)
}

fn white_index(lo: u8, k: u8) -> f32 {
    (lo..k).filter(|x| !is_black(*x)).count() as f32
}

/// Horizontal geometry of key `k` when keys lo..=hi fill `r`: (left, width, black?)
fn key_x(r: Rect, k: u8, lo: u8, hi: u8) -> (f32, f32, bool) {
    let whites = (lo..=hi).filter(|x| !is_black(*x)).count().max(1) as f32;
    let ww = r.width() / whites;
    if is_black(k) {
        let c = r.left() + (white_index(lo, k.saturating_sub(1)) + 1.0) * ww;
        (c - ww * 0.31, ww * 0.62, true)
    } else {
        (r.left() + white_index(lo, k) * ww, ww, false)
    }
}

/// Common controller sizes and where they start by default.
pub const KB_SIZES: [(u8, u8); 7] = [(25, 48), (32, 41), (37, 48), (49, 36), (61, 36), (76, 28), (88, 21)];

/// MiniLab 3 factory CCs (Arturia support chart): knobs 1-8, faders 1-4, main encoder, shift+encoder.
pub const MINILAB3_MAP: [(u8, u8); 14] = [
    (74, target::FILTER),
    (71, target::RESO),
    (76, target::DRIVE),
    (77, target::DELAY),
    (93, target::REVERB),
    (18, target::REPEATS),
    (19, target::CHORUS),
    (16, target::ARP_GATE),
    (82, target::MASTER),
    (83, target::LAYER_VOL),
    (85, target::DRUM_VOL),
    (17, target::SONG_VOL),
    (114, target::BROWSE),
    (112, target::CATEGORY),
];
/// Staff transposition (written pitch) for transposing instruments: (label, semitones added to concert pitch)
pub const WRITTEN: [(&str, i32); 7] = [
    ("Concert pitch", 0),
    ("B♭ (soprano sax, clarinet, trumpet)", 2),
    ("B♭ tenor sax (+9th)", 14),
    ("E♭ alto sax", 9),
    ("E♭ baritone sax", 21),
    ("F (horn)", 7),
    ("Tin whistle (written 8vb)", -12),
];

const ML3_KNOBS: [u8; 8] = [74, 71, 76, 77, 93, 18, 19, 16];
const ML3_FADERS: [u8; 4] = [82, 83, 85, 17];

fn arc_points(c: Pos2, r: f32, a0: f32, a1: f32) -> Vec<Pos2> {
    let n = 32;
    (0..=n).map(|i| {
        let a = a0 + (a1 - a0) * i as f32 / n as f32;
        c + Vec2::new(a.cos(), a.sin()) * r
    })
    .collect()
}

/// A synth-style knob. Drag up/down (or scroll) to change, double-click to reset.
fn knob(ui: &mut egui::Ui, label: &str, v: &mut f32, default: f32, col: Color32, fmt: impl Fn(f32) -> String) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(54.0, 64.0), Sense::click_and_drag());
    let mut changed = false;
    if resp.dragged() {
        let d = resp.drag_delta();
        let fine = if ui.input(|i| i.modifiers.shift) { 0.2 } else { 1.0 };
        *v = (*v + (d.x - d.y) * 0.006 * fine).clamp(0.0, 1.0);
        changed = true;
    }
    if resp.hovered() {
        let sc = ui.input(|i| i.smooth_scroll_delta.y);
        if sc.abs() > 0.0 {
            *v = (*v + sc * 0.002).clamp(0.0, 1.0);
            changed = true;
        }
    }
    if resp.double_clicked() {
        *v = default;
        changed = true;
    }
    let p = ui.painter();
    let c = Pos2::new(rect.center().x, rect.top() + 24.0);
    let r = 17.0;
    let a0 = std::f32::consts::PI * 0.75;
    let a1 = std::f32::consts::PI * 2.25;
    p.circle_filled(c, r - 4.0, card_hi());
    p.add(egui::Shape::line(arc_points(c, r, a0, a1), Stroke::new(3.5, gray(55))));
    let av = a0 + (a1 - a0) * *v;
    if *v > 0.003 {
        p.add(egui::Shape::line(arc_points(c, r, a0, av), Stroke::new(3.5, col)));
    }
    let tip = c + Vec2::new(av.cos(), av.sin()) * (r - 6.0);
    p.line_segment([c, tip], Stroke::new(2.5, fg()));
    let txt = if resp.hovered() || resp.dragged() { fmt(*v) } else { label.to_string() };
    p.text(Pos2::new(c.x, rect.bottom() - 7.0), Align2::CENTER_CENTER, txt, FontId::proportional(11.5), if resp.hovered() { fg() } else { text_dim() });
    resp.on_hover_text(format!("{label}: {}\nDrag up/down · Shift = fine · double-click resets", fmt(*v)));
    changed
}

fn tget(p: &Params, t: u8) -> f32 {
    match t {
        target::FILTER => p.cutoff.get(),
        target::RESO => p.resonance.get(),
        target::DRIVE => p.drive.get(),
        target::DELAY => p.delay_mix.get(),
        target::REVERB => p.reverb.get(),
        target::REPEATS => p.delay_fb.get(),
        target::CHORUS => p.chorus.get(),
        target::ARP_GATE => p.arp_gate.get(),
        target::MASTER => p.master.get(),
        target::LAYER_VOL => p.layer_vol.get(),
        target::DRUM_VOL => p.drum_vol.get(),
        target::SONG_VOL => p.song_vol.get(),
        target::SWING => p.swing.get(),
        _ => 0.0,
    }
}

fn tset(p: &Params, t: u8, v: f32) {
    match t {
        target::FILTER => p.cutoff.set(v),
        target::RESO => p.resonance.set(v),
        target::DRIVE => p.drive.set(v),
        target::DELAY => p.delay_mix.set(v),
        target::REVERB => p.reverb.set(v),
        target::REPEATS => p.delay_fb.set(v),
        target::CHORUS => p.chorus.set(v),
        target::ARP_GATE => p.arp_gate.set(v.max(0.05)),
        target::MASTER => p.master.set(v),
        target::LAYER_VOL => p.layer_vol.set(v),
        target::DRUM_VOL => p.drum_vol.set(v),
        target::SONG_VOL => p.song_vol.set(v),
        target::SWING => p.swing.set(v),
        _ => {}
    }
}

/// Vertical fader. Drag the cap.
fn fader(ui: &mut egui::Ui, label: &str, v: &mut f32, col: Color32, hot: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(32.0, 118.0), Sense::click_and_drag());
    let track = Rect::from_min_max(Pos2::new(rect.center().x - 2.0, rect.top() + 6.0), Pos2::new(rect.center().x + 2.0, rect.bottom() - 22.0));
    let mut changed = false;
    if resp.dragged() || resp.clicked() {
        if let Some(pos) = resp.interact_pointer_pos() {
            *v = (1.0 - (pos.y - track.top()) / track.height()).clamp(0.0, 1.0);
            changed = true;
        }
    }
    let p = ui.painter();
    p.rect_filled(track, 2.0, gray(45));
    let y = track.bottom() - *v * track.height();
    p.rect_filled(Rect::from_min_max(Pos2::new(track.left(), y), track.right_bottom()), 2.0, col.gamma_multiply(0.7));
    let cap = Rect::from_center_size(Pos2::new(rect.center().x, y), Vec2::new(26.0, 10.0));
    p.rect_filled(cap, 3.0, if hot { fg() } else { gray(200) });
    p.line_segment([Pos2::new(cap.left() + 3.0, y), Pos2::new(cap.right() - 3.0, y)], Stroke::new(1.5, gray(60)));
    p.text(Pos2::new(rect.center().x, rect.bottom() - 8.0), Align2::CENTER_CENTER, label, FontId::proportional(11.0), if hot { fg() } else { text_dim() });
    resp.on_hover_text(format!("{label}: {}", pct(*v)));
    changed
}

fn midi_short(n: &str) -> String {
    n.chars().take(22).collect()
}

fn pct(v: f32) -> String {
    format!("{:.0}%", v * 100.0)
}

fn card<R>(ui: &mut egui::Ui, title: &str, width: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    card_h(ui, title, width, 176.0, add)
}

fn card_h<R>(ui: &mut egui::Ui, title: &str, width: f32, height: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::new()
        .fill(card_bg())
        .corner_radius(10.0)
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(width);
            ui.set_height(height);
            ui.vertical(|ui| {
                ui.label(RichText::new(title).size(12.0).strong().color(text_dim()));
                ui.add_space(2.0);
                add(ui)
            })
            .inner
        })
        .inner
}

fn big_toggle(ui: &mut egui::Ui, on: bool, label: &str, col: Color32, size: Vec2) -> egui::Response {
    let fill = if on { col } else { card_hi() };
    let tc = if on { Color32::BLACK } else { fg() };
    ui.add_sized(size, egui::Button::new(RichText::new(label).strong().size(14.0).color(tc)).fill(fill).corner_radius(8.0))
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, h: Handles) -> Self {
        let music_font = setup_fonts(&cc.egui_ctx);
        setup_style(&cc.egui_ctx);
        let s = Settings::load();
        s.push_to(&h.p);
        let (load_tx, load_rx) = crossbeam_channel::unbounded();
        let sf_paths = sounds::find_soundfonts();
        let mut app = App {
            p: h.p,
            tx: h.tx,
            rx: h.rx,
            devices: h.devices,
            blocked: h.blocked,
            sample_rate: h.sample_rate,
            audio_info: h.audio_info,
            engine: h.engine,
            audio: h.audio,
            saved_json: String::new(),
            last_save: 0.0,
            load_tx,
            load_rx,
            loading: None,
            load_err: None,
            sf_paths,
            sf_current: None,
            presets: vec![],
            category: None,
            t0: Instant::now(),
            bars: Vec::with_capacity(4096),
            held: [[0; 128]; 3],
            keys_down: HashMap::new(),
            mouse_note: None,
            scale_on: s.scale_on,
            walk: false,
            walk_seq: vec![],
            walk_pos: 0,
            walk_laps: 0,
            wrong: [-10.0; 128],
            right: [-10.0; 128],
            song: None,
            song_files: sounds::find_songs(),
            song_err: None,
            song_user: 0,
            song_wait: false,
            taps: vec![],
            music_font,
            show_help: false,
            ml3_active: false,
            ml3_seen: false,
            cc_vals: [0; 128],
            cc_seen: [-10.0; 128],
            bend: 0,
            pad_hit: [-10.0; 8],
            wind_active: false,
            wind_name: String::new(),
            scale_filter: String::new(),
            style_done: false,
            game_mode: false,
            game: None,
            show_results: false,
            drone_running: false,
            pipe_picker: false,
            s,
        };
        // pick the soundfont: last used if it still exists, else the best one found
        let pick = app.s.soundfont.clone().filter(|p| p.exists()).or_else(|| app.sf_paths.first().cloned());
        match pick {
            Some(p) => app.load_sf(p),
            None => app.load_err = Some(format!(
                "No SoundFont (.sf2) found. Drop one onto this window, or put it in {}",
                sounds::user_dir().join("soundfonts").display()
            )),
        }
        app.rebuild_walk();
        app
    }

    /// Reopen the sound card with a new buffer size. The watchdog keeps the engine
    /// alive during the switch.
    fn game_key(&self) -> String {
        let sc = &SCALES[self.s.scale_idx.min(SCALES.len() - 1)];
        if self.s.g_pattern == 6 {
            format!("Rhythm · {} · {} bpm", game::RHYTHMS[self.s.g_rhythm], self.s.g_bpm as i32)
        } else {
            format!("{} {} · {} · {} · {} oct · {} bpm", ROOTS[self.s.scale_root].0, sc.name, game::PATTERNS[self.s.g_pattern], game::RHYTHMS[self.s.g_rhythm], self.s.g_octaves, self.s.g_bpm as i32)
        }
    }

    fn start_game(&mut self, from_song: bool) {
        self.show_results = false;
        let (chart, mask, name) = if from_song {
            let Some(song) = self.song.clone() else { return };
            if self.song_user == 0 {
                self.song_err = Some("Pick the part you play first (You play: …)".into());
                return;
            }
            let name = format!("{} · song", song.name);
            (song, self.song_user, name)
        } else {
            let sc = &SCALES[self.s.scale_idx.min(SCALES.len() - 1)];
            let cfg = game::DrillCfg {
                root_pc: ROOTS[self.s.scale_root].1,
                pattern: self.s.g_pattern,
                rhythm: self.s.g_rhythm,
                bpm: self.s.g_bpm,
                octaves: self.s.g_octaves,
                bars: self.s.g_bars,
                bank: self.s.main.0,
                program: self.s.main.1,
            };
            let name = self.game_key();
            let chart = Arc::new(game::make_drill(&cfg, sc, &name));
            // make the drill visible on small keyboards
            if let (Some(lo), Some(hi)) = (chart.notes.iter().map(|n| n.key).min(), chart.notes.iter().map(|n| n.key).max()) {
                if self.s.kb_size < 88 && (lo < self.range().0 || hi > self.range().1) {
                    let span = hi - lo;
                    if span < self.s.kb_size {
                        self.s.kb_lo = (lo / 12) * 12;
                    }
                }
            }
            self.set_song(Some(chart.clone()), if self.s.g_hear { 0 } else { 1 });
            (chart, 1u16, name)
        };
        self.p.song_wait.store(false, Relaxed);
        self.song_wait = false;
        if from_song {
            let _ = self.tx.send(Cmd::SongSeek(0.0));
        }
        self.game = Some(Game::new(chart, mask, self.s.g_any_octave, name));
        let _ = self.tx.send(Cmd::SongPlay(true));
    }

    fn game_tick(&mut self) {
        let now = now_s(&self.t0);
        let pos = self.p.song_pos.get();
        let speed = self.p.song_speed.get() as f64;
        let Some(g) = self.game.as_mut() else { return };
        let was = g.finished;
        g.tick(pos, speed, now);
        if g.finished && !was {
            let acc = g.accuracy();
            let grade = g.grade().to_string();
            let key = g.name.clone();
            let better = self.s.g_best.get(&key).map(|b| acc > b.0).unwrap_or(true);
            if better && g.judged() > 0 {
                self.s.g_best.insert(key, (acc, grade));
            }
            self.show_results = true;
        }
    }

    fn game_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new("🎮 GAME").strong().color(YOURS));
            let sidx = self.s.scale_idx.min(SCALES.len() - 1);
            if self.s.g_pattern != 6 {
                ui.label(RichText::new(format!("{} {}", ROOTS[self.s.scale_root].0, SCALES[sidx].name)).color(text_dim())).on_hover_text("Change root and scale with the Scale controls above");
            }
            egui::ComboBox::from_id_salt("g_pat").selected_text(game::PATTERNS[self.s.g_pattern]).width(140.0).show_ui(ui, |ui| {
                for (i, n) in game::PATTERNS.iter().enumerate() {
                    ui.selectable_value(&mut self.s.g_pattern, i, *n);
                }
            });
            egui::ComboBox::from_id_salt("g_rhy").selected_text(game::RHYTHMS[self.s.g_rhythm]).width(96.0).show_ui(ui, |ui| {
                for (i, n) in game::RHYTHMS.iter().enumerate() {
                    ui.selectable_value(&mut self.s.g_rhythm, i, *n);
                }
            });
            ui.add(egui::DragValue::new(&mut self.s.g_bpm).range(30.0..=240.0).speed(0.5).fixed_decimals(0).suffix(" bpm"));
            if self.s.g_pattern >= 4 {
                ui.add(egui::DragValue::new(&mut self.s.g_bars).range(1..=32).suffix(" bars"));
            }
            if self.s.g_pattern < 6 {
                for o in 1..=2u8 {
                    if ui.selectable_label(self.s.g_octaves == o, format!("{o} oct")).clicked() {
                        self.s.g_octaves = o;
                    }
                }
            }
            ui.toggle_value(&mut self.s.g_any_octave, "Any octave").on_hover_text("Right note in any octave counts — handy on a 25-key keyboard");
            ui.toggle_value(&mut self.s.g_hear, "Hear targets").on_hover_text("Play the target notes too (practice). Off = only you.");
            if big_toggle(ui, false, "▶ START", GOOD, Vec2::new(86.0, 24.0)).on_hover_text("One bar of count-in clicks, then go").clicked() {
                self.start_game(false);
            }
            if self.song.is_some() && self.song_user != 0 && self.song.as_ref().map(|s| s.name != "Scale" && !s.name.contains('·')).unwrap_or(false) {
                if ui.button("▶ Play song as game").clicked() {
                    self.start_game(true);
                }
            }
            if self.game.is_some() && ui.button("■ Stop").clicked() {
                let _ = self.tx.send(Cmd::SongPlay(false));
                self.game = None;
            }
            if let Some(b) = self.s.g_best.get(&self.game_key()) {
                ui.label(RichText::new(format!("best {} · {:.1}%", b.1, b.0 * 100.0)).color(YOURS));
            }
        });
    }

    fn restart_audio(&mut self, frames: u32) {
        self.s.latency = frames;
        self.audio = None; // closes the old stream
        std::thread::sleep(std::time::Duration::from_millis(60));
        match crate::audio::start(self.engine.clone(), self.p.clone(), frames) {
            Ok(a) => {
                self.audio_info = Ok(format!("{} · {} kHz", a.device, a.sample_rate / 1000));
                if a.sample_rate != self.sample_rate {
                    // the synth must run at the card's rate: reload the bank at the new rate
                    self.sample_rate = a.sample_rate;
                    if let Some(sf) = self.sf_current.clone() {
                        self.load_sf(sf);
                    }
                }
                self.audio = Some(a);
            }
            Err(e) => self.audio_info = Err(format!("No audio: {e}")),
        }
    }

    fn load_sf(&mut self, path: PathBuf) {
        let tx = self.load_tx.clone();
        let rate = self.sample_rate;
        self.loading = Some(path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default());
        self.load_err = None;
        std::thread::spawn(move || {
            let _ = tx.send(sounds::load_soundfont(&path, rate));
        });
    }

    fn preset_name(&self, bp: (i32, i32)) -> String {
        self.presets
            .iter()
            .find(|p| (p.bank, p.prog) == bp)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| if self.presets.is_empty() { "Loading…".into() } else { format!("Bank {} · {}", bp.0, bp.1 + 1) })
    }

    fn filtered(&self) -> Vec<usize> {
        self.presets
            .iter()
            .enumerate()
            .filter(|(_, p)| p.bank < 128)
            .filter(|(_, p)| match self.category {
                None => true,
                Some(99) => self.s.favorites.contains(&(p.bank, p.prog)),
                Some(f) => sounds::family(p) == f,
            })
            .map(|(i, _)| i)
            .collect()
    }

    fn set_preset(&mut self, layer: bool, bp: (i32, i32)) {
        if layer {
            self.s.layer = bp;
        } else {
            self.s.main = bp;
        }
        let _ = self.tx.send(Cmd::Program { layer, bank: bp.0, prog: bp.1 });
    }

    fn step_preset(&mut self, layer: bool, dir: i32) {
        let list = self.filtered();
        if list.is_empty() {
            return;
        }
        let cur = if layer { self.s.layer } else { self.s.main };
        let pos = list.iter().position(|i| (self.presets[*i].bank, self.presets[*i].prog) == cur);
        let next = match pos {
            Some(p) => (p as i32 + dir).rem_euclid(list.len() as i32) as usize,
            None => 0,
        };
        let pr = &self.presets[list[next]];
        let bp = (pr.bank, pr.prog);
        self.set_preset(layer, bp);
    }

    fn range(&self) -> (u8, u8) {
        let size = self.s.kb_size.clamp(12, 88);
        let lo = self.s.kb_lo.min(127 - size + 1);
        (lo, lo + size - 1)
    }

    fn set_kb_size(&mut self, size: u8) {
        let lo = KB_SIZES.iter().find(|(s, _)| *s == size).map(|(_, l)| *l).unwrap_or(21);
        self.s.kb_size = size;
        self.s.kb_lo = lo;
    }

    /// Slide the on-screen keyboard by octaves so `key` is visible (follows the Oct buttons).
    fn follow(&mut self, key: u8) {
        if !self.s.kb_follow || self.s.kb_size >= 88 {
            return;
        }
        let (mut lo, mut hi) = self.range();
        let size = self.s.kb_size;
        while key < lo && lo >= 12 {
            lo -= 12;
            hi -= 12;
        }
        while key > hi && hi + 12 <= 127 {
            lo += 12;
            hi += 12;
        }
        let _ = size;
        self.s.kb_lo = lo;
    }

    fn apply_controller(&mut self) {
        let devs: Vec<String> = self.devices.lock().map(|d| d.iter().filter(|n| !n.starts_with('-')).cloned().collect()).unwrap_or_default();
        let ml3 = devs.iter().any(|d| {
            let l = d.to_lowercase().replace(' ', "");
            l.contains("minilab3") || l.contains("minilabmk3")
        });
        let want = match self.s.controller {
            1 => false,
            2 => true,
            _ => ml3,
        };
        if want != self.ml3_active {
            self.ml3_active = want;
            for a in self.p.cc_map.iter() {
                a.store(target::NONE, Relaxed);
            }
            if want {
                for (cc, t) in MINILAB3_MAP {
                    self.p.cc_map[cc as usize].store(t, Relaxed);
                }
                for i in 0..8u8 {
                    self.p.cc_map[(102 + i) as usize].store(target::PAD0 + i, Relaxed);
                }
            }
        }
        // wind controllers
        let wind_dev = devs.iter().find(|d| {
            let l = d.to_lowercase();
            l.contains("ewi") || l.contains("aerophone") || ["ae-30", "ae-20", "ae-10", "ae-05", "ae-01", "ae-5"].iter().any(|m| l.contains(m))
        });
        self.wind_name = wind_dev.cloned().unwrap_or_default();
        let wind = match self.s.wind {
            1 => false,
            2 => true,
            _ => wind_dev.is_some(),
        };
        if wind != self.wind_active {
            self.wind_active = wind;
            self.p.wind_on.store(wind, Relaxed);
            if wind && wind_dev.is_some() && !self.s.wind_setup_done {
                // first time: a breath-friendly sound and the staff in view
                self.s.wind_setup_done = true;
                if self.s.main == (0, 0) {
                    self.set_preset(false, (0, 66)); // GM Tenor Sax
                }
                if self.s.view == 0 {
                    self.s.view = 2;
                }
            }
        }
        // first time we see a MiniLab 3: switch to its layout + 25 keys
        if ml3 && !self.ml3_seen {
            self.ml3_seen = true;
            if self.s.controller == 0 && !self.s.ml3_setup_done {
                self.s.ml3_setup_done = true;
                self.s.layout = 1;
                self.set_kb_size(25);
            }
        }
    }

    fn speller(&self) -> Speller {
        if self.scale_on {
            Speller::for_scale(self.s.scale_root, &SCALES[self.s.scale_idx.min(SCALES.len() - 1)])
        } else {
            Speller::default_sharps()
        }
    }

    fn scale_mask(&self) -> u16 {
        if !self.scale_on {
            return 0;
        }
        theory::scale_mask(ROOTS[self.s.scale_root].1, &SCALES[self.s.scale_idx.min(SCALES.len() - 1)])
    }

    fn rebuild_walk(&mut self) {
        let sc = &SCALES[self.s.scale_idx.min(SCALES.len() - 1)];
        let root = 60 + ROOTS[self.s.scale_root].1;
        let mut up: Vec<u8> = sc.steps.iter().map(|s| root + s).collect();
        up.push(root + 12);
        let mut seq = up.clone();
        seq.extend(up.iter().rev().skip(1));
        self.walk_seq = seq;
        self.walk_pos = 0;
        self.walk_laps = 0;
        let m = self.scale_mask();
        self.p.scale_mask.store(m, Relaxed);
    }

    fn send_note(&self, key: u8, on: bool) {
        let _ = self.tx.send(Cmd::Midi(if on { [0x90, key, 100] } else { [0x80, key, 0] }));
    }

    fn load_song_path(&mut self, path: &std::path::Path) {
        match song::load(path) {
            Ok(s) => self.set_song(Some(Arc::new(s)), 0),
            Err(e) => self.song_err = Some(e),
        }
    }

    fn set_song(&mut self, s: Option<Arc<Song>>, user: u16) {
        self.song_err = None;
        self.song_user = user;
        self.p.song_user.store(user, Relaxed);
        let _ = self.tx.send(Cmd::LoadSong(s.clone()));
        self.song = s;
    }

    // ---------- event pump ----------
    fn pump(&mut self) {
        let now = now_s(&self.t0);
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                UiEvt::Note { key, vel, src, pos } => {
                    if vel > 0 && src == src::LIVE {
                        let speed = self.p.song_speed.get() as f64;
                        if let Some(g) = self.game.as_mut() {
                            if self.p.song_playing.load(Relaxed) && !g.finished {
                                g.press(key, pos, speed, now);
                            }
                        }
                    }
                    let si = (src as usize).min(2);
                    self.held[si][key as usize] = vel;
                    if vel > 0 {
                        self.bars.push(Bar { key, src, vel, start: now, end: None });
                        if src == src::LIVE {
                            self.judge(key, now);
                            self.follow(key);
                        }
                    } else if let Some(b) = self.bars.iter_mut().rev().find(|b| b.key == key && b.src == src && b.end.is_none()) {
                        b.end = Some(now);
                    }
                }
                UiEvt::AllOff { src } => {
                    let si = (src as usize).min(2);
                    self.held[si] = [0; 128];
                    for b in self.bars.iter_mut().filter(|b| b.src == src && b.end.is_none()) {
                        b.end = Some(now);
                    }
                }
                UiEvt::Program { bank, prog } => self.s.main = (bank, prog),
                UiEvt::Trash(_) | UiEvt::TrashSong(_) => {} // dropped here, off the audio thread
                UiEvt::Cc { cc, val } => {
                    self.cc_vals[cc as usize] = val;
                    self.cc_seen[cc as usize] = now;
                }
                UiEvt::Bend(v) => self.bend = v,
                UiEvt::Pad { note, vel } => {
                    if vel > 0 && (36..44).contains(&note) {
                        self.pad_hit[(note - 36) as usize] = now;
                    }
                }
                UiEvt::Browse(d) => self.step_preset(false, d as i32),
                UiEvt::Category(d) => {
                    let cur = self.category.map(|c| if c == 99 { -1 } else { c as i32 }).unwrap_or(-2);
                    let next = (cur + d as i32).clamp(-2, 15);
                    self.category = match next {
                        -2 => None,
                        -1 => Some(99),
                        n => Some(n as usize),
                    };
                }
            }
        }
        while let Ok(res) = self.load_rx.try_recv() {
            self.loading = None;
            match res {
                Ok(l) => {
                    self.presets = l.presets;
                    self.sf_current = Some(l.path.clone());
                    self.s.soundfont = Some(l.path.clone());
                    if !self.sf_paths.contains(&l.path) {
                        self.sf_paths.insert(0, l.path);
                    }
                    let _ = self.tx.send(Cmd::Synths(l.synths));
                    for (layer, bp) in [(false, self.s.main), (true, self.s.layer)] {
                        let ok = self.presets.iter().any(|p| (p.bank, p.prog) == bp);
                        let bp = if ok { bp } else { self.presets.iter().find(|p| p.bank < 128).map(|p| (p.bank, p.prog)).unwrap_or((0, 0)) };
                        self.set_preset(layer, bp);
                    }
                }
                Err(e) => self.load_err = Some(e),
            }
        }
        // forget old bars
        self.bars.retain(|b| b.end.map(|e| now - e < 30.0).unwrap_or(true));
    }

    fn judge(&mut self, key: u8, now: f64) {
        if !self.scale_on {
            return;
        }
        if self.walk && !self.walk_seq.is_empty() {
            let target = self.walk_seq[self.walk_pos];
            if key == target {
                self.right[key as usize] = now;
                self.walk_pos += 1;
                if self.walk_pos >= self.walk_seq.len() {
                    self.walk_pos = 0;
                    self.walk_laps += 1;
                }
            } else {
                self.wrong[key as usize] = now;
            }
        } else {
            let m = self.scale_mask();
            if m & (1 << (key % 12)) == 0 {
                self.wrong[key as usize] = now;
            } else {
                self.right[key as usize] = now;
            }
        }
    }

    fn keyboard_input(&mut self, ctx: &egui::Context) {
        let focused = ctx.input(|i| i.focused);
        if !focused && !self.keys_down.is_empty() {
            for (_, k) in self.keys_down.drain() {
                let _ = self.tx.send(Cmd::Midi([0x80, k, 0]));
            }
        }
        if ctx.wants_keyboard_input() {
            return;
        }
        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            let egui::Event::Key { key, pressed, repeat, modifiers, .. } = ev else { continue };
            if modifiers.ctrl || modifiers.command || modifiers.alt {
                continue;
            }
            if let Some((_, off)) = KEYMAP.iter().find(|(k, _)| *k == key) {
                if pressed && !repeat {
                    let note = ((self.s.kb_octave + 1) * 12 + *off as i32).clamp(0, 127) as u8;
                    self.keys_down.insert(key, note);
                    self.send_note(note, true);
                } else if !pressed {
                    if let Some(n) = self.keys_down.remove(&key) {
                        self.send_note(n, false);
                    }
                }
                continue;
            }
            if !pressed || repeat {
                continue;
            }
            match key {
                Key::Z => self.s.kb_octave = (self.s.kb_octave - 1).max(0),
                Key::X => self.s.kb_octave = (self.s.kb_octave + 1).min(8),
                Key::Space => {
                    let on = !self.p.drums_on.load(Relaxed);
                    self.p.drums_on.store(on, Relaxed);
                }
                Key::Enter => {
                    let _ = self.tx.send(Cmd::LoopButton);
                }
                Key::Backspace => {
                    let _ = self.tx.send(Cmd::LoopUndo);
                }
                Key::Escape => {
                    let _ = self.tx.send(Cmd::Panic);
                    let _ = self.tx.send(Cmd::LoopStop);
                }
                Key::ArrowLeft => self.step_preset(false, -1),
                Key::ArrowRight => self.step_preset(false, 1),
                Key::F1 => self.show_help = !self.show_help,
                Key::F2 => self.s.hide_modules = !self.s.hide_modules,
                Key::V => self.s.view = (self.s.view + 1) % 3,
                Key::F3 => self.s.theme = (self.s.theme + 1) % 3,
                _ => {}
            }
        }
    }

    fn autosave(&mut self, now: f64) {
        if now - self.last_save < 3.0 {
            return;
        }
        self.last_save = now;
        self.s.pull_from(&self.p);
        self.s.scale_on = self.scale_on;
        if let Ok(j) = serde_json::to_string(&self.s) {
            if j != self.saved_json {
                self.s.save();
                self.saved_json = j;
            }
        }
    }

    // ---------- header ----------
    fn preset_combo(&mut self, ui: &mut egui::Ui, layer: bool, width: f32) {
        let cur = if layer { self.s.layer } else { self.s.main };
        let name = self.preset_name(cur);
        let list = self.filtered();
        let mut pick = None;
        let size = if layer { 14.0 } else { 19.0 };
        egui::ComboBox::from_id_salt(if layer { "layer_preset" } else { "main_preset" })
            .selected_text(RichText::new(name).size(size).strong().color(if layer { ARP } else { fg() }))
            .width(width)
            .height(520.0)
            .show_ui(ui, |ui| {
                let mut fam = usize::MAX;
                for i in list {
                    let pr = &self.presets[i];
                    let f = sounds::family(pr);
                    if self.category.is_none() && f != fam && pr.bank == 0 {
                        fam = f;
                        ui.label(RichText::new(GM_FAMILIES[f]).size(11.0).color(text_dim()));
                    }
                    let fav = if self.s.favorites.contains(&(pr.bank, pr.prog)) { "★ " } else { "" };
                    let label = format!("{fav}{:03}  {}{}", pr.prog + 1, pr.name, if pr.bank > 0 { format!("  (bank {})", pr.bank) } else { String::new() });
                    if ui.selectable_label((pr.bank, pr.prog) == cur, label).clicked() {
                        pick = Some((pr.bank, pr.prog));
                    }
                }
            });
        if let Some(bp) = pick {
            self.set_preset(layer, bp);
        }
    }

    fn header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("NOODLE").size(24.0).strong().color(LIVE));
            ui.add_space(8.0);
            if ui.add(egui::Button::new(RichText::new("◀").size(18.0))).on_hover_text("Previous sound (←)").clicked() {
                self.step_preset(false, -1);
            }
            self.preset_combo(ui, false, 300.0);
            if ui.add(egui::Button::new(RichText::new("▶").size(18.0))).on_hover_text("Next sound (→)").clicked() {
                self.step_preset(false, 1);
            }
            let cur = self.s.main;
            let is_fav = self.s.favorites.contains(&cur);
            if ui
                .add(egui::Button::new(RichText::new(if is_fav { "★" } else { "☆" }).size(18.0).color(if is_fav { YOURS } else { text_dim() })).frame(false))
                .on_hover_text("Favourite this sound")
                .clicked()
            {
                if is_fav {
                    self.s.favorites.retain(|f| *f != cur);
                } else {
                    self.s.favorites.push(cur);
                }
            }
            let cat = match self.category {
                None => "All sounds".to_string(),
                Some(99) => "★ Favourites".to_string(),
                Some(f) => GM_FAMILIES[f].to_string(),
            };
            egui::ComboBox::from_id_salt("cat").selected_text(cat).width(130.0).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.category, None, "All sounds");
                ui.selectable_value(&mut self.category, Some(99), "★ Favourites");
                ui.separator();
                for (i, f) in GM_FAMILIES.iter().enumerate() {
                    ui.selectable_value(&mut self.category, Some(i), *f);
                }
            });
            ui.separator();
            let mut mode = self.p.mode.load(Relaxed);
            for (i, n) in ["Single", "Layer", "Split"].iter().enumerate() {
                if ui.selectable_label(mode == i as u8, RichText::new(*n).size(14.0)).on_hover_text(match i {
                    0 => "One sound across the keyboard",
                    1 => "Two sounds stacked on every key",
                    _ => "Second sound below the split point",
                }).clicked() {
                    mode = i as u8;
                }
            }
            self.p.mode.store(mode, Relaxed);
            if mode > 0 {
                if ui.small_button("◀").clicked() {
                    self.step_preset(true, -1);
                }
                self.preset_combo(ui, true, 200.0);
                if ui.small_button("▶").clicked() {
                    self.step_preset(true, 1);
                }
                let mut lv = self.p.layer_vol.get();
                if ui.add(egui::Slider::new(&mut lv, 0.0..=1.0).show_value(false)).on_hover_text("Second sound volume").changed() {
                    self.p.layer_vol.set(lv);
                }
                if mode == 2 {
                    let mut sp = self.p.split.load(Relaxed) as i32;
                    ui.label(RichText::new("split").color(text_dim()));
                    if ui
                        .add(egui::DragValue::new(&mut sp).range(24..=96).custom_formatter(|v, _| theory::note_name_oct(v as u8)))
                        .on_hover_text("Notes below this go to the second sound")
                        .changed()
                    {
                        self.p.split.store(sp as u8, Relaxed);
                    }
                }
            }
        });
        ui.horizontal(|ui| {
            let mut tr = self.p.transpose.load(Relaxed);
            ui.label(RichText::new("Transpose").color(text_dim()));
            if ui.small_button("−8va").clicked() {
                tr -= 12;
            }
            if ui.small_button("−").clicked() {
                tr -= 1;
            }
            ui.label(RichText::new(format!("{:+}", tr)).strong().monospace());
            if ui.small_button("+").clicked() {
                tr += 1;
            }
            if ui.small_button("+8va").clicked() {
                tr += 12;
            }
            self.p.transpose.store(tr.clamp(-36, 36), Relaxed);
            ui.separator();
            ui.label(RichText::new("Tempo").color(text_dim()));
            let mut bpm = self.p.bpm.get();
            if ui.add(egui::DragValue::new(&mut bpm).range(40.0..=260.0).speed(0.4).fixed_decimals(0).suffix(" bpm")).changed() {
                self.p.bpm.set(bpm);
            }
            if ui.button("Tap").on_hover_text("Tap 3+ times").clicked() {
                let t = now_s(&self.t0);
                self.taps.retain(|x| t - x < 2.5);
                self.taps.push(t);
                if self.taps.len() >= 3 {
                    let d = (self.taps[self.taps.len() - 1] - self.taps[0]) / (self.taps.len() - 1) as f64;
                    self.p.bpm.set((60.0 / d).clamp(40.0, 260.0) as f32);
                }
            }
            ui.separator();
            ui.label(RichText::new("Bank").color(text_dim()));
            let cur = self.sf_current.as_ref().and_then(|p| p.file_name()).map(|n| n.to_string_lossy().to_string());
            let txt = self.loading.clone().map(|l| format!("Loading {l}…")).or(cur).unwrap_or_else(|| "none".into());
            let mut pick = None;
            egui::ComboBox::from_id_salt("sf").selected_text(txt).width(170.0).show_ui(ui, |ui| {
                for p in &self.sf_paths {
                    let n = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    let mb = std::fs::metadata(p).map(|m| m.len() / 1_000_000).unwrap_or(0);
                    if ui.selectable_label(Some(p) == self.sf_current.as_ref(), format!("{n}  ({mb} MB)")).on_hover_text(p.display().to_string()).clicked() {
                        pick = Some(p.clone());
                    }
                }
                ui.separator();
                ui.label(RichText::new("Drop any .sf2 on the window to add it").size(11.0).color(text_dim()));
            });
            if let Some(p) = pick {
                self.load_sf(p);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("?").on_hover_text("Keys & shortcuts (F1)").clicked() {
                    self.show_help = !self.show_help;
                }
                let (icon, tip) = match self.s.theme {
                    1 => ("☀", "Light mode — click for follow-system"),
                    2 => ("◐", "Following your system theme — click for dark"),
                    _ => ("🌙", "Dark mode — click for light"),
                };
                if ui.button(icon).on_hover_text(format!("{tip}  (F3)")).clicked() {
                    self.s.theme = (self.s.theme + 1) % 3;
                }
                let cpu = self.p.cpu.get();
                ui.label(RichText::new(format!("DSP {:.0}%", cpu * 100.0)).size(11.0).color(if cpu > 0.7 { BAD } else { text_dim() }));
                let ok = self.p.audio_ok.load(Relaxed);
                let frames = self.p.audio_frames.load(Relaxed);
                let sr = self.p.sample_rate.load(Relaxed).max(1);
                let ms = frames as f32 * 1000.0 / sr as f32;
                let label = match (&self.audio_info, ok) {
                    (Ok(a), true) => RichText::new(format!("🔊 {a} · {ms:.1} ms ▾")).size(11.0).color(if ms > 12.0 { YOURS } else { text_dim() }),
                    (Ok(_), false) => RichText::new("🔇 Audio stalled ▾").size(11.0).color(BAD),
                    (Err(_), _) => RichText::new("🔇 No audio output ▾").size(11.0).color(BAD),
                };
                let mut pick = None;
                ui.menu_button(label, |ui| {
                    ui.label(RichText::new("Latency (buffer size)").strong());
                    ui.label(RichText::new("Smaller = tighter feel, more CPU. If you hear crackles, go one step bigger.").size(11.0).color(text_dim()));
                    for f in [32u32, 64, 128, 256, 512, 1024] {
                        let txt = format!("{f} samples  ≈ {:.1} ms", f as f32 * 1000.0 / sr as f32);
                        if ui.selectable_label(self.s.latency == f, txt).clicked() {
                            pick = Some(f);
                            ui.close();
                        }
                    }
                    ui.separator();
                    match &self.audio_info {
                        Ok(a) => ui.label(RichText::new(format!("Device: {a}\nSound card asks for {frames} samples at a time.")).size(11.0).color(text_dim())),
                        Err(e) => ui.label(RichText::new(e).size(11.0).color(BAD)),
                    };
                    if ui.button("↻ Reconnect audio").clicked() {
                        pick = Some(self.s.latency);
                        ui.close();
                    }
                });
                if let Some(f) = pick {
                    self.restart_audio(f);
                }
                let all = self.devices.lock().map(|d| d.clone()).unwrap_or_default();
                let on: Vec<String> = all.iter().filter(|n| !n.starts_with('-')).cloned().collect();
                let label = if on.is_empty() {
                    RichText::new("○ No MIDI keyboard yet ▾").size(12.0).color(text_dim())
                } else {
                    RichText::new(format!("● {} ▾", on.join(", "))).size(12.0).color(GOOD)
                };
                let mut toggled = None;
                ui.menu_button(label, |ui| {
                    ui.label(RichText::new("MIDI inputs (untick to ignore)").size(11.0).color(text_dim()));
                    if all.is_empty() {
                        ui.label("Nothing plugged in yet.");
                    }
                    for n in &all {
                        let name = n.trim_start_matches('-').to_string();
                        let mut enabled = !n.starts_with('-');
                        if ui.checkbox(&mut enabled, &name).changed() {
                            toggled = Some((name, enabled));
                        }
                    }
                });
                if let Some((name, enabled)) = toggled {
                    self.s.midi_off.retain(|x| *x != name);
                    if !enabled {
                        self.s.midi_off.push(name);
                    }
                    if let Ok(mut b) = self.blocked.lock() {
                        *b = self.s.midi_off.clone();
                    }
                }
            });
        });
    }

    // ---------- module cards ----------
    fn modules(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::horizontal().auto_shrink([false, true]).show(ui, |ui| {
            ui.horizontal_top(|ui| {
                if self.s.layout == 1 {
                    self.minilab_card(ui);
                } else {
                    self.fx_card(ui);
                }
                if self.wind_active {
                    self.wind_card(ui);
                }
                self.loop_card(ui);
                self.drum_card(ui);
                self.arp_card(ui);
                self.drone_card(ui);
            });
        });
    }

    fn fx_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p.clone();
        card(ui, "SOUND", 262.0, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let mut v = p.reverb.get();
                if knob(ui, "Reverb", &mut v, 0.35, GOOD, pct) {
                    p.reverb.set(v)
                }
                let mut v = p.chorus.get();
                if knob(ui, "Chorus", &mut v, 0.0, GOOD, pct) {
                    p.chorus.set(v)
                }
                let mut v = p.delay_mix.get();
                if knob(ui, "Delay", &mut v, 0.0, ARP, pct) {
                    p.delay_mix.set(v)
                }
                let mut v = p.delay_fb.get();
                if knob(ui, "Repeats", &mut v, 0.35, ARP, pct) {
                    p.delay_fb.set(v)
                }
            });
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                let mut v = p.cutoff.get();
                if knob(ui, "Filter", &mut v, 1.0, LIVE, |v| if v > 0.995 { "open".into() } else { format!("{:.0} Hz", 40.0 * 500f32.powf(v)) }) {
                    p.cutoff.set(v)
                }
                let mut v = p.resonance.get();
                if knob(ui, "Reso", &mut v, 0.1, LIVE, pct) {
                    p.resonance.set(v)
                }
                let mut v = p.drive.get();
                if knob(ui, "Drive", &mut v, 0.0, BAD, pct) {
                    p.drive.set(v)
                }
                let mut v = p.master.get();
                if knob(ui, "Volume", &mut v, 0.8, fg(), pct) {
                    p.master.set(v)
                }
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Delay time").size(11.0).color(text_dim()));
                let mut dt = p.delay_time.load(Relaxed) as usize;
                for (i, (n, _)) in DELAY_TIMES.iter().enumerate() {
                    if ui.selectable_label(dt == i, RichText::new(*n).size(11.0)).clicked() {
                        dt = i;
                    }
                }
                p.delay_time.store(dt as u8, Relaxed);
            });
        });
    }

    /// Controller-shaped panel laid out like the MiniLab 3's top surface so every
    /// knob, fader, pad and strip sits where it is on the hardware.
    fn minilab_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p.clone();
        let now = now_s(&self.t0);
        let mapped = self.ml3_active;
        let title = if mapped { "MINILAB 3  ·  mapped 1:1" } else { "MINILAB 3 LAYOUT  ·  not detected (Controller: MiniLab 3 to force)" };
        let cur_name = self.preset_name(self.s.main);
        let cat = match self.category {
            None => "All sounds".to_string(),
            Some(99) => "Favourites".to_string(),
            Some(f) => GM_FAMILIES[f].to_string(),
        };
        let mut browse = 0i32;
        let mut pad_press: Option<(u8, bool)> = None;
        card_h(ui, title, 610.0, 196.0, |ui| {
            let (area, _) = ui.allocate_exact_size(Vec2::new(610.0, 176.0), Sense::hover());
            let o = area.min;
            let painter = ui.painter().clone();
            let hot = |cc: u8| now - self.cc_seen[cc as usize] < 0.5;
            // --- left column: shift/hold, oct, touch strips
            let btn = |r: Rect, label: &str, lit: bool| {
                painter.rect_filled(r, 4.0, if lit { fg() } else { card_hi() });
                painter.text(r.center(), Align2::CENTER_CENTER, label, FontId::proportional(10.5), if lit { Color32::BLACK } else { text_dim() });
            };
            btn(Rect::from_min_size(o + Vec2::new(0.0, 0.0), Vec2::new(40.0, 18.0)), "Shift", hot(9) && self.cc_vals[9] > 0);
            btn(Rect::from_min_size(o + Vec2::new(46.0, 0.0), Vec2::new(40.0, 18.0)), "Hold", false);
            btn(Rect::from_min_size(o + Vec2::new(0.0, 24.0), Vec2::new(40.0, 18.0)), "Oct −", false);
            btn(Rect::from_min_size(o + Vec2::new(46.0, 24.0), Vec2::new(40.0, 18.0)), "Oct +", false);
            let strip_l = Rect::from_min_size(o + Vec2::new(6.0, 50.0), Vec2::new(28.0, 120.0));
            let strip_r = Rect::from_min_size(o + Vec2::new(52.0, 50.0), Vec2::new(28.0, 120.0));
            for r in [strip_l, strip_r] {
                painter.rect_filled(r, 5.0, gray(38));
            }
            let by = strip_l.center().y - (self.bend as f32 / 8192.0) * (strip_l.height() / 2.0 - 6.0);
            painter.rect_filled(Rect::from_center_size(Pos2::new(strip_l.center().x, by), Vec2::new(24.0, 6.0)), 2.0, if self.bend != 0 { LIVE } else { gray(140) });
            let mv = self.cc_vals[1] as f32 / 127.0;
            painter.rect_filled(Rect::from_min_max(Pos2::new(strip_r.left(), strip_r.bottom() - mv * strip_r.height()), strip_r.max), 5.0, ARP.gamma_multiply(0.8));
            painter.text(strip_l.center_bottom() + Vec2::new(0.0, -8.0), Align2::CENTER_CENTER, "bend", FontId::proportional(9.5), text_dim());
            painter.text(strip_r.center_bottom() + Vec2::new(0.0, -8.0), Align2::CENTER_CENTER, "mod", FontId::proportional(9.5), fg());
            // --- display + main encoder
            let disp = Rect::from_min_size(o + Vec2::new(98.0, 0.0), Vec2::new(84.0, 118.0));
            painter.rect_filled(disp, 6.0, Color32::from_rgb(8, 9, 11));
            let screen = Rect::from_min_size(disp.min + Vec2::new(6.0, 6.0), Vec2::new(72.0, 46.0));
            painter.rect_filled(screen, 3.0, Color32::from_rgb(18, 22, 28));
            let (mut l1, mut l2) = (String::new(), String::new());
            for w in cur_name.split_whitespace() {
                if l2.is_empty() && l1.chars().count() + w.chars().count() < 12 {
                    if !l1.is_empty() {
                        l1.push(' ');
                    }
                    l1.push_str(w);
                } else {
                    if !l2.is_empty() {
                        l2.push(' ');
                    }
                    l2.push_str(w);
                }
            }
            let l2: String = l2.chars().take(13).collect();
            let (l1, l2) = (l1.as_str(), l2.as_str());
            painter.text(screen.min + Vec2::new(4.0, 4.0), Align2::LEFT_TOP, cat.chars().take(12).collect::<String>(), FontId::proportional(8.5), text_dim());
            painter.text(screen.min + Vec2::new(4.0, 17.0), Align2::LEFT_TOP, l1.trim(), FontId::proportional(11.0), Color32::WHITE);
            painter.text(screen.min + Vec2::new(4.0, 30.0), Align2::LEFT_TOP, l2.trim(), FontId::proportional(11.0), Color32::WHITE);
            let enc_c = Pos2::new(disp.center().x, disp.bottom() - 32.0);
            let enc_r = Rect::from_center_size(enc_c, Vec2::splat(44.0));
            let enc = ui.interact(enc_r, ui.id().with("ml3enc"), Sense::click_and_drag());
            painter.circle_filled(enc_c, 18.0, if hot(114) || hot(112) || enc.hovered() { gray(70) } else { gray(40) });
            painter.circle_stroke(enc_c, 18.0, Stroke::new(1.0, gray(90)));
            if enc.dragged() {
                let d = enc.drag_delta();
                let acc = ui.data_mut(|m| {
                    let a = m.get_temp_mut_or_default::<f32>(ui.id().with("ml3acc"));
                    *a += d.x - d.y;
                    let steps = (*a / 24.0).trunc();
                    *a -= steps * 24.0;
                    steps as i32
                });
                browse += acc;
            }
            let sc = if enc.hovered() { ui.input(|i| i.raw_scroll_delta.y) } else { 0.0 };
            if sc.abs() > 0.5 {
                browse += if sc > 0.0 { -1 } else { 1 };
            }
            enc.on_hover_text("Main encoder: browse sounds (Shift + turn on the hardware = category)");
            // --- 8 knobs, 2 rows of 4
            for (i, cc) in ML3_KNOBS.iter().enumerate() {
                let t = p.cc_map[*cc as usize].load(Relaxed);
                let t = if t == target::NONE { MINILAB3_MAP[i].1 } else { t };
                let x = 192.0 + (i % 4) as f32 * 58.0;
                let y = (i / 4) as f32 * 60.0;
                let r = Rect::from_min_size(o + Vec2::new(x, y), Vec2::new(56.0, 62.0));
                if hot(*cc) {
                    painter.circle_filled(r.center_top() + Vec2::new(0.0, 24.0), 23.0, LIVE.gamma_multiply(0.25));
                }
                ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
                    let mut v = tget(&p, t);
                    let col = match t {
                        target::FILTER | target::RESO => LIVE,
                        target::DRIVE => BAD,
                        target::DELAY | target::REPEATS | target::ARP_GATE => ARP,
                        _ => GOOD,
                    };
                    if knob(ui, target::name(t), &mut v, tget(&Params::new(), t), col, pct) {
                        tset(&p, t, v);
                    }
                });
            }
            // --- 4 faders
            for (i, cc) in ML3_FADERS.iter().enumerate() {
                let t = MINILAB3_MAP[8 + i].1;
                let r = Rect::from_min_size(o + Vec2::new(434.0 + i as f32 * 42.0, 0.0), Vec2::new(36.0, 120.0));
                ui.scope_builder(UiBuilder::new().max_rect(r), |ui| {
                    let mut v = tget(&p, t);
                    if fader(ui, target::name(t), &mut v, GOOD, hot(*cc)) {
                        tset(&p, t, v);
                    }
                });
            }
            // --- 8 pads under the knobs/faders
            for i in 0..8usize {
                let r = Rect::from_min_size(o + Vec2::new(98.0 + i as f32 * 63.0, 128.0), Vec2::new(55.0, 44.0));
                let resp = ui.interact(r, ui.id().with(("ml3pad", i)), Sense::click_and_drag());
                let age = (now - self.pad_hit[i]) as f32;
                let glow = (1.0 - age / 0.35).clamp(0.0, 1.0);
                painter.rect_filled(r, 6.0, Color32::from_rgb(14, 15, 18));
                painter.rect_stroke(r, 6.0, Stroke::new(2.0 + 2.0 * glow, YOURS.gamma_multiply(0.35 + 0.65 * glow)), StrokeKind::Inside);
                if glow > 0.0 {
                    painter.rect_filled(r.shrink(3.0), 5.0, YOURS.gamma_multiply(0.5 * glow));
                }
                painter.text(r.center_bottom() + Vec2::new(0.0, -8.0), Align2::CENTER_CENTER, format!("{}", i + 1), FontId::proportional(10.0), text_dim());
                if resp.drag_started() || resp.is_pointer_button_down_on() && !resp.dragged() && resp.clicked() {
                    pad_press = Some((i as u8, true));
                }
                if resp.drag_stopped() {
                    pad_press = Some((i as u8, false));
                }
                resp.on_hover_text("Pad: drum hit (pads send notes on channel 10)");
            }
        });
        if browse != 0 {
            self.step_preset(false, browse.signum());
        }
        if let Some((i, on)) = pad_press {
            let note = 36 + i;
            let _ = self.tx.send(Cmd::Midi(if on { [0x99, note, 110] } else { [0x89, note, 0] }));
        }
    }

    fn select_pipe(&mut self, i: usize) {
        let pp = &crate::pipes::PIPES[i.min(crate::pipes::PIPES.len() - 1)];
        self.s.pipe_idx = i;
        self.s.pipe_drones = pp.drones.iter().map(|d| d.2).collect();
        self.s.pipe_cont = i != 4; // Northumbrian: closed fingering — silence between notes is part of the style
        self.scale_on = true;
        self.s.scale_idx = theory::scale_index(pp.scale);
        self.s.scale_root = ROOTS.iter().position(|r| r.1 == pp.tonic % 12).unwrap_or(9);
        self.rebuild_walk();
    }

    fn set_drone_mode(&mut self, m: u8) {
        self.s.drone_mode = m;
        match m {
            1 => {
                let i = self.s.pipe_idx;
                self.select_pipe(i);
                self.drone_running = true;
            }
            2 => self.drone_running = true,
            3 => {
                self.set_preset(false, (0, 20)); // GM Reed Organ ≈ harmonium
                self.scale_on = true;
                self.rebuild_walk();
            }
            _ => self.drone_running = false,
        }
    }

    /// Push the drone/pipe/harmonium state to the engine every frame (cheap atomics).
    fn apply_drones(&mut self) {
        let p = &self.p;
        let mode = self.s.drone_mode;
        let root_pc = ROOTS[self.s.scale_root].1 as i32;
        let mut cents = [i32::MIN; 13];
        let mut tone = 3u8;
        match mode {
            1 => {
                let pp = &crate::pipes::PIPES[self.s.pipe_idx.min(crate::pipes::PIPES.len() - 1)];
                // tonic follows the Scale key, staying in the pipe's register
                let base = pp.tonic as i32;
                let mut tonic = base - base.rem_euclid(12) + root_pc;
                if tonic - base > 6 {
                    tonic -= 12;
                } else if base - tonic > 6 {
                    tonic += 12;
                }
                p.pipe_tonic.store(tonic.clamp(0, 127) as u8, Relaxed);
                for (i, d) in pp.drones.iter().enumerate().take(4) {
                    if i < 4 && self.s.pipe_drones.get(i).copied().unwrap_or(d.2) {
                        cents[i] = (tonic + d.0 as i32) * 100 + self.s.pipe_tune;
                    }
                }
                tone = pp.tone;
            }
            2 => {
                let sa = if self.s.shruti_low { 48 } else { 60 } + root_pc;
                for semi in 0..=12 {
                    if self.s.shruti_switches & (1 << semi) != 0 {
                        cents[semi] = (sa + semi as i32) * 100;
                    }
                }
            }
            3 => {
                // harmonium drone stops: Sa, Pa, Ma, Sa'
                let sa = 48 + root_pc;
                for (bit, semi) in [(3u8, 0i32), (4, 7), (5, 5), (6, 12)] {
                    if self.s.harm_stops & (1 << bit) != 0 {
                        cents[semi as usize] = (sa + semi) * 100;
                    }
                }
            }
            _ => {}
        }
        for i in 0..13 {
            p.drone_cents[i].store(cents[i], Relaxed);
        }
        p.chanter_voice.store(self.s.pipe_idx.min(crate::pipes::CHANTERS.len() - 1) as u8, Relaxed);
        p.chanter_level.set(self.s.chanter_level);
        p.harm_octaves.store(self.s.harm_stops & 0b111, Relaxed);
        if mode == 3 {
            self.drone_running = self.s.harm_stops & 0b111_1000 != 0;
        }
        p.drone_tone.store(tone, Relaxed);
        p.drone_level.set(self.s.drone_level);
        p.drone_on.store(self.drone_running && mode != 0, Relaxed);
        p.pipe_mode.store(mode == 1, Relaxed);
        p.pipe_just.store(self.s.pipe_just, Relaxed);
        p.pipe_tune.store(self.s.pipe_tune, Relaxed);
        p.pipe_continuous.store(self.s.pipe_cont, Relaxed);
        p.chord_on.store(mode == 3, Relaxed);
        p.chord_kind.store(self.s.chord_kind, Relaxed);
    }

    fn drone_card(&mut self, ui: &mut egui::Ui) {
        let title = match self.s.drone_mode {
            1 => "DRONE · BAGPIPES",
            2 => "DRONE · SHRUTI BOX",
            3 => "DRONE · HARMONIUM",
            _ => "DRONE",
        };
        card(ui, title, 300.0, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for (i, n) in ["Off", "Bagpipes", "Shruti", "Harmonium"].iter().enumerate() {
                    if ui.selectable_label(self.s.drone_mode == i as u8, RichText::new(*n).size(12.5)).clicked() && self.s.drone_mode != i as u8 {
                        self.set_drone_mode(i as u8);
                    }
                }
            });
            let key = ROOTS[self.s.scale_root].0;
            match self.s.drone_mode {
                0 => {
                    ui.add_space(6.0);
                    ui.label(RichText::new("Bagpipes: drones + a just-tuned chanter that never drops out.\nShruti: Sa–Pa drones to sing with.\nHarmonium: one key = the right chord in your Scale key.").size(11.5).color(text_dim()));
                }
                1 => {
                    let pp = &crate::pipes::PIPES[self.s.pipe_idx.min(crate::pipes::PIPES.len() - 1)];
                    let tonic = self.p.pipe_tonic.load(Relaxed);
                    if self.s.pipe_drones.len() != pp.drones.len() {
                        self.s.pipe_drones = pp.drones.iter().map(|d| d.2).collect();
                    }
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.set_width(200.0);
                            if ui.button(RichText::new(format!("{} ▾", pp.name)).strong()).on_hover_text("Choose a bagpipe").clicked() {
                                self.pipe_picker = true;
                            }
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
                                for (i, d) in pp.drones.iter().enumerate() {
                                    let on = self.s.pipe_drones[i];
                                    let nn = theory::note_name_oct((tonic as i32 + d.0 as i32).clamp(0, 127) as u8);
                                    let txt = RichText::new(format!("{} {}", d.1, nn)).size(11.0).color(if on { Color32::BLACK } else { fg() });
                                    if ui.add(egui::Button::new(txt).fill(if on { GOOD } else { card_hi() }).corner_radius(9.0)).on_hover_text("Drone on/off").clicked() {
                                        self.s.pipe_drones[i] = !on;
                                    }
                                }
                            });
                            ui.checkbox(&mut self.s.pipe_just, "Just tuning").on_hover_text("Chanter notes tuned pure against the drones (5-limit just intonation) instead of piano tuning");
                            ui.checkbox(&mut self.s.pipe_cont, "Never stops").on_hover_text("Chanter keeps sounding with all fingers up, like real pipes (a wind controller's breath still stops it)");
                        });
                        let (r, resp) = ui.allocate_exact_size(Vec2::new(78.0, 92.0), Sense::click());
                        ui.painter().rect_filled(r, 8.0, deep());
                        crate::pipe_art::draw(ui.painter(), r.shrink(4.0), self.s.pipe_idx);
                        if resp.on_hover_text(pp.note).clicked() {
                            self.pipe_picker = true;
                        }
                    });
                }
                2 => {
                    ui.horizontal(|ui| {
                        let mut preset = None;
                        egui::ComboBox::from_id_salt("shruti").selected_text("Presets").width(96.0).show_ui(ui, |ui| {
                            for (i, (n, _)) in crate::pipes::SHRUTI.iter().enumerate() {
                                if ui.selectable_label(false, *n).clicked() {
                                    preset = Some(i);
                                }
                            }
                        });
                        if let Some(i) = preset {
                            self.s.shruti_switches = crate::pipes::SHRUTI[i].1.iter().fold(0u16, |m, r| m | (1 << *r));
                        }
                        ui.label(RichText::new(format!("Sa = {key}")).strong().color(GOOD)).on_hover_text("Follows the Scale key");
                        ui.selectable_value(&mut self.s.shruti_low, true, "Low");
                        ui.selectable_value(&mut self.s.shruti_low, false, "Mid");
                        let lab = if self.s.sargam { "Sa Re" } else { "C D" };
                        ui.toggle_value(&mut self.s.sargam, lab).on_hover_text("Lever labels: sargam or note names");
                    });
                    let labels: [String; 13] = std::array::from_fn(|semi| {
                        if self.s.sargam {
                            ["Sa", "re", "Re", "ga", "Ga", "Ma", "Ma'", "Pa", "dha", "Dha", "ni", "Ni", "Sa'"][semi].to_string()
                        } else {
                            self.speller().name(((ROOTS[self.s.scale_root].1 as usize + semi) % 12) as u8)
                        }
                    });
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(280.0, 104.0), Sense::click());
                    let rects = crate::pipe_art::shruti_lever_rects(r);
                    let hover = resp.hover_pos().and_then(|pos| rects.iter().position(|lr| lr.expand(3.0).contains(pos)));
                    crate::pipe_art::draw_shruti(ui.painter(), r, self.s.shruti_switches, &labels, hover, fg());
                    if resp.clicked() {
                        if let Some(i) = hover {
                            self.s.shruti_switches ^= 1 << crate::pipe_art::SHRUTI_LEVERS[i];
                            self.drone_running = true;
                        }
                    }
                    resp.on_hover_text("Flip a lever to open that reed");
                }
                _ => {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 3.0;
                        for (i, n) in crate::pipes::CHORD_TYPES.iter().enumerate() {
                            if ui.selectable_label(self.s.chord_kind == i as u8, RichText::new(*n).size(11.5)).clicked() {
                                self.s.chord_kind = i as u8;
                            }
                        }
                    });
                    let mut lit = [false; 128];
                    for src_h in &self.held {
                        for (k, v) in src_h.iter().enumerate() {
                            if *v > 0 {
                                lit[k] = true;
                            }
                        }
                    }
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(280.0, 104.0), Sense::click());
                    let rects = crate::pipe_art::harmonium_stop_rects(r);
                    let hover = resp.hover_pos().and_then(|pos| rects.iter().position(|sr| sr.contains(pos)));
                    crate::pipe_art::draw_harmonium(ui.painter(), r, self.s.harm_stops, &lit, hover, fg());
                    if resp.clicked() {
                        if let Some(i) = hover {
                            self.s.harm_stops ^= 1 << i;
                        }
                    }
                    resp.on_hover_text("Pull a stop: Bass / Male / Female reeds, or Sa / Pa / Ma / Sa' drones");
                }
            }
            if self.s.drone_mode != 0 {
                ui.horizontal(|ui| {
                    let mut lv = self.s.drone_level;
                    if knob(ui, "Drones", &mut lv, 0.6, GOOD, pct) {
                        self.s.drone_level = lv;
                    }
                    if self.s.drone_mode == 1 {
                        let mut cl = self.s.chanter_level;
                        if knob(ui, "Chanter", &mut cl, 0.7, LIVE, pct) {
                            self.s.chanter_level = cl;
                        }
                        let mut t = (self.s.pipe_tune as f32 + 100.0) / 200.0;
                        if knob(ui, "Tune", &mut t, 0.5, LIVE, |v| format!("{:+.0} ¢", v * 200.0 - 100.0)) {
                            self.s.pipe_tune = (t * 200.0 - 100.0).round() as i32;
                        }
                    }
                    if self.s.drone_mode != 3 {
                        let on = self.drone_running;
                        let lbl = match (on, self.s.drone_mode) {
                            (false, 1) => "▶ STRIKE IN",
                            (false, _) => "▶ START",
                            (true, _) => "■ STOP",
                        };
                        if big_toggle(ui, on, lbl, GOOD, Vec2::new(110.0, 30.0)).on_hover_text("Drones on / off").clicked() {
                            self.drone_running = !on;
                            if on {
                                let _ = self.tx.send(Cmd::Panic); // also silences a sustaining chanter
                            }
                        }
                    }
                });
            }
        });
    }

    fn wind_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p.clone();
        let title = if self.wind_name.is_empty() { "WIND".to_string() } else { format!("WIND · {}", midi_short(&self.wind_name)) };
        card(ui, &title, 250.0, |ui| {
            ui.horizontal_top(|ui| {
                // breath meter
                let (r, _) = ui.allocate_exact_size(Vec2::new(30.0, 150.0), Sense::hover());
                let pnt = ui.painter();
                pnt.rect_filled(r, 6.0, gray(35));
                let b = p.breath_val.get().clamp(0.0, 1.0);
                let fill = Rect::from_min_max(Pos2::new(r.left(), r.bottom() - b * r.height()), r.max);
                pnt.rect_filled(fill, 6.0, ARP.lerp_to_gamma(LIVE, b));
                pnt.text(r.center_bottom() + Vec2::new(0.0, -8.0), Align2::CENTER_CENTER, "air", FontId::proportional(10.0), fg());
                ui.vertical(|ui| {
                    let names: [(u8, &str); 5] = [(0, "Auto"), (2, "CC2 Breath"), (11, "CC11 Expression"), (7, "CC7 Volume"), (200, "Aftertouch")];
                    let mut src = p.breath_src.load(Relaxed);
                    let locked = p.breath_locked.load(Relaxed);
                    let cur = names.iter().find(|n| n.0 == src).map(|n| n.1).unwrap_or("Auto");
                    let label = if src == 0 && locked != 0 {
                        format!("Auto → {}", names.iter().find(|n| n.0 == locked).map(|n| n.1).unwrap_or("?"))
                    } else {
                        cur.to_string()
                    };
                    egui::ComboBox::from_id_salt("breath_src").selected_text(label).width(170.0).show_ui(ui, |ui| {
                        for (id, n) in names {
                            ui.selectable_value(&mut src, id, n);
                        }
                    });
                    p.breath_src.store(src, Relaxed);
                    ui.horizontal(|ui| {
                        let mut c = p.breath_curve.get();
                        if knob(ui, "Response", &mut c, 0.5, ARP, |v| if v < 0.4 { "soft".into() } else if v > 0.6 { "hard".into() } else { "linear".into() }) {
                            p.breath_curve.set(c);
                        }
                        let mut f = p.breath_filter.get();
                        if knob(ui, "Air→Filter", &mut f, 0.3, LIVE, pct) {
                            p.breath_filter.set(f);
                        }
                    });
                    ui.label(RichText::new("Breath drives expression; blowing harder also opens the filter.").size(10.5).color(text_dim()));
                    ui.label(RichText::new("Aerophone: MIDI CONTROL on, 'MIDI Ctrl Sound' Off to silence its own sounds.").size(10.5).color(text_dim()));
                });
            });
        });
    }

    fn arp_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p.clone();
        card(ui, "ARPEGGIATOR", 190.0, |ui| {
            ui.horizontal(|ui| {
                let on = p.arp_on.load(Relaxed);
                if big_toggle(ui, on, if on { "ARP ON" } else { "ARP OFF" }, ARP, Vec2::new(90.0, 30.0)).clicked() {
                    p.arp_on.store(!on, Relaxed);
                }
                let mut latch = p.arp_latch.load(Relaxed);
                if ui.checkbox(&mut latch, "Latch").on_hover_text("Keeps playing after you let go").changed() {
                    p.arp_latch.store(latch, Relaxed);
                }
            });
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
                let mut m = p.arp_mode.load(Relaxed) as usize;
                for (i, n) in ARP_MODES.iter().enumerate() {
                    if ui.selectable_label(m == i, RichText::new(*n).size(11.5)).clicked() {
                        m = i;
                    }
                }
                p.arp_mode.store(m as u8, Relaxed);
            });
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
                let mut r = p.arp_rate.load(Relaxed) as usize;
                for (i, (n, _)) in ARP_RATES.iter().enumerate() {
                    if ui.selectable_label(r == i, RichText::new(*n).size(11.5)).clicked() {
                        r = i;
                    }
                }
                p.arp_rate.store(r as u8, Relaxed);
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("Oct").size(11.5).color(text_dim()));
                let mut o = p.arp_oct.load(Relaxed);
                for i in 1..=4u8 {
                    if ui.selectable_label(o == i, RichText::new(i.to_string()).size(11.5)).clicked() {
                        o = i;
                    }
                }
                p.arp_oct.store(o, Relaxed);
                let mut g = p.arp_gate.get();
                if knob(ui, "Gate", &mut g, 0.6, ARP, pct) {
                    p.arp_gate.set(g.max(0.05));
                }
            });
        });
    }

    fn drum_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p.clone();
        card(ui, "DRUMS", 500.0, |ui| {
            ui.horizontal(|ui| {
                let on = p.drums_on.load(Relaxed);
                if big_toggle(ui, on, if on { "■ STOP" } else { "▶ PLAY" }, GOOD, Vec2::new(78.0, 26.0)).on_hover_text("Space").clicked() {
                    p.drums_on.store(!on, Relaxed);
                }
                let pat_name = PATTERNS.get(self.s.drum_pattern).map(|p| p.0).unwrap_or("Custom");
                let mut pick = None;
                egui::ComboBox::from_id_salt("pattern").selected_text(pat_name).width(150.0).show_ui(ui, |ui| {
                    for (i, (n, _, _)) in PATTERNS.iter().enumerate() {
                        if ui.selectable_label(i == self.s.drum_pattern, *n).clicked() {
                            pick = Some(i);
                        }
                    }
                });
                if let Some(i) = pick {
                    self.s.drum_pattern = i;
                    let (_, rows, len) = PATTERNS[i];
                    for r in 0..DRUM_ROWS {
                        p.drum_rows[r].store(rows[r], Relaxed);
                    }
                    p.drum_len.store(len, Relaxed);
                }
                let kit = p.drum_kit.load(Relaxed);
                let kname = KITS.iter().find(|k| k.1 == kit).map(|k| k.0).unwrap_or("Kit");
                egui::ComboBox::from_id_salt("kit").selected_text(kname).width(100.0).show_ui(ui, |ui| {
                    for (n, prog) in KITS {
                        if ui.selectable_label(kit == *prog, *n).clicked() {
                            p.drum_kit.store(*prog, Relaxed);
                        }
                    }
                });
                let mut len = p.drum_len.load(Relaxed);
                for l in [12u8, 16] {
                    if ui.selectable_label(len == l, RichText::new(format!("{l}")).size(11.0)).on_hover_text(if l == 12 { "12 steps: 6/8, 3/4" } else { "16 steps: 4/4" }).clicked() {
                        len = l;
                    }
                }
                p.drum_len.store(len, Relaxed);
            });
            ui.horizontal(|ui| {
                // step grid
                let cw = 19.0;
                let chh = 15.0;
                let lw = 56.0;
                let size = Vec2::new(lw + cw * 16.0, chh * DRUM_ROWS as f32);
                let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
                let painter = ui.painter_at(rect);
                let step = p.drum_step.load(Relaxed);
                let len = p.drum_len.load(Relaxed) as usize;
                for r in 0..DRUM_ROWS {
                    let y = rect.top() + r as f32 * chh;
                    painter.text(Pos2::new(rect.left(), y + chh / 2.0), Align2::LEFT_CENTER, DRUM_NAMES[r], FontId::proportional(11.0), text_dim());
                    let bits = p.drum_rows[r].load(Relaxed);
                    for s in 0..16 {
                        let x = rect.left() + lw + s as f32 * cw;
                        let cell = Rect::from_min_size(Pos2::new(x + 1.0, y + 1.0), Vec2::new(cw - 2.0, chh - 2.0));
                        let on = bits & (1 << s) != 0;
                        let beat = (s / 4) % 2 == 0 && len == 16 || (s / 3) % 2 == 0 && len == 12;
                        let mut col = if s >= len {
                            gray(26)
                        } else if on {
                            GOOD
                        } else if beat {
                            gray(52)
                        } else {
                            gray(40)
                        };
                        if s as u8 == step && on {
                            col = fg();
                        }
                        painter.rect_filled(cell, 3.0, col);
                        if s as u8 == step {
                            painter.rect_stroke(cell, 3.0, Stroke::new(1.5, fg()), StrokeKind::Inside);
                        }
                    }
                }
                if resp.clicked() {
                    if let Some(pos) = resp.interact_pointer_pos() {
                        let s = ((pos.x - rect.left() - lw) / cw).floor();
                        let r = ((pos.y - rect.top()) / chh).floor();
                        if s >= 0.0 && s < 16.0 && r >= 0.0 && (r as usize) < DRUM_ROWS {
                            let r = r as usize;
                            let v = p.drum_rows[r].load(Relaxed) ^ (1 << s as u16);
                            p.drum_rows[r].store(v, Relaxed);
                            self.s.drum_pattern = usize::MAX;
                        }
                    }
                }
                ui.vertical(|ui| {
                    let mut sw = p.swing.get();
                    if knob(ui, "Swing", &mut sw, 0.0, GOOD, pct) {
                        p.swing.set(sw);
                    }
                    let mut dv = p.drum_vol.get();
                    if knob(ui, "Level", &mut dv, 0.8, GOOD, pct) {
                        p.drum_vol.set(dv);
                    }
                });
            });
        });
    }

    fn loop_card(&mut self, ui: &mut egui::Ui) {
        let p = self.p.clone();
        let tx = self.tx.clone();
        let t = now_s(&self.t0);
        card(ui, "LOOPER", 200.0, |ui| {
            let st = p.loop_state.load(Relaxed);
            let pulse = ((t * 4.0).sin() * 0.5 + 0.5) as f32;
            let (label, col) = match st {
                lstate::EMPTY => ("● RECORD", BAD),
                lstate::REC => ("■ CLOSE LOOP", BAD.gamma_multiply(0.6 + 0.4 * pulse)),
                lstate::PLAY => ("+ OVERDUB", GOOD),
                lstate::DUB => ("✓ DONE", LIVE.gamma_multiply(0.6 + 0.4 * pulse)),
                _ => ("▶ PLAY LOOP", GOOD),
            };
            if ui
                .add_sized(Vec2::new(180.0, 46.0), egui::Button::new(RichText::new(label).size(17.0).strong().color(Color32::BLACK)).fill(col).corner_radius(10.0))
                .on_hover_text("Enter")
                .clicked()
            {
                let _ = tx.send(Cmd::LoopButton);
            }
            // progress
            let (r, _) = ui.allocate_exact_size(Vec2::new(180.0, 10.0), Sense::hover());
            ui.painter().rect_filled(r, 4.0, gray(40));
            if st != lstate::EMPTY {
                let mut f = r;
                f.set_width(r.width() * p.loop_pos.get().clamp(0.0, 1.0));
                ui.painter().rect_filled(f, 4.0, if st == lstate::REC { BAD } else { LOOPC });
            }
            ui.horizontal(|ui| {
                if ui.button("■ Stop").clicked() {
                    let _ = tx.send(Cmd::LoopStop);
                }
                if ui.button("↶ Undo").on_hover_text("Remove the last layer (Backspace)").clicked() {
                    let _ = tx.send(Cmd::LoopUndo);
                }
                if ui.button("✕").on_hover_text("Clear loop").clicked() {
                    let _ = tx.send(Cmd::LoopClear);
                }
            });
            let passes = p.loop_passes.load(Relaxed);
            let beats = p.loop_beats.get();
            let info = match st {
                lstate::EMPTY => "Press, play, press again.\nLength snaps to whole bars.".to_string(),
                lstate::REC => "Recording… press to close".to_string(),
                _ => format!("{} layer{} / {}  ·  {} bars", passes, if passes == 1 { "" } else { "s" }, MAX_PASSES, (beats / 4.0).max(0.25)),
            };
            ui.label(RichText::new(info).size(11.0).color(text_dim()));
            ui.label(RichText::new("Each layer keeps its own sound — switch sounds between overdubs.").size(10.5).color(text_dim()));
        });
    }

    // ---------- learn bar: view, scales, songs ----------
    fn learn_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("View").color(text_dim()));
            for (i, n) in ["Falling notes", "Score", "Both"].iter().enumerate() {
                if ui.selectable_label(self.s.view == i as u8, *n).clicked() {
                    self.s.view = i as u8;
                }
            }
            let lbl = if self.s.score_dark { "🌙 Dark staff" } else { "☀ Paper staff" };
            ui.toggle_value(&mut self.s.score_dark, lbl).on_hover_text("Staff colours");
            let w = (self.s.written as usize).min(WRITTEN.len() - 1);
            egui::ComboBox::from_id_salt("written").selected_text(WRITTEN[w].0.split(" (").next().unwrap_or("")).width(118.0).show_ui(ui, |ui| {
                for (i, (n, _)) in WRITTEN.iter().enumerate() {
                    ui.selectable_value(&mut self.s.written, i as u8, *n);
                }
            }).response.on_hover_text("Show the staff in written pitch for your instrument (sax, clarinet, whistle…)");
            ui.separator();
            if ui.toggle_value(&mut self.game_mode, RichText::new("🎮 Game").strong()).on_hover_text("Rhythm-game training: notes fall, you hit them on time").changed() {
                if self.game_mode {
                    self.scale_on = true;
                    self.rebuild_walk();
                    if self.s.view == 1 {
                        self.s.view = 2;
                    }
                } else {
                    let _ = self.tx.send(Cmd::SongPlay(false));
                    self.game = None;
                }
            }
            let was = self.scale_on;
            ui.toggle_value(&mut self.scale_on, RichText::new("🎼 Scale").strong());
            if was != self.scale_on {
                self.rebuild_walk();
                if !self.scale_on {
                    self.walk = false;
                    self.p.scale_snap.store(false, Relaxed);
                }
            }
            let mut changed = false;
            egui::ComboBox::from_id_salt("root").selected_text(ROOTS[self.s.scale_root].0).width(46.0).height(400.0).show_ui(ui, |ui| {
                for (i, r) in ROOTS.iter().enumerate() {
                    changed |= ui.selectable_value(&mut self.s.scale_root, i, r.0).changed();
                }
            });
            let sidx = self.s.scale_idx.min(SCALES.len() - 1);
            egui::ComboBox::from_id_salt("scale").selected_text(SCALES[sidx].name).width(240.0).height(600.0).show_ui(ui, |ui| {
                let te = ui.add(egui::TextEdit::singleline(&mut self.scale_filter).hint_text(format!("🔍 search {} scales & other names…", SCALES.len())).desired_width(200.0));
                if !te.has_focus() && self.scale_filter.is_empty() {
                    te.request_focus(); // type straight away; keys don't play notes while searching
                }
                let f = self.scale_filter.to_lowercase();
                let mut group = "";
                for (i, sc) in SCALES.iter().enumerate() {
                    if !f.is_empty()
                        && !sc.name.to_lowercase().contains(&f)
                        && !sc.aka.to_lowercase().contains(&f)
                        && !sc.group.to_lowercase().contains(&f)
                        && !sc.hint.to_lowercase().contains(&f)
                    {
                        continue;
                    }
                    if sc.group != group {
                        group = sc.group;
                        ui.label(RichText::new(group).size(11.0).color(YOURS));
                    }
                    let label = if sc.aka.is_empty() {
                        egui::text::LayoutJob::simple_singleline(sc.name.into(), FontId::proportional(14.0), fg())
                    } else {
                        let mut j = egui::text::LayoutJob::default();
                        j.append(sc.name, 0.0, egui::TextFormat::simple(FontId::proportional(14.0), fg()));
                        j.append(&format!("   {}", sc.aka), 0.0, egui::TextFormat::simple(FontId::proportional(11.5), text_dim()));
                        j
                    };
                    changed |= ui.selectable_value(&mut self.s.scale_idx, i, label).changed();
                }
            });
            if changed {
                self.scale_on = true;
                self.scale_filter.clear();
                self.rebuild_walk();
            }
            if self.scale_on {
                let mut snap = self.p.scale_snap.load(Relaxed);
                if ui.toggle_value(&mut snap, "Snap").on_hover_text("Wrong notes get bent to the nearest scale note — can't play a bum note").changed() {
                    self.p.scale_snap.store(snap, Relaxed);
                }
                if ui.toggle_value(&mut self.walk, "Walk it").on_hover_text("Play the scale up and down one note at a time; the next note is outlined").changed() {
                    self.walk_pos = 0;
                    self.walk_laps = 0;
                }
                if ui.button("▶ Hear it").clicked() {
                    let sc = &SCALES[sidx];
                    let root = 60 + ROOTS[self.s.scale_root].1;
                    let mut up: Vec<u8> = sc.steps.iter().map(|s| root + s).collect();
                    up.push(root + 12);
                    let s = song::scale_song(&up, self.p.bpm.get(), self.s.main.0, self.s.main.1);
                    self.set_song(Some(Arc::new(s)), 0);
                    let _ = self.tx.send(Cmd::SongPlay(true));
                }
                let sp = self.speller();
                let names: Vec<String> = SCALES[sidx].steps.iter().map(|s| sp.name((ROOTS[self.s.scale_root].1 + s) % 12)).collect();
                ui.label(RichText::new(names.join(" ")).strong().color(pick(GOOD, Color32::from_rgb(0x1a, 0x8a, 0x3e))));
                if self.walk {
                    ui.label(RichText::new(format!("step {}/{} · laps {}", self.walk_pos + 1, self.walk_seq.len(), self.walk_laps)).color(YOURS));
                }
            }
        });
        if self.scale_on {
            let sc = &SCALES[self.s.scale_idx.min(SCALES.len() - 1)];
            let mut line = String::new();
            if !sc.aka.is_empty() {
                line.push_str(&format!("Also called: {}.  ", sc.aka));
            }
            line.push_str(sc.hint);
            if !line.is_empty() {
                ui.label(RichText::new(line).size(11.5).color(text_dim()));
            }
        }
        if self.game_mode {
            self.game_bar(ui);
        }
        // song bar
        ui.horizontal(|ui| {
            ui.label(RichText::new("Song").color(text_dim()));
            let name = self.song.as_ref().map(|s| s.name.clone()).unwrap_or_else(|| "Open a MIDI file…".into());
            let mut pick = None;
            egui::ComboBox::from_id_salt("songs").selected_text(name).width(220.0).height(400.0).show_ui(ui, |ui| {
                if self.song_files.is_empty() {
                    ui.label(RichText::new("No .mid files found yet").color(text_dim()));
                }
                for f in &self.song_files {
                    let n = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
                    if ui.selectable_label(false, n).on_hover_text(f.display().to_string()).clicked() {
                        pick = Some(f.clone());
                    }
                }
                ui.separator();
                ui.label(RichText::new(format!(
                    "Drag a .mid onto the window, or drop files in\n{}\n(MuseScore: File › Export › MIDI)",
                    sounds::user_dir().join("songs").display()
                )).size(11.0).color(text_dim()));
                if ui.small_button("↻ Rescan").clicked() {
                    self.song_files = sounds::find_songs();
                }
            });
            if let Some(f) = pick {
                self.load_song_path(&f);
            }
            if let Some(e) = &self.song_err {
                ui.label(RichText::new(e).color(BAD));
            }
            let Some(song) = self.song.clone() else { return };
            let playing = self.p.song_playing.load(Relaxed);
            if ui.button("⏮").clicked() {
                let _ = self.tx.send(Cmd::SongSeek(0.0));
            }
            if big_toggle(ui, playing, if playing { "⏸" } else { "▶" }, GOOD, Vec2::new(40.0, 22.0)).clicked() {
                let _ = self.tx.send(Cmd::SongPlay(!playing));
            }
            let mut pos = self.p.song_pos.get() as f32;
            let fmt = |t: f32| format!("{}:{:02}", (t / 60.0) as u32, (t % 60.0) as u32);
            ui.label(RichText::new(format!("{} / {}", fmt(pos), fmt(song.dur as f32))).monospace().size(12.0));
            if ui.add(egui::Slider::new(&mut pos, 0.0..=song.dur as f32).show_value(false)).changed() {
                let _ = self.tx.send(Cmd::SongSeek(pos as f64));
            }
            let mut spd = self.p.song_speed.get() * 100.0;
            if ui.add(egui::Slider::new(&mut spd, 25.0..=150.0).suffix("%").integer().text("speed")).changed() {
                self.p.song_speed.set(spd / 100.0);
            }
            let waiting = self.p.song_waiting.load(Relaxed);
            let wlabel = if waiting { "⏳ Waiting for you" } else { "Wait for me" };
            if ui
                .toggle_value(&mut self.song_wait, RichText::new(wlabel).color(if waiting { YOURS } else { fg() }))
                .on_hover_text("Song pauses at each of YOUR notes until you play them")
                .changed()
            {
                self.p.song_wait.store(self.song_wait, Relaxed);
            }
            let mut sv = self.p.song_vol.get();
            if ui.add(egui::Slider::new(&mut sv, 0.0..=1.0).show_value(false)).on_hover_text("Song volume").changed() {
                self.p.song_vol.set(sv);
            }
            if ui.button("✕").on_hover_text("Close song").clicked() {
                self.set_song(None, 0);
            }
        });
        if let Some(song) = self.song.clone() {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("You play:").color(YOURS)).on_hover_text("Pick the part(s) you want to play. They go silent and light up yellow.");
                for ch in 0..16u16 {
                    if song.channels & (1 << ch) == 0 {
                        continue;
                    }
                    let prog = song.programs[ch as usize];
                    let inst = if ch == 9 {
                        "Drums".to_string()
                    } else if prog >= 0 {
                        self.presets.iter().find(|p| p.bank == 0 && p.prog == prog as i32).map(|p| p.name.clone()).unwrap_or(format!("Prog {}", prog + 1))
                    } else {
                        "Piano".into()
                    };
                    let on = self.song_user & (1 << ch) != 0;
                    let txt = RichText::new(format!("{} · {} ({})", ch + 1, inst, song.notes_per_ch[ch as usize])).size(11.5).color(if on { Color32::BLACK } else { fg() });
                    if ui.add(egui::Button::new(txt).fill(if on { YOURS } else { card_hi() }).corner_radius(10.0)).clicked() {
                        self.song_user ^= 1 << ch;
                        self.p.song_user.store(self.song_user, Relaxed);
                    }
                }
            });
        }
    }

    // ---------- visualisations ----------
    fn chord_label(&self, painter: &egui::Painter, rect: Rect) {
        let mut keys: Vec<u8> = (0..128u8).filter(|k| self.held[0][*k as usize] > 0).collect();
        if keys.is_empty() {
            keys = (0..128u8).filter(|k| self.held[2][*k as usize] > 0).collect();
        }
        if let Some((big, small)) = theory::name_chord(&keys, &self.speller()) {
            painter.text(rect.left_top() + Vec2::new(14.0, 8.0), Align2::LEFT_TOP, big, FontId::proportional(34.0), fg());
            painter.text(rect.left_top() + Vec2::new(16.0, 50.0), Align2::LEFT_TOP, small, FontId::proportional(14.0), text_dim());
        }
    }

    fn waterfall(&self, ui: &mut egui::Ui, rect: Rect) {
        let (lo, hi) = self.range();
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 0.0, deep());
        let now = now_s(&self.t0);
        let mask = self.scale_mask();
        // lanes
        for k in lo..=hi {
            let (x, w, black) = key_x(rect, k, lo, hi);
            if k % 12 == 0 {
                painter.line_segment([Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())], Stroke::new(1.0, gray(34)));
            }
            if mask != 0 && mask & (1 << (k % 12)) != 0 {
                let lane = Rect::from_min_max(Pos2::new(x, rect.top()), Pos2::new(x + w, rect.bottom()));
                let c = if k % 12 == ROOTS[self.s.scale_root].1 { GOOD.gamma_multiply(0.13) } else { GOOD.gamma_multiply(if black { 0.06 } else { 0.05 }) };
                painter.rect_filled(lane, 0.0, c);
            }
        }
        if let Some(song) = &self.song {
            // Synthesia style: song notes fall onto the keys
            let pos = self.p.song_pos.get();
            let speed = 160.0f64;
            let horizon = rect.height() as f64 / speed;
            let gstate = self.game.as_ref().filter(|g| Arc::ptr_eq(&g.chart, song));
            let now = now_s(&self.t0);
            for (i, n) in song.notes.iter().enumerate() {
                if n.start > pos + horizon {
                    break;
                }
                if n.end < pos - 0.6 || n.key < lo || n.key > hi || n.ch == 9 {
                    continue;
                }
                let (x, w, black) = key_x(rect, n.key, lo, hi);
                let yours = self.song_user & (1 << n.ch) != 0 || gstate.map(|g| g.mask & (1 << n.ch) != 0).unwrap_or(false);
                let judged = gstate.map(|g| g.state[i]).unwrap_or(game::J_PENDING);
                if judged != game::J_PENDING && judged != game::J_MISS {
                    // hit: burst at the hit line, then gone
                    let age = (now - gstate.unwrap().hit_at[i]) as f32;
                    if age < 0.35 {
                        let c = match judged {
                            game::J_PERFECT => YOURS,
                            game::J_GREAT => GOOD,
                            _ => ARP,
                        };
                        let grow = 1.0 + age * 3.0;
                        let r = Rect::from_center_size(Pos2::new(x + w / 2.0, rect.bottom() - 6.0), Vec2::new(w * grow, 18.0 * grow));
                        painter.rect_filled(r, 6.0, c.gamma_multiply(1.0 - age / 0.35));
                    }
                    continue;
                }
                if n.end < pos {
                    continue;
                }
                let yb = rect.bottom() - ((n.start - pos) * speed) as f32;
                let yt = rect.bottom() - ((n.end - pos) * speed) as f32;
                let base = if judged == game::J_MISS { BAD } else if yours { YOURS } else { SONG };
                let col = if black { base.gamma_multiply(0.75) } else { base };
                let r = Rect::from_min_max(Pos2::new(x + 1.0, yt.max(rect.top())), Pos2::new(x + w - 1.0, yb.min(rect.bottom() + 40.0)));
                painter.rect_filled(r, 3.0, if yours { col } else { col.gamma_multiply(0.6) });
                if yours && judged == game::J_PENDING {
                    painter.rect_stroke(r, 3.0, Stroke::new(1.0, fg().gamma_multiply(0.5)), StrokeKind::Inside);
                }
            }
            // hit line
            painter.line_segment([Pos2::new(rect.left(), rect.bottom() - 1.0), Pos2::new(rect.right(), rect.bottom() - 1.0)], Stroke::new(2.0, YOURS.gamma_multiply(0.5)));
            if let Some(g) = self.game.as_ref() {
                let now = now_s(&self.t0);
                // judgement pop-ups float up from the hit line
                for pp in &g.popups {
                    if pp.key < lo || pp.key > hi {
                        continue;
                    }
                    let (x, w, _) = key_x(rect, pp.key, lo, hi);
                    let age = (now - pp.t) as f32;
                    let c = match pp.kind {
                        game::J_PERFECT => YOURS,
                        game::J_GREAT => GOOD,
                        game::J_GOOD => ARP,
                        _ => BAD,
                    };
                    painter.text(Pos2::new(x + w / 2.0, rect.bottom() - 34.0 - age * 70.0), Align2::CENTER_CENTER, pp.text, FontId::proportional(17.0), c.gamma_multiply((1.0 - age / 0.8).max(0.0)));
                }
                // combo + score
                if g.combo >= 3 {
                    let pulse = if g.combo % 25 == 0 { 1.25 } else { 1.0 };
                    painter.text(rect.center_top() + Vec2::new(0.0, 46.0), Align2::CENTER_CENTER, format!("{}", g.combo), FontId::proportional(54.0 * pulse), YOURS.gamma_multiply(0.9));
                    painter.text(rect.center_top() + Vec2::new(0.0, 84.0), Align2::CENTER_CENTER, "COMBO", FontId::proportional(14.0), text_dim());
                }
                let mult = 1 + (g.combo / 25).min(3);
                painter.text(rect.right_top() + Vec2::new(-14.0, 10.0), Align2::RIGHT_TOP, format!("{:07}", g.score), FontId::monospace(26.0), fg());
                painter.text(rect.right_top() + Vec2::new(-14.0, 42.0), Align2::RIGHT_TOP, format!("{:.1}%  ·  ×{}", g.accuracy() * 100.0, mult), FontId::proportional(14.0), text_dim());
                // progress
                let prog = (self.p.song_pos.get() / g.chart.dur.max(0.1)).clamp(0.0, 1.0) as f32;
                let bar = Rect::from_min_size(rect.left_top(), Vec2::new(rect.width() * prog, 4.0));
                painter.rect_filled(bar, 0.0, YOURS.gamma_multiply(0.8));
                // streak glow
                if g.combo >= 25 {
                    painter.rect_stroke(rect.shrink(1.0), 0.0, Stroke::new(3.0, YOURS.gamma_multiply(0.35)), StrokeKind::Inside);
                }
            }
            // your notes still glow at the bottom
            for b in self.bars.iter().filter(|b| b.end.is_none() && b.key >= lo && b.key <= hi) {
                let (x, w, _) = key_x(rect, b.key, lo, hi);
                let r = Rect::from_min_max(Pos2::new(x, rect.bottom() - 16.0), Pos2::new(x + w, rect.bottom()));
                painter.rect_filled(r, 2.0, SRC_COL[(b.src as usize).min(2)]);
            }
            if self.p.song_waiting.load(Relaxed) {
                painter.text(rect.right_top() + Vec2::new(-14.0, 10.0), Align2::RIGHT_TOP, "⏳ your turn — play the yellow notes", FontId::proportional(18.0), YOURS);
            }
        } else {
            // free play: what you play rises from the keys
            let speed = 110.0f64;
            for b in &self.bars {
                if b.key < lo || b.key > hi {
                    continue;
                }
                let end = b.end.unwrap_or(now);
                let yb = rect.bottom() - ((now - end) * speed) as f32;
                let yt = rect.bottom() - ((now - b.start) * speed) as f32;
                if yb < rect.top() {
                    continue;
                }
                let (x, w, black) = key_x(rect, b.key, lo, hi);
                let mut col = SRC_COL[(b.src as usize).min(2)];
                if mask != 0 && b.src == src::LIVE && mask & (1 << (b.key % 12)) == 0 {
                    col = BAD;
                }
                let a = 0.45 + 0.55 * (b.vel as f32 / 127.0);
                let col = if black { col.gamma_multiply(0.8 * a) } else { col.gamma_multiply(a) };
                let r = Rect::from_min_max(Pos2::new(x + 1.5, yt.max(rect.top())), Pos2::new(x + w - 1.5, yb));
                painter.rect_filled(r, 3.0, col);
            }
        }
        self.chord_label(&painter, rect);
        if self.bars.is_empty() && self.song.is_none() {
            painter.text(rect.center(), Align2::CENTER_CENTER, "Just play.", FontId::proportional(40.0), gray(60));
            painter.text(rect.center() + Vec2::new(0.0, 36.0), Align2::CENTER_CENTER, "No keyboard? Your computer keys A W S E D F T G Y H U J K play notes · Z / X shift octave", FontId::proportional(14.0), gray(80));
        }
    }

    /// Draw a SMuFL glyph (Bravura) with its baseline at `y`. `space` = staff space in px.
    fn glyph(&self, painter: &egui::Painter, ch: char, x: f32, y: f32, space: f32, col: Color32) -> f32 {
        let font = FontId::new(space * 4.0, FontFamily::Name("music".into()));
        let galley = painter.layout_no_wrap(ch.to_string(), font, col);
        let base = galley.rows.first().map(|r| r.pos.y + r.row.glyphs.first().map(|g| g.pos.y).unwrap_or(0.0)).unwrap_or(0.0);
        let w = galley.size().x;
        painter.galley(Pos2::new(x, y - base), galley, col);
        w
    }

    fn score(&self, ui: &mut egui::Ui, rect: Rect) {
        let painter = ui.painter_at(rect);
        let dark = self.s.score_dark;
        painter.rect_filled(rect, 0.0, if dark { Color32::from_rgb(0x14, 0x17, 0x1c) } else { Color32::from_rgb(0xf3, 0xef, 0xe6) });
        let ink = if dark { Color32::from_rgb(0xd6, 0xdb, 0xe3) } else { Color32::from_rgb(0x1c, 0x1c, 0x22) };
        let sp = (rect.height() / 19.0).clamp(6.0, 16.0); // staff space
        let half = sp / 2.0;
        let cy = rect.center().y + sp * 0.5;
        let y_of = |step: i32| cy - step as f32 * half;
        let left = rect.left() + 14.0;
        let right = rect.right() - 10.0;
        for s in [2, 4, 6, 8, 10, -2, -4, -6, -8, -10] {
            painter.line_segment([Pos2::new(left, y_of(s)), Pos2::new(right, y_of(s))], Stroke::new(1.0, ink.gamma_multiply(0.7)));
        }
        // brace-ish system line
        painter.line_segment([Pos2::new(left, y_of(10)), Pos2::new(left, y_of(-10))], Stroke::new(2.0, ink));
        let clef_x = left + 6.0;
        if self.music_font {
            self.glyph(&painter, '\u{E050}', clef_x, y_of(4), sp, ink); // G clef on G4 line
            self.glyph(&painter, '\u{E062}', clef_x, y_of(-4), sp, ink); // F clef on F3 line
        } else {
            painter.text(Pos2::new(clef_x, y_of(6)), Align2::LEFT_CENTER, "G", FontId::proportional(sp * 3.0), ink);
            painter.text(Pos2::new(clef_x, y_of(-6)), Align2::LEFT_CENTER, "F", FontId::proportional(sp * 3.0), ink);
        }
        let woff = WRITTEN[(self.s.written as usize).min(WRITTEN.len() - 1)].1;
        let speller = if self.scale_on {
            let pc = ((ROOTS[self.s.scale_root].1 as i32 + woff).rem_euclid(12)) as u8;
            let ri = ROOTS.iter().position(|r| r.1 == pc).unwrap_or(0);
            Speller::for_scale(ri, &SCALES[self.s.scale_idx.min(SCALES.len() - 1)])
        } else {
            Speller::default_sharps()
        };
        let wk = |k: u8| (k as i32 + woff).clamp(0, 127) as u8;
        let has_song = self.song.is_some();
        let now_x = if has_song { left + (right - left) * 0.3 } else { right - 60.0 };
        let px_per_s = 110.0f64;
        let clip_left = left + sp * 4.0;
        painter.line_segment([Pos2::new(now_x, y_of(14)), Pos2::new(now_x, y_of(-14))], Stroke::new(1.5, LIVE.gamma_multiply(0.5)));

        let draw_note = |key: u8, x: f32, x_end: f32, col: Color32, tail: bool| {
            if x < clip_left - 4.0 || x > right {
                return;
            }
            let key = wk(key);
            let (step, acc) = theory::staff_step(&speller, key);
            let treble = key >= 60;
            let y = y_of(step);
            // ledger lines
            let mut ledgers: Vec<i32> = vec![];
            if treble {
                if step <= 0 {
                    let mut s = 0;
                    while s >= step {
                        ledgers.push(s);
                        s -= 2;
                    }
                }
                let mut s = 12;
                while s <= step {
                    ledgers.push(s);
                    s += 2;
                }
            } else {
                if step >= 0 {
                    ledgers.push(0);
                }
                let mut s = -12;
                while s >= step {
                    ledgers.push(s);
                    s -= 2;
                }
            }
            for l in ledgers {
                painter.line_segment([Pos2::new(x - sp * 0.95, y_of(l)), Pos2::new(x + sp * 0.95, y_of(l))], Stroke::new(1.2, ink));
            }
            if tail && x_end > x + sp {
                painter.line_segment([Pos2::new(x, y), Pos2::new(x_end.min(right), y)], Stroke::new(sp * 0.45, col.gamma_multiply(0.35)));
            }
            if self.music_font {
                self.glyph(&painter, '\u{E0A4}', x - sp * 0.59, y, sp, col);
                if acc != 0 {
                    let g = match acc {
                        -2 => '\u{E264}',
                        -1 => '\u{E260}',
                        1 => '\u{E262}',
                        _ => '\u{E263}',
                    };
                    self.glyph(&painter, g, x - sp * 1.9, y, sp, col);
                }
            } else {
                painter.circle_filled(Pos2::new(x, y), half * 1.1, col);
                if acc != 0 {
                    painter.text(Pos2::new(x - sp * 1.4, y), Align2::CENTER_CENTER, theory::acc_str(acc), FontId::proportional(sp * 1.6), col);
                }
            }
        };

        let now = now_s(&self.t0);
        if let Some(song) = &self.song {
            let pos = self.p.song_pos.get();
            let back = ((now_x - clip_left) as f64) / px_per_s;
            let ahead = ((right - now_x) as f64) / px_per_s;
            for n in song.notes.iter() {
                if n.start > pos + ahead {
                    break;
                }
                if n.end < pos - back || n.ch == 9 {
                    continue;
                }
                let yours = self.song_user & (1 << n.ch) != 0;
                if self.song_user != 0 && !yours {
                    continue; // only show your part when you've picked one
                }
                let x = now_x + ((n.start - pos) * px_per_s) as f32;
                let xe = now_x + ((n.end - pos) * px_per_s) as f32;
                let col = if yours { if dark { YOURS } else { Color32::from_rgb(0xb0, 0x7a, 0x00) } } else { ink };
                draw_note(n.key, x, xe, col, true);
            }
        }
        for b in &self.bars {
            let end = b.end.unwrap_or(now);
            let x = now_x - ((now - b.start) * px_per_s) as f32;
            let xe = now_x - ((now - end) * px_per_s) as f32;
            if xe < clip_left {
                continue;
            }
            let mut col = match (b.src, dark) {
                (src::LIVE, false) => Color32::from_rgb(0xd9, 0x5f, 0x0e),
                (src::ARP, false) => Color32::from_rgb(0x0a, 0x8c, 0xb0),
                (_, false) => Color32::from_rgb(0x6c, 0x45, 0xc8),
                (src::LIVE, true) => LIVE,
                (src::ARP, true) => ARP,
                (_, true) => LOOPC,
            };
            if self.scale_on && b.src == src::LIVE && self.scale_mask() & (1 << (b.key % 12)) == 0 {
                col = if dark { BAD } else { Color32::from_rgb(0xd0, 0x20, 0x30) };
            }
            draw_note(b.key, x, xe, col, true);
        }
        if self.scale_on && self.walk && !self.walk_seq.is_empty() {
            let k = self.walk_seq[self.walk_pos];
            let pulse = ((now * 5.0).sin() * 0.5 + 0.5) as f32;
            draw_note(k, now_x + 36.0, now_x + 36.0, (if dark { GOOD } else { Color32::from_rgb(0x2e, 0x9e, 0x4f) }).gamma_multiply(0.55 + 0.45 * pulse), false);
            painter.text(Pos2::new(now_x + 36.0, y_of(15)), Align2::CENTER_CENTER, format!("next: {}", speller.name(wk(k) % 12)), FontId::proportional(13.0), Color32::from_rgb(0x2e, 0x9e, 0x4f));
        }
        // chord name in ink
        let mut keys: Vec<u8> = (0..128u8).filter(|k| self.held[0][*k as usize] > 0).collect();
        if keys.is_empty() {
            keys = (0..128u8).filter(|k| self.held[2][*k as usize] > 0).collect();
        }
        let keys: Vec<u8> = keys.into_iter().map(wk).collect();
        if woff != 0 {
            painter.text(Pos2::new(right - 8.0, rect.top() + 6.0), Align2::RIGHT_TOP, format!("written for {}", WRITTEN[(self.s.written as usize).min(WRITTEN.len() - 1)].0), FontId::proportional(11.0), ink.gamma_multiply(0.6));
        }
        if let Some((big, small)) = theory::name_chord(&keys, &speller) {
            painter.text(Pos2::new(left + sp * 5.0, rect.top() + 4.0), Align2::LEFT_TOP, big, FontId::proportional(26.0), ink);
            painter.text(Pos2::new(left + sp * 5.0, rect.top() + 34.0), Align2::LEFT_TOP, small, FontId::proportional(12.0), ink.gamma_multiply(0.6));
        }
    }

    fn piano(&mut self, ui: &mut egui::Ui) {
        let (lo, hi) = self.range();
        let mut rect = ui.available_rect_before_wrap();
        if self.s.kb_size < 88 {
            // mini 88-key overview: where the visible keys sit on a full piano
            let strip = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 12.0));
            rect.min.y += 16.0;
            let p = ui.painter();
            for k in 21..=108u8 {
                let (x, w, b) = key_x(strip, k, 21, 108);
                let r = Rect::from_min_size(Pos2::new(x, strip.top()), Vec2::new(w, if b { 7.0 } else { 12.0 }));
                let inside = k >= lo && k <= hi;
                let base = if b { gray(30) } else { Color32::from_gray(if inside { 200 } else { 95 }) };
                let col = if self.held.iter().any(|h| h[k as usize] > 0) { LIVE } else { base };
                p.rect_filled(if b { r } else { r.shrink2(Vec2::new(0.4, 0.0)) }, 1.0, col);
            }
            let (x0, _, _) = key_x(strip, lo, 21, 108);
            let (x1, w1, _) = key_x(strip, hi, 21, 108);
            p.rect_stroke(Rect::from_min_max(Pos2::new(x0 - 1.0, strip.top() - 1.0), Pos2::new(x1 + w1 + 1.0, strip.bottom() + 1.0)), 2.0, Stroke::new(1.5, LIVE), StrokeKind::Outside);
        }
        let resp = ui.allocate_rect(rect, Sense::click_and_drag());
        let painter = ui.painter_at(rect);
        let now = now_s(&self.t0);
        let mask = self.scale_mask();
        let big = self.s.kb_size <= 37;
        let root_pc = ROOTS[self.s.scale_root].1;
        let speller = self.speller();
        let walk_target = if self.scale_on && self.walk { self.walk_seq.get(self.walk_pos).copied() } else { None };
        let song_now: Vec<(u8, bool)> = match &self.song {
            Some(s) => {
                let pos = self.p.song_pos.get();
                s.notes.iter().take_while(|n| n.start <= pos + 0.02).filter(|n| n.end > pos && n.ch != 9).map(|n| (n.key, self.song_user & (1 << n.ch) != 0)).collect()
            }
            None => vec![],
        };
        let state_col = |k: u8| -> Option<Color32> {
            for s in 0..3 {
                if self.held[s][k as usize] > 0 {
                    return Some(SRC_COL[s]);
                }
            }
            if let Some((_, yours)) = song_now.iter().find(|(kk, _)| *kk == k) {
                return Some(if *yours { YOURS } else { SONG });
            }
            None
        };
        let bh = rect.height() * 0.62;
        for black in [false, true] {
            for k in lo..=hi {
                let (x, w, b) = key_x(rect, k, lo, hi);
                if b != black {
                    continue;
                }
                let r = Rect::from_min_size(Pos2::new(x, rect.top()), Vec2::new(w, if b { bh } else { rect.height() }));
                let base = if b { Color32::from_rgb(0x18, 0x19, 0x1c) } else { Color32::from_rgb(0xf4, 0xf1, 0xea) };
                let mut fill = base;
                if let Some(c) = state_col(k) {
                    fill = c;
                }
                let wr = (now - self.wrong[k as usize]) as f32;
                if wr < 0.45 {
                    fill = BAD.gamma_multiply(1.0 - wr / 0.45 * 0.5);
                }
                let shrink = if b { 1.0 } else { 0.7 };
                painter.rect_filled(r.shrink2(Vec2::new(shrink, 0.0)), if b { 3.0 } else { 4.0 }, fill);
                if !b {
                    painter.rect_stroke(r.shrink2(Vec2::new(shrink, 0.0)), 4.0, Stroke::new(1.0, gray(90)), StrokeKind::Inside);
                }
                // scale dots + names
                if mask != 0 && mask & (1 << (k % 12)) != 0 {
                    let is_root = k % 12 == root_pc;
                    let cy = if b { r.bottom() - 10.0 } else { r.bottom() - 26.0 };
                    let c = Pos2::new(r.center().x, cy);
                    painter.circle_filled(c, if is_root { 6.0 } else { 4.5 }, if is_root { Color32::from_rgb(0x1f, 0x9d, 0x4c) } else { GOOD });
                    if !b {
                        painter.text(Pos2::new(r.center().x, r.bottom() - 12.0), Align2::CENTER_CENTER, speller.name(k % 12), FontId::proportional(10.5), Color32::from_rgb(0x1f, 0x7d, 0x3c));
                    }
                } else if !b && k % 12 == 0 {
                    painter.text(Pos2::new(r.center().x, r.bottom() - 10.0), Align2::CENTER_CENTER, format!("C{}", k / 12 - 1), FontId::proportional(if big { 13.0 } else { 10.0 }), gray(110));
                } else if !b && big {
                    painter.text(Pos2::new(r.center().x, r.bottom() - 10.0), Align2::CENTER_CENTER, theory::NOTE_SHARP[(k % 12) as usize], FontId::proportional(12.0), gray(150));
                }
                if Some(k) == walk_target {
                    let pulse = ((now * 5.0).sin() * 0.5 + 0.5) as f32;
                    painter.rect_stroke(r.shrink(1.5), 4.0, Stroke::new(3.0, YOURS.gamma_multiply(0.5 + 0.5 * pulse)), StrokeKind::Inside);
                }
                let rt = (now - self.right[k as usize]) as f32;
                if rt < 0.35 && self.walk {
                    painter.rect_stroke(r.shrink(1.0), 4.0, Stroke::new(3.0, GOOD.gamma_multiply(1.0 - rt / 0.35)), StrokeKind::Inside);
                }
            }
        }
        // mouse playing (with glissando)
        let pointer_key = |pos: Pos2| -> Option<u8> {
            if !rect.contains(pos) {
                return None;
            }
            if pos.y < rect.top() + bh {
                for k in lo..=hi {
                    let (x, w, b) = key_x(rect, k, lo, hi);
                    if b && pos.x >= x && pos.x < x + w {
                        return Some(k);
                    }
                }
            }
            (lo..=hi).find(|&k| {
                let (x, w, b) = key_x(rect, k, lo, hi);
                !b && pos.x >= x && pos.x < x + w
            })
        };
        let down = resp.is_pointer_button_down_on() || resp.dragged();
        let target = if down { resp.interact_pointer_pos().and_then(pointer_key) } else { None };
        if target != self.mouse_note {
            if let Some(k) = self.mouse_note {
                self.send_note(k, false);
            }
            if let Some(k) = target {
                self.send_note(k, true);
            }
            self.mouse_note = target;
        }
    }

    fn pipe_picker(&mut self, ctx: &egui::Context) {
        if !self.pipe_picker {
            return;
        }
        let mut open = true;
        let mut pick = None;
        egui::Window::new("Choose your pipes").collapsible(false).resizable(false).open(&mut open).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            egui::Grid::new("pipes_grid").spacing([10.0, 10.0]).show(ui, |ui| {
                for (i, pp) in crate::pipes::PIPES.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(Vec2::new(150.0, 172.0), Sense::click());
                    let sel = i == self.s.pipe_idx;
                    let bgc = if resp.hovered() { card_hi() } else { card_bg() };
                    ui.painter().rect_filled(r, 10.0, bgc);
                    if sel {
                        ui.painter().rect_stroke(r, 10.0, Stroke::new(2.5, LIVE), StrokeKind::Inside);
                    }
                    let art = Rect::from_min_size(r.min + Vec2::new(15.0, 8.0), Vec2::new(120.0, 120.0));
                    ui.painter().rect_filled(art, 8.0, deep());
                    crate::pipe_art::draw(ui.painter(), art.shrink(6.0), i);
                    ui.painter().text(Pos2::new(r.center().x, r.bottom() - 34.0), Align2::CENTER_CENTER, pp.name, FontId::proportional(12.5), fg());
                    let drones = pp.drones.iter().filter(|d| d.2).count();
                    let key = if matches!(pp.tonic % 12, 1 | 3 | 8 | 10) { theory::NOTE_FLAT[(pp.tonic % 12) as usize] } else { theory::NOTE_SHARP[(pp.tonic % 12) as usize] };
                    ui.painter().text(Pos2::new(r.center().x, r.bottom() - 16.0), Align2::CENTER_CENTER, format!("{key} · {drones} drone{}", if drones == 1 { "" } else { "s" }), FontId::proportional(11.0), text_dim());
                    if resp.on_hover_text(pp.note).clicked() {
                        pick = Some(i);
                    }
                    if i % 4 == 3 {
                        ui.end_row();
                    }
                }
            });
        });
        if let Some(i) = pick {
            self.select_pipe(i);
            self.pipe_picker = false;
        } else if !open {
            self.pipe_picker = false;
        }
    }

    fn results(&mut self, ctx: &egui::Context) {
        if !self.show_results {
            return;
        }
        let Some(g) = self.game.as_ref() else { return };
        let (grade, acc, score, maxc, counts, wrong, total, name) = (g.grade(), g.accuracy(), g.score, g.max_combo, g.counts, g.wrong, g.total, g.name.clone());
        let best = self.s.g_best.get(&name).cloned();
        let mut again = false;
        let mut faster = false;
        let mut close = false;
        egui::Window::new("Results").collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).show(ctx, |ui| {
            ui.set_min_width(320.0);
            ui.vertical_centered(|ui| {
                let gc = match grade {
                    "S+" | "S" => YOURS,
                    "A" => GOOD,
                    "B" => ARP,
                    _ => BAD,
                };
                ui.label(RichText::new(grade).size(72.0).strong().color(gc));
                ui.label(RichText::new(format!("{:.1}%", acc * 100.0)).size(24.0));
                ui.label(RichText::new(&name).size(12.0).color(text_dim()));
            });
            ui.add_space(8.0);
            egui::Grid::new("res").num_columns(2).spacing([30.0, 4.0]).show(ui, |ui| {
                for (lbl, v, c) in [
                    ("Perfect", counts[1], YOURS),
                    ("Great", counts[2], GOOD),
                    ("Good", counts[3], ARP),
                    ("Miss", counts[4], BAD),
                    ("Wrong notes", wrong, BAD),
                ] {
                    ui.label(RichText::new(lbl).color(c));
                    ui.label(RichText::new(v.to_string()).monospace());
                    ui.end_row();
                }
                ui.label("Max combo");
                ui.label(RichText::new(format!("{maxc} / {total}")).monospace());
                ui.end_row();
                ui.label("Score");
                ui.label(RichText::new(score.to_string()).monospace());
                ui.end_row();
            });
            if let Some((b, bg)) = best {
                ui.label(RichText::new(format!("Personal best: {bg} · {:.1}%", b * 100.0)).color(YOURS));
            }
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                again = ui.button("↻ Again").clicked();
                faster = ui.button("⏩ Again, +5 bpm").clicked();
                close = ui.button("Done").clicked();
            });
        });
        let from_song = name.ends_with("· song");
        if again || faster {
            if faster {
                if from_song {
                    let sp = (self.p.song_speed.get() + 0.05).min(2.0);
                    self.p.song_speed.set(sp);
                } else {
                    self.s.g_bpm = (self.s.g_bpm + 5.0).min(240.0);
                }
            }
            self.start_game(from_song);
        } else if close {
            self.show_results = false;
        }
    }

    fn kb_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(RichText::new("Keys").color(text_dim()));
            for (size, _) in KB_SIZES {
                if ui.selectable_label(self.s.kb_size == size, RichText::new(size.to_string()).size(12.5)).clicked() {
                    self.set_kb_size(size);
                }
            }
            if self.s.kb_size < 88 {
                if ui.small_button("◀ oct").clicked() && self.s.kb_lo >= 12 {
                    self.s.kb_lo -= 12;
                }
                if ui.small_button("oct ▶").clicked() && self.range().1 + 12 <= 127 {
                    self.s.kb_lo += 12;
                }
                ui.toggle_value(&mut self.s.kb_follow, "Follow").on_hover_text("Slide along when you play outside the visible keys (e.g. after Oct +/−)");
                let (lo, hi) = self.range();
                ui.label(RichText::new(format!("{} – {}", theory::note_name_oct(lo), theory::note_name_oct(hi))).monospace().size(12.0).color(text_dim()));
            }
            ui.separator();
            ui.label(RichText::new("Layout").color(text_dim()));
            for (i, n) in ["Classic", "MiniLab 3"].iter().enumerate() {
                if ui.selectable_label(self.s.layout == i as u8, *n).clicked() {
                    self.s.layout = i as u8;
                    if i == 1 && self.s.kb_size == 88 {
                        self.set_kb_size(25);
                    }
                }
            }
            ui.separator();
            ui.label(RichText::new("Controller").color(text_dim()));
            let names = ["Auto-detect", "Off (raw MIDI)", "MiniLab 3"];
            egui::ComboBox::from_id_salt("ctrl").selected_text(names[self.s.controller.min(2) as usize]).width(120.0).show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    ui.selectable_value(&mut self.s.controller, i as u8, *n);
                }
            });
            if self.ml3_active {
                ui.label(RichText::new("● MiniLab 3 mapped").size(11.5).color(GOOD)).on_hover_text("Knobs, faders, pads and the main encoder are mapped 1:1");
            }
            ui.separator();
            ui.label(RichText::new("Wind").color(text_dim()));
            let wn = ["Auto-detect", "Off", "On"];
            egui::ComboBox::from_id_salt("wind").selected_text(wn[self.s.wind.min(2) as usize]).width(96.0).show_ui(ui, |ui| {
                for (i, n) in wn.iter().enumerate() {
                    ui.selectable_value(&mut self.s.wind, i as u8, *n);
                }
            }).response.on_hover_text("EWI / Aerophone / any breath controller: breath → expression");
            if self.wind_active {
                ui.label(RichText::new("● breath").size(11.5).color(ARP));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new("V view · F2 panels · F1 help").size(11.0).color(text_dim()));
            });
        });
    }

    fn help(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        egui::Window::new("Keys & shortcuts").collapsible(false).resizable(false).open(&mut self.show_help).show(ctx, |ui| {
            egui::Grid::new("help").num_columns(2).spacing([20.0, 6.0]).show(ui, |ui| {
                for (k, v) in [
                    ("A W S E D F T G Y H U J K O L P ;", "play notes (piano layout on your computer keys)"),
                    ("Z / X", "octave down / up for computer keys"),
                    ("← / →", "previous / next sound"),
                    ("Space", "drums play / stop"),
                    ("Enter", "looper: record → close → overdub → done"),
                    ("Backspace", "undo last loop layer"),
                    ("Esc", "panic: all notes off, stop loop"),
                    ("V", "cycle view: falling notes → score → both"),
                    ("F3 / ☀🌙 button", "dark / light / follow system theme"),
                    ("F2", "hide / show the sound, arp, drum and looper panels"),
                    ("Drag & drop", ".mid songs or .sf2 sound banks onto the window"),
                    ("Knobs", "drag up/down, scroll, Shift = fine, double-click = reset"),
                    ("MIDI", "any keyboard is picked up automatically, even if you plug it in later. Pads on channel 10 play drums. Program Change switches sounds."),
                ] {
                    ui.label(RichText::new(k).strong().monospace());
                    ui.label(v);
                    ui.end_row();
                }
            });
        });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let want_light = match self.s.theme {
            1 => true,
            2 => ctx.system_theme() == Some(egui::Theme::Light),
            _ => false,
        };
        if want_light != light() || !self.style_done {
            LIGHT.store(want_light, Relaxed);
            setup_style(ctx);
            self.style_done = true;
        }
        self.apply_controller();
        self.apply_drones();
        self.pump();
        self.keyboard_input(ctx);
        // drag & drop
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        for f in dropped {
            if let Some(p) = f.path {
                let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                match ext.as_str() {
                    "sf2" => self.load_sf(p),
                    "mid" | "midi" | "kar" => {
                        self.load_song_path(&p);
                        if !self.song_files.contains(&p) {
                            self.song_files.insert(0, p);
                        }
                    }
                    "mscz" | "musicxml" | "mxl" => self.song_err = Some("MuseScore files: use File › Export › MIDI for now (MusicXML import is planned)".into()),
                    _ => {}
                }
            }
        }

        egui::TopBottomPanel::top("header").frame(egui::Frame::new().fill(panel()).inner_margin(egui::Margin::symmetric(10, 8))).show(ctx, |ui| {
            self.header(ui);
            if let Some(e) = &self.load_err {
                ui.label(RichText::new(e).color(BAD));
            }
        });
        if !self.s.hide_modules {
            egui::TopBottomPanel::top("modules").frame(egui::Frame::new().fill(bg()).inner_margin(egui::Margin::symmetric(10, 8))).show(ctx, |ui| {
                self.modules(ui);
            });
        }
        egui::TopBottomPanel::bottom("kbbar").frame(egui::Frame::new().fill(panel()).inner_margin(egui::Margin { left: 10, right: 10, top: 5, bottom: 0 })).show(ctx, |ui| {
            self.kb_bar(ui);
        });
        egui::TopBottomPanel::bottom("piano")
            .exact_height(if self.s.kb_size <= 37 { 170.0 } else { 140.0 })
            .frame(egui::Frame::new().fill(panel()).inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 8 }))
            .show(ctx, |ui| self.piano(ui));
        egui::CentralPanel::default().frame(egui::Frame::new().fill(bg()).inner_margin(egui::Margin { left: 10, right: 10, top: 4, bottom: 0 })).show(ctx, |ui| {
            self.learn_bar(ui);
            ui.add_space(4.0);
            let rect = ui.available_rect_before_wrap();
            ui.allocate_rect(rect, Sense::hover());
            match self.s.view {
                0 => self.waterfall(ui, rect),
                1 => self.score(ui, rect),
                _ => {
                    let split = rect.top() + rect.height() * 0.55;
                    let top = Rect::from_min_max(rect.min, Pos2::new(rect.right(), split - 2.0));
                    let bot = Rect::from_min_max(Pos2::new(rect.left(), split + 2.0), rect.max);
                    self.score(ui, top);
                    self.waterfall(ui, bot);
                }
            }
        });
        self.help(ctx);
        self.game_tick();
        self.results(ctx);
        self.pipe_picker(ctx);
        let now = now_s(&self.t0);
        self.autosave(now);
        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.s.pull_from(&self.p);
        self.s.scale_on = self.scale_on;
        self.s.save();
    }
}

fn setup_style(ctx: &egui::Context) {
    let mut v = if light() { egui::Visuals::light() } else { egui::Visuals::dark() };
    v.panel_fill = bg();
    v.window_fill = panel();
    v.extreme_bg_color = deep();
    v.widgets.inactive.weak_bg_fill = card_hi();
    v.widgets.inactive.bg_fill = card_hi();
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0x38, 0x3e, 0x48);
    v.selection.bg_fill = LIVE.gamma_multiply(0.85);
    v.selection.stroke = Stroke::new(1.0, Color32::BLACK);
    if light() {
        v.widgets.hovered.weak_bg_fill = Color32::from_rgb(0xd4, 0xd8, 0xdf);
    }
    ctx.set_visuals(v);
    ctx.style_mut(|s| {
        s.spacing.item_spacing = Vec2::new(6.0, 5.0);
        s.spacing.button_padding = Vec2::new(7.0, 3.0);
    });
}

/// Bravura (SIL OFL) for clefs/noteheads, plus a system symbol font for ♯ ♭ in text.
fn setup_fonts(ctx: &egui::Context) -> bool {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert("bravura".into(), Arc::new(egui::FontData::from_static(include_bytes!("../assets/Bravura.otf"))));
    fonts.families.insert(FontFamily::Name("music".into()), vec!["bravura".into()]);
    let candidates = [
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/TTF/DejaVuSans.ttf",
        "/usr/share/fonts/dejavu/DejaVuSans.ttf",
        "/usr/share/fonts/noto/NotoSansSymbols-Regular.ttf",
        "C:\\Windows\\Fonts\\seguisym.ttf",
    ];
    let mut added = false;
    for c in candidates {
        if let Ok(bytes) = std::fs::read(c) {
            fonts.font_data.insert("symbols".into(), Arc::new(egui::FontData::from_owned(bytes)));
            fonts.families.entry(FontFamily::Proportional).or_default().push("symbols".into());
            added = true;
            break;
        }
    }
    let _ = added;
    fonts.families.entry(FontFamily::Proportional).or_default().push("bravura".into());
    ctx.set_fonts(fonts);
    true
}
