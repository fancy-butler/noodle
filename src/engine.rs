// Noodle — real-time audio engine.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Everything in `Engine::process` runs on the audio thread: no allocation,
// no locks. The UI talks to it through atomics (continuous params) and a
// bounded channel (discrete commands).

use crate::song::Song;
use crossbeam_channel::{Receiver, Sender};
use rustysynth::Synthesizer;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU16, AtomicU32, AtomicU64, AtomicU8, Ordering::Relaxed};
use std::sync::Arc;

pub struct AF32(AtomicU32);
impl AF32 {
    pub fn new(v: f32) -> Self {
        Self(AtomicU32::new(v.to_bits()))
    }
    pub fn get(&self) -> f32 {
        f32::from_bits(self.0.load(Relaxed))
    }
    pub fn set(&self, v: f32) {
        self.0.store(v.to_bits(), Relaxed)
    }
}
pub struct AF64(AtomicU64);
impl AF64 {
    pub fn new(v: f64) -> Self {
        Self(AtomicU64::new(v.to_bits()))
    }
    pub fn get(&self) -> f64 {
        f64::from_bits(self.0.load(Relaxed))
    }
    pub fn set(&self, v: f64) {
        self.0.store(v.to_bits(), Relaxed)
    }
}

pub const MAIN_CH: i32 = 0;
pub const LAYER_CH: i32 = 1;
pub const DRUM_CH: i32 = 9;
/// Each looper overdub pass gets its own channel pair, so every layer of the
/// loop keeps the instrument it was recorded with.
pub const PASS_CH: [(u8, u8); 6] = [(2, 3), (4, 5), (6, 7), (10, 11), (12, 13), (14, 15)];
pub const MAX_PASSES: usize = PASS_CH.len();

pub const DRUM_ROWS: usize = 8;
pub const DRUM_NOTES: [u8; DRUM_ROWS] = [36, 38, 39, 42, 46, 45, 50, 49];
pub const DRUM_NAMES: [&str; DRUM_ROWS] = ["Kick", "Snare", "Clap", "Hat", "Open hat", "Low tom", "Hi tom", "Crash"];
const DRUM_VEL: [i32; DRUM_ROWS] = [118, 108, 100, 82, 90, 100, 100, 96];

pub const ARP_RATES: [(&str, f64); 6] = [("1/4", 1.0), ("1/8", 0.5), ("1/8T", 1.0 / 3.0), ("1/16", 0.25), ("1/16T", 1.0 / 6.0), ("1/32", 0.125)];
pub const ARP_MODES: [&str; 5] = ["Up", "Down", "Up-Down", "Random", "Chord"];
pub const DELAY_TIMES: [(&str, f64); 5] = [("1/16", 0.25), ("1/8", 0.5), ("1/8T", 1.0 / 3.0), ("1/8.", 0.75), ("1/4", 1.0)];

pub mod src {
    pub const LIVE: u8 = 0;
    pub const ARP: u8 = 1;
    pub const LOOP: u8 = 2;
}

/// What a mapped MIDI CC controls (Params::cc_map).
pub mod target {
    pub const NONE: u8 = 0;
    pub const FILTER: u8 = 1;
    pub const RESO: u8 = 2;
    pub const DRIVE: u8 = 3;
    pub const DELAY: u8 = 4;
    pub const REVERB: u8 = 5;
    pub const REPEATS: u8 = 6;
    pub const CHORUS: u8 = 7;
    pub const ARP_GATE: u8 = 8;
    pub const MASTER: u8 = 9;
    pub const LAYER_VOL: u8 = 10;
    pub const DRUM_VOL: u8 = 11;
    pub const SONG_VOL: u8 = 12;
    pub const SWING: u8 = 13;
    pub const BROWSE: u8 = 20;
    pub const CATEGORY: u8 = 21;
    pub const PAD0: u8 = 30; // 30..37 → drum rows

    pub fn name(t: u8) -> &'static str {
        match t {
            FILTER => "Filter",
            RESO => "Reso",
            DRIVE => "Drive",
            DELAY => "Delay",
            REVERB => "Reverb",
            REPEATS => "Repeats",
            CHORUS => "Chorus",
            ARP_GATE => "Arp gate",
            MASTER => "Volume",
            LAYER_VOL => "Layer",
            DRUM_VOL => "Drums",
            SONG_VOL => "Song",
            SWING => "Swing",
            BROWSE => "Sounds",
            CATEGORY => "Category",
            _ => "—",
        }
    }
}

pub mod lstate {
    pub const EMPTY: u8 = 0;
    pub const REC: u8 = 1;
    pub const PLAY: u8 = 2;
    pub const DUB: u8 = 3;
    pub const STOPPED: u8 = 4;
}

pub struct Params {
    // sound
    pub master: AF32,
    pub reverb: AF32,
    pub chorus: AF32,
    pub delay_mix: AF32,
    pub delay_fb: AF32,
    pub delay_time: AtomicU8,
    pub cutoff: AF32,
    pub resonance: AF32,
    pub drive: AF32,
    pub bpm: AF32,
    // keyboard
    pub mode: AtomicU8, // 0 single, 1 layer, 2 split
    pub split: AtomicU8,
    pub layer_vol: AF32,
    pub transpose: AtomicI32, // semitones
    pub scale_mask: AtomicU16,
    pub scale_snap: AtomicBool,
    // arp
    pub arp_on: AtomicBool,
    pub arp_mode: AtomicU8,
    pub arp_rate: AtomicU8,
    pub arp_oct: AtomicU8,
    pub arp_latch: AtomicBool,
    pub arp_gate: AF32,
    // drums
    pub drums_on: AtomicBool,
    pub drum_rows: [AtomicU16; DRUM_ROWS],
    pub drum_len: AtomicU8,
    pub drum_kit: AtomicU8,
    pub drum_vol: AF32,
    pub swing: AF32,
    // song
    pub song_speed: AF32,
    pub song_user: AtomicU16, // channels the user plays (muted in playback)
    pub song_wait: AtomicBool,
    pub song_vol: AF32,
    // engine → UI
    pub drum_step: AtomicU8,
    pub loop_state: AtomicU8,
    pub loop_pos: AF32,
    pub loop_beats: AF32,
    pub loop_passes: AtomicU8,
    pub song_pos: AF64,
    pub song_playing: AtomicBool,
    pub song_waiting: AtomicBool,
    pub cpu: AF32,
    pub sample_rate: AtomicU32,
    pub synth_ready: AtomicBool,
    pub beat: AF64,
    pub cc_map: [AtomicU8; 128],
    // wind controllers (EWI, Aerophone…): breath → expression
    pub wind_on: AtomicBool,
    pub breath_src: AtomicU8, // 0 auto, 2/7/11 = that CC, 200 = aftertouch
    pub breath_curve: AF32,   // 0 soft … 0.5 linear … 1 hard
    pub breath_filter: AF32,  // how much breath opens the filter
    pub breath_val: AF32,     // engine → UI
    pub breath_locked: AtomicU8,
    /// incremented by the sound card callback; the watchdog checks it
    pub audio_beats: AtomicU64,
    pub audio_ok: AtomicBool,
}

