// Noodle — remembers how you left it.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use crate::engine::{Params, DRUM_ROWS};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::Ordering::Relaxed;

#[derive(Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Settings {
    pub soundfont: Option<PathBuf>,
    pub main: (i32, i32),
    pub layer: (i32, i32),
    pub mode: u8,
    pub split: u8,
    pub layer_vol: f32,
    pub transpose: i32,
    pub master: f32,
    pub reverb: f32,
    pub chorus: f32,
    pub delay_mix: f32,
    pub delay_fb: f32,
    pub delay_time: u8,
    pub cutoff: f32,
    pub resonance: f32,
    pub drive: f32,
    pub bpm: f32,
    pub arp_mode: u8,
    pub arp_rate: u8,
    pub arp_oct: u8,
    pub arp_gate: f32,
    pub arp_latch: bool,
    pub drum_rows: [u16; DRUM_ROWS],
    pub drum_len: u8,
    pub drum_kit: u8,
    pub drum_vol: f32,
    pub swing: f32,
    pub drum_pattern: usize,
    pub view: u8,
    pub scale_root: usize,
    pub scale_idx: usize,
    pub scale_on: bool,
    pub scale_snap: bool,
    pub kb_octave: i32,
    pub favorites: Vec<(i32, i32)>,
    pub kb_size: u8,
    pub kb_lo: u8,
    pub kb_follow: bool,
    pub layout: u8,     // 0 classic, 1 MiniLab 3
    pub controller: u8, // 0 auto-detect, 1 off, 2 MiniLab 3
    pub score_dark: bool,
    pub hide_modules: bool,
    pub ml3_setup_done: bool,
    pub wind: u8, // 0 auto-detect, 1 off, 2 on
    pub breath_src: u8,
    pub breath_curve: f32,
    pub breath_filter: f32,
    pub wind_setup_done: bool,
    pub written: u8, // staff transposition preset
    pub theme: u8,   // 0 dark, 1 light, 2 follow system
    pub midi_off: Vec<String>,
    pub latency: u32, // frames per buffer
}

impl Default for Settings {
    fn default() -> Self {
        let p = Params::new();
        Settings {
            soundfont: None,
            main: (0, 0),
            layer: (0, 48),
            mode: 0,
            split: 60,
            layer_vol: 0.8,
            transpose: 0,
            master: p.master.get(),
            reverb: p.reverb.get(),
            chorus: p.chorus.get(),
            delay_mix: 0.0,
            delay_fb: 0.35,
            delay_time: 3,
            cutoff: 1.0,
            resonance: 0.1,
            drive: 0.0,
            bpm: 110.0,
            arp_mode: 0,
            arp_rate: 3,
            arp_oct: 1,
            arp_gate: 0.6,
            arp_latch: false,
            drum_rows: crate::drums::PATTERNS[1].1,
            drum_len: 16,
            drum_kit: 0,
            drum_vol: 0.8,
            swing: 0.0,
            drum_pattern: 1,
            view: 2,
            scale_root: 2,
            scale_idx: 0,
            scale_on: false,
            scale_snap: false,
            kb_octave: 4,
            favorites: vec![],
            kb_size: 88,
            kb_lo: 21,
            kb_follow: true,
            layout: 0,
            controller: 0,
            score_dark: false,
            hide_modules: false,
            ml3_setup_done: false,
            wind: 0,
            breath_src: 0,
            breath_curve: 0.5,
            breath_filter: 0.3,
            wind_setup_done: false,
            written: 0,
            theme: 0,
            midi_off: vec![],
            latency: 128,
        }
    }
}

fn path() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from(".")).join("noodle").join("settings.json")
}

impl Settings {
    pub fn load() -> Self {
        std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    pub fn save(&self) {
        let p = path();
        if let Some(d) = p.parent() {
            let _ = std::fs::create_dir_all(d);
        }
        if let Ok(s) = serde_json::to_string_pretty(self) {
            let tmp = p.with_extension("json.tmp");
            if std::fs::write(&tmp, s).is_ok() {
                let _ = std::fs::rename(tmp, p);
            }
        }
    }

    pub fn push_to(&self, p: &Params) {
        p.mode.store(self.mode, Relaxed);
        p.split.store(self.split, Relaxed);
        p.layer_vol.set(self.layer_vol);
        p.transpose.store(self.transpose, Relaxed);
        p.master.set(self.master);
        p.reverb.set(self.reverb);
        p.chorus.set(self.chorus);
        p.delay_mix.set(self.delay_mix);
        p.delay_fb.set(self.delay_fb);
        p.delay_time.store(self.delay_time, Relaxed);
        p.cutoff.set(self.cutoff);
        p.resonance.set(self.resonance);
        p.drive.set(self.drive);
        p.bpm.set(self.bpm);
        p.arp_mode.store(self.arp_mode, Relaxed);
        p.arp_rate.store(self.arp_rate, Relaxed);
        p.arp_oct.store(self.arp_oct, Relaxed);
        p.arp_gate.set(self.arp_gate);
        p.arp_latch.store(self.arp_latch, Relaxed);
        for i in 0..DRUM_ROWS {
            p.drum_rows[i].store(self.drum_rows[i], Relaxed);
        }
        p.drum_len.store(self.drum_len, Relaxed);
        p.drum_kit.store(self.drum_kit, Relaxed);
        p.drum_vol.set(self.drum_vol);
        p.swing.set(self.swing);
        p.scale_snap.store(self.scale_snap, Relaxed);
        p.breath_src.store(self.breath_src, Relaxed);
        p.breath_curve.set(self.breath_curve);
        p.breath_filter.set(self.breath_filter);
    }

    pub fn pull_from(&mut self, p: &Params) {
        self.mode = p.mode.load(Relaxed);
        self.split = p.split.load(Relaxed);
        self.layer_vol = p.layer_vol.get();
        self.transpose = p.transpose.load(Relaxed);
        self.master = p.master.get();
        self.reverb = p.reverb.get();
        self.chorus = p.chorus.get();
        self.delay_mix = p.delay_mix.get();
        self.delay_fb = p.delay_fb.get();
        self.delay_time = p.delay_time.load(Relaxed);
        self.cutoff = p.cutoff.get();
        self.resonance = p.resonance.get();
        self.drive = p.drive.get();
        self.bpm = p.bpm.get();
        self.arp_mode = p.arp_mode.load(Relaxed);
        self.arp_rate = p.arp_rate.load(Relaxed);
        self.arp_oct = p.arp_oct.load(Relaxed);
        self.arp_gate = p.arp_gate.get();
        self.arp_latch = p.arp_latch.load(Relaxed);
        for i in 0..DRUM_ROWS {
            self.drum_rows[i] = p.drum_rows[i].load(Relaxed);
        }
        self.drum_len = p.drum_len.load(Relaxed);
        self.drum_kit = p.drum_kit.load(Relaxed);
        self.drum_vol = p.drum_vol.get();
        self.swing = p.swing.get();
        self.scale_snap = p.scale_snap.load(Relaxed);
        self.breath_src = p.breath_src.load(Relaxed);
        self.breath_curve = p.breath_curve.get();
        self.breath_filter = p.breath_filter.get();
    }
}