impl Params {
    pub fn new() -> Self {
        Params {
            master: AF32::new(0.8),
            reverb: AF32::new(0.35),
            chorus: AF32::new(0.0),
            delay_mix: AF32::new(0.0),
            delay_fb: AF32::new(0.35),
            delay_time: AtomicU8::new(3),
            cutoff: AF32::new(1.0),
            resonance: AF32::new(0.1),
            drive: AF32::new(0.0),
            bpm: AF32::new(110.0),
            mode: AtomicU8::new(0),
            split: AtomicU8::new(60),
            layer_vol: AF32::new(0.8),
            transpose: AtomicI32::new(0),
            scale_mask: AtomicU16::new(0),
            scale_snap: AtomicBool::new(false),
            arp_on: AtomicBool::new(false),
            arp_mode: AtomicU8::new(0),
            arp_rate: AtomicU8::new(3),
            arp_oct: AtomicU8::new(1),
            arp_latch: AtomicBool::new(false),
            arp_gate: AF32::new(0.6),
            drums_on: AtomicBool::new(false),
            drum_rows: Default::default(),
            drum_len: AtomicU8::new(16),
            drum_kit: AtomicU8::new(0),
            drum_vol: AF32::new(0.8),
            swing: AF32::new(0.0),
            song_speed: AF32::new(1.0),
            song_user: AtomicU16::new(0),
            song_wait: AtomicBool::new(false),
            song_vol: AF32::new(0.8),
            drum_step: AtomicU8::new(255),
            loop_state: AtomicU8::new(0),
            loop_pos: AF32::new(0.0),
            loop_beats: AF32::new(0.0),
            loop_passes: AtomicU8::new(0),
            song_pos: AF64::new(0.0),
            song_playing: AtomicBool::new(false),
            song_waiting: AtomicBool::new(false),
            cpu: AF32::new(0.0),
            sample_rate: AtomicU32::new(48000),
            synth_ready: AtomicBool::new(false),
            beat: AF64::new(0.0),
            cc_map: std::array::from_fn(|_| AtomicU8::new(0)),
            wind_on: AtomicBool::new(false),
            breath_src: AtomicU8::new(0),
            breath_curve: AF32::new(0.5),
            breath_filter: AF32::new(0.0),
            breath_val: AF32::new(1.0),
            breath_locked: AtomicU8::new(0),
            audio_beats: AtomicU64::new(0),
            audio_ok: AtomicBool::new(true),
        }
    }
}

pub struct SynthPair {
    pub main: Synthesizer,
    pub song: Synthesizer,
}

pub enum Cmd {
    /// Raw MIDI from a keyboard (or the computer keys / mouse).
    Midi([u8; 3]),
    Program { layer: bool, bank: i32, prog: i32 },
    LoopButton,
    LoopStop,
    LoopClear,
    LoopUndo,
    Panic,
    Synths(Box<SynthPair>),
    LoadSong(Option<Arc<Song>>),
    SongPlay(bool),
    SongSeek(f64),
}

pub enum UiEvt {
    Note { key: u8, vel: u8, src: u8 },
    AllOff { src: u8 },
    Program { bank: i32, prog: i32 },
    Trash(Box<SynthPair>),
    TrashSong(Arc<Song>),
    /// every incoming CC (for the controller view)
    Cc { cc: u8, val: u8 },
    Bend(i16),
    /// drum pad hit: note number on channel 10, or mapped pad index
    Pad { note: u8, vel: u8 },
    /// relative encoder: browse sounds / categories
    Browse(i8),
    Category(i8),
}

#[derive(Clone, Copy)]
struct LoopEv {
    t: u32,
    ch: u8,
    st: u8,
    d1: u8,
    d2: u8,
    pass: u8,
}

const SLICE: usize = 32;
const LOOP_CAP: usize = 60_000;

pub struct Engine {
    p: Arc<Params>,
    rx: Receiver<Cmd>,
    tx: Sender<UiEvt>,
    synths: Option<Box<SynthPair>>,
    sr: f32,
    bl: Vec<f32>,
    br: Vec<f32>,
    sl: Vec<f32>,
    srr: Vec<f32>,
    // fx
    dl: Vec<f32>,
    dr: Vec<f32>,
    dpos: usize,
    svf: [[f32; 2]; 2],
    cut_s: f32,
    applied: [f32; 4], // reverb, chorus, layer vol, drum vol
    applied_kit: i32,
    // programs
    main_prog: (i32, i32),
    layer_prog: (i32, i32),
    // clock
    beat: f64,
    // live input
    sounded_for: [i16; 128],
    to_arp: [bool; 128],
    raw_held: [bool; 128],
    // arp
    arp_keys: [(u8, u8); 32],
    arp_n: usize,
    arp_phys: usize,
    arp_i: usize,
    arp_next: f64,
    arp_off_at: f64,
    arp_cur: [i16; 32],
    arp_cur_n: usize,
    arp_running: bool,
    rng: u32,
    // drums
    drums_were_on: bool,
    drum_origin: f64,
    drum_k: u64,
    // looper
    lp_evs: Vec<LoopEv>,
    lp_sorted: usize,
    lp_state: u8,
    lp_len: u32,
    lp_pos: u32,
    lp_elapsed: u32,
    lp_pass: usize,
    lp_cursor: usize,
    lp_active: [[bool; 128]; 16],
    // song
    song: Option<Arc<Song>>,
    song_cursor: usize,
    song_pos: f64,
    song_playing: bool,
    wait_keys: [bool; 128],
    waiting: bool,
    wait_done_t: f64,
    enc_last: [u8; 128],
    breath_lock: u8,
    wind_was: bool,
}

#[inline]
fn soft_clip(x: f32) -> f32 {
    let a = x.abs();
    if a < 0.85 {
        x
    } else {
        x.signum() * (0.85 + 0.15 * ((a - 0.85) / 0.15).tanh())
    }
}

#[inline]
fn bit(m: u16, i: usize) -> bool {
    m & (1 << i) != 0
}

impl Engine {
    pub fn new(p: Arc<Params>, rx: Receiver<Cmd>, tx: Sender<UiEvt>, sr: f32, max_block: usize) -> Self {
        let dlen = (sr as usize) * 4;
        let mb = max_block.max(4096);
        Engine {
            p,
            rx,
            tx,
            synths: None,
            sr,
            bl: vec![0.0; mb],
            br: vec![0.0; mb],
            sl: vec![0.0; mb],
            srr: vec![0.0; mb],
            dl: vec![0.0; dlen],
            dr: vec![0.0; dlen],
            dpos: 0,
            svf: [[0.0; 2]; 2],
            cut_s: 1.0,
            applied: [-1.0; 4],
            applied_kit: -1,
            main_prog: (0, 0),
            layer_prog: (0, 48),
            beat: 0.0,
            sounded_for: [-1; 128],
            to_arp: [false; 128],
            raw_held: [false; 128],
            arp_keys: [(0, 0); 32],
            arp_n: 0,
            arp_phys: 0,
            arp_i: 0,
            arp_next: 0.0,
            arp_off_at: f64::MAX,
            arp_cur: [-1; 32],
            arp_cur_n: 0,
            arp_running: false,
            rng: 0x1234_5678,
            drums_were_on: false,
            drum_origin: 0.0,
            drum_k: 0,
            lp_evs: Vec::with_capacity(LOOP_CAP),
            lp_sorted: 0,
            lp_state: lstate::EMPTY,
            lp_len: 0,
            lp_pos: 0,
            lp_elapsed: 0,
            lp_pass: 0,
            lp_cursor: 0,
            lp_active: [[false; 128]; 16],
            song: None,
            song_cursor: 0,
            song_pos: 0.0,
            song_playing: false,
            wait_keys: [false; 128],
            waiting: false,
            wait_done_t: -1.0,
            enc_last: [64; 128],
            breath_lock: 0,
            wind_was: false,
        }
    }

    /// Called before the stream starts (never on the audio thread).
    pub fn set_sample_rate(&mut self, sr: f32) {
        if (sr - self.sr).abs() < 1.0 {
            return;
        }
        self.sr = sr;
        let dlen = (sr as usize) * 4;
        self.dl = vec![0.0; dlen];
        self.dr = vec![0.0; dlen];
        self.dpos = 0;
        self.p.sample_rate.store(sr as u32, Relaxed);
    }

    fn vis(&self, key: u8, vel: u8, src: u8) {
        let _ = self.tx.try_send(UiEvt::Note { key, vel, src });
    }

    fn samples_per_beat(&self) -> f64 {
        self.sr as f64 * 60.0 / self.p.bpm.get().clamp(20.0, 400.0) as f64
    }

    // ---------- raw synth helpers ----------
    fn msg(&mut self, ch: i32, st: i32, d1: i32, d2: i32) {
        if let Some(s) = self.synths.as_mut() {
            s.main.process_midi_message(ch, st, d1, d2);
        }
    }
    fn set_program(&mut self, ch: i32, bank: i32, prog: i32) {
        self.msg(ch, 0xB0, 0, bank.clamp(0, 127));
        self.msg(ch, 0xC0, prog, 0);
    }

    fn apply_all_state(&mut self) {
        let (mb, mp) = self.main_prog;
        let (lb, lpg) = self.layer_prog;
        self.set_program(MAIN_CH, mb, mp);
        self.set_program(LAYER_CH, lb, lpg);
        self.applied = [-1.0; 4];
        self.applied_kit = -1;
        self.apply_params();
    }

    fn apply_params(&mut self) {
        let rv = self.p.reverb.get();
        let ch = self.p.chorus.get();
        if (rv - self.applied[0]).abs() > 0.004 {
            self.applied[0] = rv;
            for c in 0..16 {
                self.msg(c, 0xB0, 91, (rv * 127.0) as i32);
            }
        }
        if (ch - self.applied[1]).abs() > 0.004 {
            self.applied[1] = ch;
            for c in 0..16 {
                self.msg(c, 0xB0, 93, (ch * 127.0) as i32);
            }
        }
        let lv = self.p.layer_vol.get();
        if (lv - self.applied[2]).abs() > 0.004 {
            self.applied[2] = lv;
            self.msg(LAYER_CH, 0xB0, 7, (lv * 127.0) as i32);
        }
        let dv = self.p.drum_vol.get();
        if (dv - self.applied[3]).abs() > 0.004 {
            self.applied[3] = dv;
            self.msg(DRUM_CH, 0xB0, 7, (dv * 127.0) as i32);
        }
        let kit = self.p.drum_kit.load(Relaxed) as i32;
        if kit != self.applied_kit {
            self.applied_kit = kit;
            self.msg(DRUM_CH, 0xC0, kit, 0);
        }
    }

    // ---------- looper recording ----------
    fn lp_record(&mut self, ch: u8, st: u8, d1: u8, d2: u8) {
        if self.lp_state != lstate::REC && self.lp_state != lstate::DUB {
            return;
        }
        let (a, b) = PASS_CH[self.lp_pass];
        let tch = match ch as i32 {
            MAIN_CH => a,
            LAYER_CH => b,
            DRUM_CH => 9,
            _ => return,
        };
        let t = if self.lp_state == lstate::REC { self.lp_elapsed } else { self.lp_pos };
        if self.lp_evs.len() < LOOP_CAP {
            self.lp_evs.push(LoopEv { t, ch: tch, st, d1, d2, pass: self.lp_pass as u8 });
            if tch != 9 {
                if st == 0x90 && d2 > 0 {
                    self.lp_active[tch as usize][d1 as usize] = true;
                } else if st == 0x80 || st == 0x90 {
                    self.lp_active[tch as usize][d1 as usize] = false;
                }
            }
        }
    }

    // ---------- note routing (main / layer / split) ----------
    fn route(&mut self, key: u8, vel: u8, on: bool) {
        let mode = self.p.mode.load(Relaxed);
        let split = self.p.split.load(Relaxed);
        let mut chs: [i32; 2] = [-1, -1];
        if on {
            match mode {
                1 => chs = [MAIN_CH, LAYER_CH],
                2 => chs[0] = if key < split { LAYER_CH } else { MAIN_CH },
                _ => chs[0] = MAIN_CH,
            }
        } else {
            // note-off goes everywhere it could have gone (mode may have changed while held)
            chs = [MAIN_CH, LAYER_CH];
        }
        for ch in chs {
            if ch < 0 {
                continue;
            }
            if let Some(s) = self.synths.as_mut() {
                if on {
                    s.main.note_on(ch, key as i32, vel as i32);
                } else {
                    s.main.note_off(ch, key as i32);
                }
            }
            self.lp_record(ch as u8, if on { 0x90 } else { 0x80 }, key, if on { vel } else { 0 });
        }
    }

    fn snap(&self, k: i32) -> i32 {
        let mask = self.p.scale_mask.load(Relaxed);
        if !self.p.scale_snap.load(Relaxed) || mask == 0 {
            return k;
        }
        for d in 0..12 {
            for c in [k - d, k + d] {
                if (0..128).contains(&c) && bit(mask, (c % 12) as usize) {
                    return c;
                }
            }
        }
        k
    }

    fn live_on(&mut self, k: u8, vel: u8) {
        if self.sounded_for[k as usize] >= 0 {
            self.live_off(k);
        }
        self.raw_held[k as usize] = true;
        if self.waiting && self.wait_keys[k as usize] {
            self.wait_keys[k as usize] = false;
            if !self.wait_keys.iter().any(|x| *x) {
                self.waiting = false;
            }
        }
        let t = (k as i32 + self.p.transpose.load(Relaxed)).clamp(0, 127);
        let s = self.snap(t) as u8;
        self.sounded_for[k as usize] = s as i16;
        self.vis(s, vel, src::LIVE);
        if self.p.arp_on.load(Relaxed) {
            self.to_arp[k as usize] = true;
            self.arp_add(s, vel);
        } else {
            self.to_arp[k as usize] = false;
            self.route(s, vel, true);
        }
    }

    fn live_off(&mut self, k: u8) {
        self.raw_held[k as usize] = false;
        let s = self.sounded_for[k as usize];
        if s < 0 {
            return;
        }
        self.sounded_for[k as usize] = -1;
        let s = s as u8;
        self.vis(s, 0, src::LIVE);
        if self.to_arp[k as usize] {
            self.arp_remove(s);
        } else {
            self.route(s, 0, false);
        }
    }

    fn handle_midi(&mut self, m: [u8; 3]) {
        let st = m[0] & 0xF0;
        let ch = m[0] & 0x0F;
        if ch == 9 && (st == 0x90 || st == 0x80) {
            // drum pads on channel 10 play the drum kit
            let on = st == 0x90 && m[2] > 0;
            let _ = self.tx.try_send(UiEvt::Pad { note: m[1], vel: if on { m[2] } else { 0 } });
            if on {
                self.msg(DRUM_CH, 0x90, m[1] as i32, m[2] as i32);
            } else {
                self.msg(DRUM_CH, 0x80, m[1] as i32, 0);
            }
            self.lp_record(9, if on { 0x90 } else { 0x80 }, m[1], m[2]);
            return;
        }
        if self.p.wind_on.load(Relaxed) {
            let id = match st {
                0xB0 if matches!(m[1], 2 | 7 | 11) => m[1],
                0xD0 | 0xA0 => 200,
                _ => 0,
            };
            if id != 0 {
                let val = if st == 0xD0 { m[1] } else { m[2] };
                let want = self.p.breath_src.load(Relaxed);
                let accept = if want == 0 {
                    // lock onto the first breath-type message we see (EWIs can send several at once)
                    if self.breath_lock == 0 {
                        self.breath_lock = id;
                        self.p.breath_locked.store(id, Relaxed);
                    }
                    self.breath_lock == id
                } else {
                    want == id
                };
                if st == 0xB0 {
                    let _ = self.tx.try_send(UiEvt::Cc { cc: m[1], val });
                }
                if accept {
                    self.breath(val);
                }
                return;
            }
        }
        match st {
            0x90 if m[2] > 0 => self.live_on(m[1] & 0x7F, m[2]),
            0x90 | 0x80 => self.live_off(m[1] & 0x7F),
            0xB0 => {
                let cc = m[1];
                let _ = self.tx.try_send(UiEvt::Cc { cc, val: m[2] });
                let t = self.p.cc_map[(cc & 0x7F) as usize].load(Relaxed);
                if t != target::NONE {
                    self.apply_target(t, cc, m[2]);
                    return; // mapped controls don't leak into the synth
                }
                if cc == 0 || cc == 32 {
                    return; // bank select handled by the browser
                }
                if cc == 123 || cc == 120 {
                    self.panic();
                    return;
                }
                for c in [MAIN_CH, LAYER_CH] {
                    self.msg(c, 0xB0, cc as i32, m[2] as i32);
                    self.lp_record(c as u8, 0xB0, cc, m[2]);
                }
            }
            0xE0 | 0xD0 | 0xA0 => {
                if st == 0xE0 {
                    let v = ((m[2] as i16) << 7 | m[1] as i16) - 8192;
                    let _ = self.tx.try_send(UiEvt::Bend(v));
                }
                for c in [MAIN_CH, LAYER_CH] {
                    self.msg(c, st as i32, m[1] as i32, m[2] as i32);
                    self.lp_record(c as u8, st, m[1], m[2]);
                }
            }
            0xC0 => {
                let bank = self.main_prog.0;
                self.main_prog = (bank, m[1] as i32);
                self.set_program(MAIN_CH, bank, m[1] as i32);
                let _ = self.tx.try_send(UiEvt::Program { bank, prog: m[1] as i32 });
            }
            _ => {}
        }
    }

    /// Breath → expression (CC11) on the playing channels, with a response curve.
    fn breath(&mut self, val: u8) {
        let x = val as f32 / 127.0;
        let c = self.p.breath_curve.get().clamp(0.0, 1.0);
        let e = 2f32.powf((c - 0.5) * 3.0); // soft <1 … hard >1
        let y = x.powf(e);
        self.p.breath_val.set(y);
        let v = (y * 127.0).round() as u8;
        for ch in [MAIN_CH, LAYER_CH] {
            self.msg(ch, 0xB0, 11, v as i32);
            self.lp_record(ch as u8, 0xB0, 11, v);
        }
    }

    /// Relative encoders send small offsets around 64; absolute ones jump around.
    fn enc_dir(&mut self, cc: u8, val: u8) -> i8 {
        let d = if (56..=72).contains(&val) {
            (val as i16 - 64).signum() as i8
        } else {
            (val as i16 - self.enc_last[cc as usize] as i16).signum() as i8
        };
        self.enc_last[cc as usize] = val;
        d
    }

    fn apply_target(&mut self, t: u8, cc: u8, val: u8) {
        let v = val as f32 / 127.0;
        let p = &self.p;
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
            target::BROWSE | target::CATEGORY => {
                let d = self.enc_dir(cc, val);
                if d != 0 {
                    let _ = self.tx.try_send(if t == target::BROWSE { UiEvt::Browse(d) } else { UiEvt::Category(d) });
                }
            }
            x if (target::PAD0..target::PAD0 + DRUM_ROWS as u8).contains(&x) => {
                let row = (x - target::PAD0) as usize;
                if val > 0 {
                    self.msg(DRUM_CH, 0x90, DRUM_NOTES[row] as i32, val as i32);
                    self.lp_record(9, 0x90, DRUM_NOTES[row], val);
                }
                let _ = self.tx.try_send(UiEvt::Pad { note: 36 + row as u8, vel: val });
            }
            _ => {}
        }
    }

    fn panic(&mut self) {
        if let Some(s) = self.synths.as_mut() {
            s.main.note_off_all(true);
            s.song.note_off_all(true);
            for c in 0..16 {
                s.main.process_midi_message(c, 0xB0, 64, 0);
            }
        }
        self.sounded_for = [-1; 128];
        self.raw_held = [false; 128];
        self.arp_n = 0;
        self.arp_phys = 0;
        self.arp_cur_n = 0;
        self.arp_running = false;
        for s in [src::LIVE, src::ARP, src::LOOP] {
            let _ = self.tx.try_send(UiEvt::AllOff { src: s });
        }
    }

    // ---------- arpeggiator ----------
    fn arp_add(&mut self, key: u8, vel: u8) {
        let latch = self.p.arp_latch.load(Relaxed);
        if latch && self.arp_phys == 0 {
            self.arp_n = 0; // fresh chord replaces the latched one
        }
        self.arp_phys += 1;
        if self.arp_keys[..self.arp_n].iter().any(|(k, _)| *k == key) {
            return;
        }
        if self.arp_n < self.arp_keys.len() {
            let mut i = self.arp_n;
            while i > 0 && self.arp_keys[i - 1].0 > key {
                self.arp_keys[i] = self.arp_keys[i - 1];
                i -= 1;
            }
            self.arp_keys[i] = (key, vel);
            self.arp_n += 1;
        }
    }

    fn arp_remove(&mut self, key: u8) {
        self.arp_phys = self.arp_phys.saturating_sub(1);
        if self.p.arp_latch.load(Relaxed) {
            return;
        }
        if let Some(pos) = self.arp_keys[..self.arp_n].iter().position(|(k, _)| *k == key) {
            for i in pos..self.arp_n - 1 {
                self.arp_keys[i] = self.arp_keys[i + 1];
            }
            self.arp_n -= 1;
        }
    }

    fn arp_release(&mut self) {
        for i in 0..self.arp_cur_n {
            let k = self.arp_cur[i];
            if k >= 0 {
                self.route(k as u8, 0, false);
                self.vis(k as u8, 0, src::ARP);
            }
        }
        self.arp_cur_n = 0;
    }

    fn rand(&mut self) -> u32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        x
    }

    fn tick_arp(&mut self, b0: f64, b1: f64) {
        let on = self.p.arp_on.load(Relaxed);
        if !on || (!self.p.arp_latch.load(Relaxed) && self.arp_phys == 0) {
            self.arp_n = 0;
        }
        if self.arp_cur_n > 0 && self.arp_off_at < b1 {
            self.arp_release();
            self.arp_off_at = f64::MAX;
        }
        if !on || self.arp_n == 0 {
            if self.arp_cur_n > 0 {
                self.arp_release();
            }
            self.arp_running = false;
            return;
        }
        let rate = ARP_RATES[(self.p.arp_rate.load(Relaxed) as usize).min(ARP_RATES.len() - 1)].1;
        if !self.arp_running {
            self.arp_running = true;
            self.arp_i = 0;
            self.arp_next = if self.p.drums_on.load(Relaxed) {
                let rel = b0 - self.drum_origin;
                self.drum_origin + (rel / rate).ceil() * rate
            } else {
                b0
            };
        }
        while self.arp_next < b1 {
            self.arp_release();
            let n = self.arp_n;
            let oct = (self.p.arp_oct.load(Relaxed) as usize).clamp(1, 4);
            let len = n * oct;
            let mode = self.p.arp_mode.load(Relaxed);
            let mut picks: [(u8, u8); 32] = [(0, 0); 32];
            let mut np = 0;
            if mode == 4 {
                // chord: whole chord, octave cycling
                let o = self.arp_i % oct;
                for i in 0..n {
                    let (k, v) = self.arp_keys[i];
                    picks[np] = ((k as usize + 12 * o).min(127) as u8, v);
                    np += 1;
                }
            } else {
                let idx = match mode {
                    1 => len - 1 - (self.arp_i % len),
                    2 => {
                        let period = if len > 1 { 2 * len - 2 } else { 1 };
                        let j = self.arp_i % period;
                        if j < len {
                            j
                        } else {
                            period - j
                        }
                    }
                    3 => (self.rand() as usize) % len,
                    _ => self.arp_i % len,
                };
                let (k, v) = self.arp_keys[idx % n];
                picks[0] = ((k as usize + 12 * (idx / n)).min(127) as u8, v);
                np = 1;
            }
            for i in 0..np {
                let (k, v) = picks[i];
                self.route(k, v, true);
                self.vis(k, v, src::ARP);
                self.arp_cur[i] = k as i16;
            }
            self.arp_cur_n = np;
            self.arp_off_at = self.arp_next + rate * self.p.arp_gate.get().clamp(0.05, 1.0) as f64;
            self.arp_i += 1;
            self.arp_next += rate;
        }
    }

    // ---------- drum machine ----------
    fn tick_drums(&mut self, b0: f64, b1: f64) {
        let on = self.p.drums_on.load(Relaxed);
        if on && !self.drums_were_on {
            self.drum_origin = b0;
            self.drum_k = 0;
        }
        if !on && self.drums_were_on {
            self.p.drum_step.store(255, Relaxed);
        }
        self.drums_were_on = on;
        if !on {
            return;
        }
        let swing = self.p.swing.get().clamp(0.0, 1.0) as f64;
        let len = (self.p.drum_len.load(Relaxed) as u64).clamp(1, 16);
        loop {
            let k = self.drum_k;
            let mut t = self.drum_origin + k as f64 * 0.25;
            if k % 2 == 1 {
                t += swing / 12.0;
            }
            if t >= b1 {
                break;
            }
            let step = (k % len) as usize;
            self.p.drum_step.store(step as u8, Relaxed);
            for row in 0..DRUM_ROWS {
                if bit(self.p.drum_rows[row].load(Relaxed), step) {
                    self.msg(DRUM_CH, 0x90, DRUM_NOTES[row] as i32, DRUM_VEL[row]);
                }
            }
            self.drum_k += 1;
        }
    }

    // ---------- looper ----------
    fn lp_set_pass_programs(&mut self) {
        let (a, b) = PASS_CH[self.lp_pass];
        let (mb, mp) = self.main_prog;
        let (lb, lpg) = self.layer_prog;
        self.set_program(a as i32, mb, mp);
        self.set_program(b as i32, lb, lpg);
        let rv = (self.p.reverb.get() * 127.0) as i32;
        let lv = (self.p.layer_vol.get() * 127.0) as i32;
        for c in [a, b] {
            self.msg(c as i32, 0xB0, 91, rv);
            self.msg(c as i32, 0xB0, 7, 100);
        }
        self.msg(b as i32, 0xB0, 7, lv);
    }

    fn lp_close_pass(&mut self, end_t: u32) {
        let (a, b) = PASS_CH[self.lp_pass];
        for ch in [a, b] {
            for k in 0..128 {
                if self.lp_active[ch as usize][k] {
                    self.lp_active[ch as usize][k] = false;
                    if self.lp_evs.len() < LOOP_CAP {
                        self.lp_evs.push(LoopEv { t: end_t, ch, st: 0x80, d1: k as u8, d2: 0, pass: self.lp_pass as u8 });
                    }
                }
            }
            if self.lp_evs.len() < LOOP_CAP {
                self.lp_evs.push(LoopEv { t: end_t, ch, st: 0xB0, d1: 64, d2: 0, pass: self.lp_pass as u8 });
            }
        }
        if self.lp_len > 0 {
            let len = self.lp_len;
            for e in self.lp_evs.iter_mut() {
                e.t %= len;
            }
        }
        self.lp_evs.sort_unstable_by_key(|e| (e.t, if e.st == 0x80 { 0 } else { 1 }));
        self.lp_sorted = self.lp_evs.len();
        self.lp_cursor = self.lp_evs.partition_point(|e| e.t < self.lp_pos);
    }

    fn lp_all_off(&mut self) {
        for (a, b) in PASS_CH {
            for c in [a, b] {
                self.msg(c as i32, 0xB0, 64, 0);
                if let Some(s) = self.synths.as_mut() {
                    s.main.note_off_all_channel(c as i32, false);
                }
            }
        }
        let _ = self.tx.try_send(UiEvt::AllOff { src: src::LOOP });
    }

    fn lp_button(&mut self) {
        match self.lp_state {
            lstate::EMPTY => {
                self.lp_evs.clear();
                self.lp_sorted = 0;
                self.lp_pass = 0;
                self.lp_elapsed = 0;
                self.lp_len = 0;
                self.lp_pos = 0;
                self.lp_active = [[false; 128]; 16];
                self.lp_set_pass_programs();
                self.lp_state = lstate::REC;
            }
            lstate::REC => {
                let spb = self.samples_per_beat();
                let beats = self.lp_elapsed as f64 / spb;
                let q = if beats >= 3.0 { ((beats / 4.0).round().max(1.0)) * 4.0 } else { beats.round().max(1.0) };
                self.lp_len = (q * spb) as u32;
                self.lp_pos = self.lp_elapsed % self.lp_len;
                self.p.loop_beats.set(q as f32);
                let end = self.lp_len.saturating_sub(1).min(self.lp_elapsed);
                self.lp_close_pass(end);
                self.lp_state = lstate::PLAY;
            }
            lstate::PLAY => {
                if self.lp_pass + 1 < MAX_PASSES {
                    self.lp_pass += 1;
                    self.lp_set_pass_programs();
                    self.lp_state = lstate::DUB;
                }
            }
            lstate::DUB => {
                let end = self.lp_pos;
                self.lp_close_pass(end);
                self.lp_state = lstate::PLAY;
            }
            lstate::STOPPED => {
                self.lp_pos = 0;
                self.lp_cursor = 0;
                self.lp_state = lstate::PLAY;
            }
            _ => {}
        }
    }

    fn lp_stop(&mut self) {
        match self.lp_state {
            lstate::REC => {
                self.lp_button();
                self.lp_state = lstate::STOPPED;
            }
            lstate::DUB => {
                let end = self.lp_pos;
                self.lp_close_pass(end);
                self.lp_state = lstate::STOPPED;
            }
            lstate::PLAY => self.lp_state = lstate::STOPPED,
            _ => {}
        }
        self.lp_all_off();
    }

    fn lp_undo(&mut self) {
        match self.lp_state {
            lstate::EMPTY => {}
            lstate::REC => self.lp_clear(),
            _ => {
                if self.lp_state == lstate::DUB {
                    let end = self.lp_pos;
                    self.lp_close_pass(end);
                    self.lp_state = lstate::PLAY;
                }
                if self.lp_pass == 0 {
                    self.lp_clear();
                    return;
                }
                let last = self.lp_pass as u8;
                self.lp_evs.retain(|e| e.pass != last);
                self.lp_sorted = self.lp_evs.len();
                self.lp_cursor = self.lp_evs.partition_point(|e| e.t < self.lp_pos);
                let (a, b) = PASS_CH[self.lp_pass];
                for c in [a, b] {
                    self.msg(c as i32, 0xB0, 64, 0);
                    if let Some(s) = self.synths.as_mut() {
                        s.main.note_off_all_channel(c as i32, false);
                    }
                }
                self.lp_pass -= 1;
                let _ = self.tx.try_send(UiEvt::AllOff { src: src::LOOP });
            }
        }
    }

    fn lp_clear(&mut self) {
        self.lp_all_off();
        self.lp_evs.clear();
        self.lp_sorted = 0;
        self.lp_state = lstate::EMPTY;
        self.lp_pass = 0;
        self.lp_len = 0;
        self.lp_pos = 0;
        self.p.loop_beats.set(0.0);
    }

    fn lp_play_range(&mut self, from: u32, to: u32) {
        while self.lp_cursor < self.lp_sorted {
            let e = self.lp_evs[self.lp_cursor];
            if e.t >= to {
                break;
            }
            if e.t >= from {
                let on = e.st == 0x90 && e.d2 > 0;
                self.msg(e.ch as i32, e.st as i32, e.d1 as i32, e.d2 as i32);
                if e.ch != 9 && (e.st == 0x90 || e.st == 0x80) {
                    self.vis(e.d1, if on { e.d2 } else { 0 }, src::LOOP);
                }
            }
            self.lp_cursor += 1;
        }
    }

    fn tick_looper(&mut self, n: u32) {
        match self.lp_state {
            lstate::REC => {
                self.lp_elapsed = self.lp_elapsed.saturating_add(n);
                let spb = self.samples_per_beat();
                self.p.loop_pos.set(((self.lp_elapsed as f64 / spb) % 4.0 / 4.0) as f32);
            }
            lstate::PLAY | lstate::DUB if self.lp_len > 0 => {
                let end = self.lp_pos + n;
                if end < self.lp_len {
                    self.lp_play_range(self.lp_pos, end);
                    self.lp_pos = end;
                } else {
                    self.lp_play_range(self.lp_pos, self.lp_len);
                    self.lp_cursor = 0;
                    let rest = end - self.lp_len;
                    self.lp_play_range(0, rest);
                    self.lp_pos = rest;
                }
                self.p.loop_pos.set(self.lp_pos as f32 / self.lp_len as f32);
            }
            _ => {}
        }
    }

    // ---------- song player ----------
    fn song_seek(&mut self, t: f64) {
        let Some(song) = self.song.clone() else { return };
        if let Some(s) = self.synths.as_mut() {
            s.song.note_off_all(true);
            s.song.reset_all_controllers();
        }
        let t = t.clamp(0.0, song.dur);
        let cursor = song.evs.partition_point(|e| e.t < t);
        // restore programs / controllers that happened before the seek point
        if let Some(s) = self.synths.as_mut() {
            for e in &song.evs[..cursor] {
                if e.st == 0xC0 || (e.st == 0xB0 && e.d1 != 64) {
                    s.song.process_midi_message(e.ch as i32, e.st as i32, e.d1 as i32, e.d2 as i32);
                }
            }
        }
        self.song_cursor = cursor;
        self.song_pos = t;
        self.waiting = false;
        self.wait_keys = [false; 128];
        self.wait_done_t = t - 0.001;
    }

    fn tick_song(&mut self, n: usize) {
        let Some(song) = self.song.clone() else { return };
        if !self.song_playing {
            return;
        }
        if self.waiting {
            if !self.p.song_wait.load(Relaxed) {
                self.waiting = false;
            } else {
                return;
            }
        }
        let user = self.p.song_user.load(Relaxed);
        let wait = self.p.song_wait.load(Relaxed);
        let dt = n as f64 / self.sr as f64 * self.p.song_speed.get().clamp(0.1, 2.0) as f64;
        let to = self.song_pos + dt;
        while self.song_cursor < song.evs.len() {
            let e = song.evs[self.song_cursor];
            if e.t >= to {
                break;
            }
            let is_user = bit(user, e.ch as usize);
            let is_note = e.st == 0x90 || e.st == 0x80;
            if is_user && wait && e.st == 0x90 && e.d2 > 0 && e.t > self.wait_done_t {
                // stop here until the player hits every note of this chord
                let mut j = self.song_cursor;
                while j < song.evs.len() && song.evs[j].t <= e.t + 0.04 {
                    let f = song.evs[j];
                    if f.st == 0x90 && f.d2 > 0 && bit(user, f.ch as usize) {
                        let k = (f.d1 as i32 - self.p.transpose.load(Relaxed)).clamp(0, 127) as usize;
                        self.wait_keys[k] = true;
                    }
                    j += 1;
                }
                self.wait_done_t = e.t + 0.04;
                self.song_pos = e.t;
                self.waiting = true;
                self.p.song_pos.set(self.song_pos);
                return;
            }
            if !(is_user && is_note) {
                if let Some(s) = self.synths.as_mut() {
                    s.song.process_midi_message(e.ch as i32, e.st as i32, e.d1 as i32, e.d2 as i32);
                }
            }
            self.song_cursor += 1;
        }
        self.song_pos = to;
        if self.song_pos >= song.dur {
            self.song_playing = false;
            self.song_pos = song.dur;
            if let Some(s) = self.synths.as_mut() {
                s.song.note_off_all(false);
            }
        }
        self.p.song_pos.set(self.song_pos);
    }

    // ---------- commands ----------
    fn drain(&mut self) {
        while let Ok(cmd) = self.rx.try_recv() {
            match cmd {
                Cmd::Midi(m) => self.handle_midi(m),
                Cmd::Program { layer, bank, prog } => {
                    if layer {
                        self.layer_prog = (bank, prog);
                        self.set_program(LAYER_CH, bank, prog);
                    } else {
                        self.main_prog = (bank, prog);
                        self.set_program(MAIN_CH, bank, prog);
                    }
                }
                Cmd::LoopButton => self.lp_button(),
                Cmd::LoopStop => self.lp_stop(),
                Cmd::LoopClear => self.lp_clear(),
                Cmd::LoopUndo => self.lp_undo(),
                Cmd::Panic => self.panic(),
                Cmd::Synths(s) => {
                    if let Some(old) = self.synths.replace(s) {
                        let _ = self.tx.try_send(UiEvt::Trash(old));
                    }
                    self.apply_all_state();
                    let t = self.song_pos;
                    self.song_seek(t);
                    self.p.synth_ready.store(true, Relaxed);
                }
                Cmd::LoadSong(s) => {
                    if let Some(sy) = self.synths.as_mut() {
                        sy.song.note_off_all(true);
                        sy.song.reset();
                    }
                    if let Some(old) = std::mem::replace(&mut self.song, s) {
                        let _ = self.tx.try_send(UiEvt::TrashSong(old));
                    }
                    self.song_playing = false;
                    self.song_seek(0.0);
                    self.p.song_pos.set(0.0);
                }
                Cmd::SongPlay(on) => {
                    if on {
                        if let Some(s) = &self.song {
                            if self.song_pos >= s.dur - 0.01 {
                                self.song_seek(0.0);
                            }
                        }
                    } else if let Some(s) = self.synths.as_mut() {
                        s.song.note_off_all(false);
                    }
                    self.song_playing = on && self.song.is_some();
                }
                Cmd::SongSeek(t) => self.song_seek(t),
            }
        }
    }

    /// Render `n` stereo frames into `out_l`/`out_r`.
    pub fn process(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let start = std::time::Instant::now();
        let n = out_l.len();
        self.drain();
        if self.synths.is_none() {
            out_l.fill(0.0);
            out_r.fill(0.0);
            return;
        }
        self.apply_params();
        let wind = self.p.wind_on.load(Relaxed);
        if wind != self.wind_was {
            self.wind_was = wind;
            self.breath_lock = 0;
            self.p.breath_locked.store(0, Relaxed);
            self.p.breath_val.set(1.0);
            for ch in [MAIN_CH, LAYER_CH] {
                self.msg(ch, 0xB0, 11, 127);
            }
        }
        if self.p.breath_src.load(Relaxed) != 0 {
            self.breath_lock = 0;
        }
        let mut done = 0;
        let sr = self.sr;
        while done < n {
            let len = SLICE.min(n - done);
            let bps = self.p.bpm.get().clamp(20.0, 400.0) as f64 / 60.0 / sr as f64;
            let b0 = self.beat;
            let b1 = b0 + len as f64 * bps;
            self.tick_drums(b0, b1);
            self.tick_arp(b0, b1);
            self.tick_looper(len as u32);
            self.tick_song(len);
            self.beat = b1;

            let synths = self.synths.as_mut().unwrap();
            synths.main.render(&mut self.bl[..len], &mut self.br[..len]);
            let have_song = self.song.is_some();
            if have_song {
                synths.song.render(&mut self.sl[..len], &mut self.srr[..len]);
            }

            // ---- master FX chain ----
            let song_vol = self.p.song_vol.get();
            let drive = self.p.drive.get();
            let dg = 1.0 + drive * 12.0;
            let dcomp = 1.0 / (1.0 + drive * 2.5);
            let master = self.p.master.get();
            // filter
            let mut target = self.p.cutoff.get().clamp(0.0, 1.0);
            if wind {
                let bf = self.p.breath_filter.get().clamp(0.0, 1.0);
                target *= 1.0 - bf * 0.85 * (1.0 - self.p.breath_val.get());
            }
            self.cut_s += (target - self.cut_s) * 0.2;
            let filt_on = self.cut_s < 0.995;
            let fc = 40.0 * 500f32.powf(self.cut_s);
            let g = (std::f32::consts::PI * (fc / sr).min(0.49)).tan();
            let k = 2.0 - 1.9 * self.p.resonance.get().clamp(0.0, 1.0);
            let a1 = 1.0 / (1.0 + g * (g + k));
            let a2 = g * a1;
            let a3 = g * a2;
            // delay
            let dmix = self.p.delay_mix.get();
            let dfb = self.p.delay_fb.get().clamp(0.0, 0.92);
            let dbeats = DELAY_TIMES[(self.p.delay_time.load(Relaxed) as usize).min(DELAY_TIMES.len() - 1)].1;
            let dlen = self.dl.len();
            let dsamp = ((dbeats / bps) as usize).clamp(1, dlen - 1);

            for i in 0..len {
                let mut x = [self.bl[i], self.br[i]];
                if have_song {
                    x[0] += self.sl[i] * song_vol;
                    x[1] += self.srr[i] * song_vol;
                }
                for c in 0..2 {
                    let mut v = x[c];
                    if drive > 0.001 {
                        v = (v * dg).tanh() * dcomp;
                    }
                    if filt_on {
                        let s = &mut self.svf[c];
                        let v3 = v - s[1];
                        let v1 = a1 * s[0] + a2 * v3;
                        let v2 = s[1] + a2 * s[0] + a3 * v3;
                        s[0] = 2.0 * v1 - s[0];
                        s[1] = 2.0 * v2 - s[1];
                        v = v2;
                    }
                    x[c] = v;
                }
                let rp = (self.dpos + dlen - dsamp) % dlen;
                let (rl, rr) = (self.dl[rp], self.dr[rp]);
                // ping-pong: each side feeds the other
                self.dl[self.dpos] = (x[0] + x[1]) * 0.5 * (dmix > 0.0) as u8 as f32 + rr * dfb;
                self.dr[self.dpos] = rl * dfb;
                self.dpos = (self.dpos + 1) % dlen;
                // rustysynth runs at ~-6 dB; make up 2x, then a soft knee so big chords don't crackle
                let l = soft_clip((x[0] + rl * dmix) * master * 2.0);
                let r = soft_clip((x[1] + rr * dmix) * master * 2.0);
                out_l[done + i] = l;
                out_r[done + i] = r;
            }
            done += len;
        }
        self.p.beat.set(self.beat);
        self.p.loop_state.store(self.lp_state, Relaxed);
        self.p.loop_passes.store(if self.lp_state == lstate::EMPTY { 0 } else { self.lp_pass as u8 + 1 }, Relaxed);
        self.p.song_playing.store(self.song_playing, Relaxed);
        self.p.song_waiting.store(self.waiting, Relaxed);
        let el = start.elapsed().as_secs_f32();
        let budget = n as f32 / sr;
        let c = self.p.cpu.get();
        self.p.cpu.set(c * 0.95 + (el / budget) * 0.05);
    }
}
