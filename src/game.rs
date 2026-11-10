// Noodle — rhythm-game training mode: drill charts, timing judgement, scoring.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use crate::song::{Song, SongEv, SongNote};
use crate::theory::Scale;
use std::sync::Arc;

pub const PATTERNS: [&str; 7] = ["Up & down", "Thirds", "Groups of 4", "Arpeggio", "Random walk", "Sight-read (random)", "Rhythm only"];
pub const RHYTHMS: [&str; 5] = ["Quarters", "Eighths", "Triplets", "Sixteenths", "Mixed"];

/// Timing windows in real seconds (scaled by song speed when judging).
pub const WIN_PERFECT: f64 = 0.045;
pub const WIN_GREAT: f64 = 0.090;
pub const WIN_GOOD: f64 = 0.140;

pub const J_PENDING: u8 = 0;
pub const J_PERFECT: u8 = 1;
pub const J_GREAT: u8 = 2;
pub const J_GOOD: u8 = 3;
pub const J_MISS: u8 = 4;

#[derive(Clone)]
pub struct DrillCfg {
    pub root_pc: u8,
    pub pattern: usize,
    pub rhythm: usize,
    pub bpm: f32,
    pub octaves: u8,
    pub bars: u8,
    pub bank: i32,
    pub program: i32,
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 16) as u32
    }
    fn range(&mut self, n: usize) -> usize {
        (self.next() as usize) % n.max(1)
    }
}

/// Note lengths (in beats) for one beat of the chosen rhythm. None = rest.
fn beat_cells(rhythm: usize, rng: &mut Rng) -> Vec<Option<f64>> {
    match rhythm {
        0 => vec![Some(1.0)],
        1 => vec![Some(0.5), Some(0.5)],
        2 => vec![Some(1.0 / 3.0); 3],
        3 => vec![Some(0.25); 4],
        _ => {
            let opts: [&[Option<f64>]; 8] = [
                &[Some(1.0)],
                &[Some(0.5), Some(0.5)],
                &[Some(0.75), Some(0.25)],
                &[Some(0.25), Some(0.25), Some(0.5)],
                &[Some(0.5), Some(0.25), Some(0.25)],
                &[None, Some(0.5)],
                &[Some(1.0 / 3.0), Some(1.0 / 3.0), Some(1.0 / 3.0)],
                &[Some(0.25), Some(0.5), Some(0.25)],
            ];
            opts[rng.range(opts.len())].to_vec()
        }
    }
}

/// Build a playable chart. Channel 0 = your notes, channel 9 = the click.
pub fn make_drill(cfg: &DrillCfg, scale: &Scale, name: &str) -> Song {
    let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(7) | 1;
    let mut rng = Rng(seed);
    let root = 60 + cfg.root_pc as i32;
    let oct = cfg.octaves.clamp(1, 3) as i32;
    // all scale degrees across the range, plus the top root
    let mut deg: Vec<i32> = Vec::new();
    for o in 0..oct {
        for s in scale.steps {
            deg.push(root + 12 * o + *s as i32);
        }
    }
    deg.push(root + 12 * oct);
    let n = deg.len();

    // pitch sequence (None for rhythm-only = root every time)
    let mut seq: Vec<i32> = match cfg.pattern {
        0 => {
            let mut v = deg.clone();
            v.extend(deg.iter().rev().skip(1));
            v
        }
        1 => {
            let mut v = Vec::new();
            for i in 0..n.saturating_sub(2) {
                v.push(deg[i]);
                v.push(deg[i + 2]);
            }
            for i in (2..n).rev() {
                v.push(deg[i]);
                v.push(deg[i - 2]);
            }
            v.push(deg[0]);
            v
        }
        2 => {
            let mut v = Vec::new();
            for i in 0..n.saturating_sub(3) {
                v.extend_from_slice(&deg[i..i + 4]);
            }
            for i in (3..n).rev() {
                v.extend(deg[i - 3..=i].iter().rev());
            }
            v.push(deg[0]);
            v
        }
        3 => {
            // broken tonic chord (degrees 1, 3, 5 of the scale) up and down the range
            let per = scale.steps.len();
            let mut tones: Vec<i32> = Vec::new();
            for o in 0..oct as usize {
                for d in [0usize, 2, 4] {
                    if d < per {
                        tones.push(deg[o * per + d]);
                    }
                }
            }
            tones.push(*deg.last().unwrap());
            let mut v = tones.clone();
            v.extend(tones.iter().rev().skip(1));
            v
        }
        _ => Vec::new(), // random / sight-read / rhythm: generated per note below
    };

    let bpm = cfg.bpm.clamp(30.0, 300.0) as f64;
    let spb = 60.0 / bpm;
    let lead_beats = 4.0;
    let mut notes: Vec<SongNote> = Vec::new();
    let mut evs: Vec<SongEv> = vec![
        SongEv { t: 0.0, ch: 0, st: 0xB0, d1: 0, d2: cfg.bank.clamp(0, 127) as u8 },
        SongEv { t: 0.0, ch: 0, st: 0xC0, d1: cfg.program.clamp(0, 127) as u8, d2: 0 },
    ];
    let mut beat = lead_beats;
    let fixed = !seq.is_empty();
    let target_beats = if fixed { f64::MAX } else { cfg.bars.clamp(1, 64) as f64 * 4.0 };
    let mut idx = 0usize;
    let mut walk = (n / 2) as i32;
    'outer: loop {
        for cell in beat_cells(cfg.rhythm, &mut rng) {
            let Some(len) = cell else {
                beat += 0.5;
                continue;
            };
            let key = if fixed {
                if idx >= seq.len() {
                    break 'outer;
                }
                seq[idx]
            } else {
                match cfg.pattern {
                    4 => {
                        // random walk: mostly steps, sometimes skips
                        let step = [-2, -1, -1, 1, 1, 2, 0, 3, -3][rng.range(9)];
                        walk = (walk + step).clamp(0, n as i32 - 1);
                        deg[walk as usize]
                    }
                    5 => deg[rng.range(n)],
                    _ => root,
                }
            };
            idx += 1;
            let start = beat * spb;
            let dur = (len * spb * 0.9).max(0.05);
            let k = key.clamp(0, 127) as u8;
            notes.push(SongNote { start, end: start + dur, key: k, vel: 100, ch: 0 });
            evs.push(SongEv { t: start, ch: 0, st: 0x90, d1: k, d2: 100 });
            evs.push(SongEv { t: start + dur, ch: 0, st: 0x80, d1: k, d2: 0 });
            beat += len;
            if !fixed && beat >= lead_beats + target_beats - 1e-6 {
                break 'outer;
            }
        }
    }
    if fixed {
        seq.clear();
    }
    // click: whole beats from 0 to the end (accent on the bar)
    let end_beat = (beat.ceil() + 1.0).max(lead_beats + 1.0);
    let mut b = 0.0;
    while b < end_beat {
        let accent = (b as i64) % 4 == 0;
        let t = b * spb;
        let k = if accent { 76 } else { 77 }; // GM hi / low wood block
        evs.push(SongEv { t, ch: 9, st: 0x90, d1: k, d2: if accent { 110 } else { 80 } });
        evs.push(SongEv { t: t + 0.05, ch: 9, st: 0x80, d1: k, d2: 0 });
        b += 1.0;
    }
    evs.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
    let dur = end_beat * spb + 0.3;
    let mut programs = [-1i16; 16];
    programs[0] = cfg.program as i16;
    let mut npc = [0u32; 16];
    npc[0] = notes.len() as u32;
    Song { name: name.to_string(), notes, evs, dur, channels: 1, programs, notes_per_ch: npc }
}

pub struct Popup {
    pub text: &'static str,
    pub key: u8,
    pub t: f64,
    pub kind: u8,
}

pub struct Game {
    pub chart: Arc<Song>,
    pub mask: u16,
    pub any_octave: bool,
    pub state: Vec<u8>,
    pub hit_at: Vec<f64>,
    first_pending: usize,
    pub score: u64,
    pub combo: u32,
    pub max_combo: u32,
    pub counts: [u32; 5],
    pub wrong: u32,
    pub popups: Vec<Popup>,
    pub finished: bool,
    pub total: u32,
    pub name: String,
}

impl Game {
    pub fn new(chart: Arc<Song>, mask: u16, any_octave: bool, name: String) -> Self {
        let n = chart.notes.len();
        let total = chart.notes.iter().filter(|x| mask & (1 << x.ch) != 0).count() as u32;
        Game {
            chart,
            mask,
            any_octave,
            state: vec![J_PENDING; n],
            hit_at: vec![0.0; n],
            first_pending: 0,
            score: 0,
            combo: 0,
            max_combo: 0,
            counts: [0; 5],
            wrong: 0,
            popups: Vec::new(),
            finished: false,
            total: total.max(1),
            name,
        }
    }

    fn mine(&self, i: usize) -> bool {
        self.mask & (1 << self.chart.notes[i].ch) != 0
    }

    /// A key was pressed at song time `t` (seconds), wall clock `now`.
    pub fn press(&mut self, key: u8, t: f64, speed: f64, now: f64) {
        if self.finished {
            return;
        }
        let w = WIN_GOOD * speed;
        let mut best: Option<(usize, f64)> = None;
        let mut near = false;
        for i in self.first_pending..self.chart.notes.len() {
            let n = &self.chart.notes[i];
            if n.start > t + w {
                break;
            }
            if self.state[i] != J_PENDING || !self.mine(i) || n.start < t - w {
                continue;
            }
            near = true;
            let ok = if self.any_octave { n.key % 12 == key % 12 } else { n.key == key };
            if ok {
                let d = (n.start - t).abs();
                if best.map(|b| d < b.1).unwrap_or(true) {
                    best = Some((i, d));
                }
            }
        }
        match best {
            Some((i, d)) => {
                let real = d / speed.max(0.05);
                let (j, txt, pts) = if real <= WIN_PERFECT {
                    (J_PERFECT, "PERFECT", 300)
                } else if real <= WIN_GREAT {
                    (J_GREAT, "GREAT", 200)
                } else {
                    (J_GOOD, "GOOD", 100)
                };
                self.state[i] = j;
                self.hit_at[i] = now;
                self.counts[j as usize] += 1;
                self.combo += 1;
                self.max_combo = self.max_combo.max(self.combo);
                let mult = 1 + (self.combo / 25).min(3) as u64;
                self.score += pts * mult;
                self.popups.push(Popup { text: txt, key: self.chart.notes[i].key, t: now, kind: j });
            }
            None => {
                // a stray or wrong note while the chart is running
                let _ = near;
                self.wrong += 1;
                self.combo = 0;
                self.popups.push(Popup { text: "✗", key, t: now, kind: 9 });
            }
        }
    }

    /// Advance: anything that slid past the window is a miss.
    pub fn tick(&mut self, pos: f64, speed: f64, now: f64) {
        let w = WIN_GOOD * speed;
        while self.first_pending < self.chart.notes.len() && self.state[self.first_pending] != J_PENDING {
            self.first_pending += 1;
        }
        for i in self.first_pending..self.chart.notes.len() {
            let n = &self.chart.notes[i];
            if n.start >= pos - w {
                break;
            }
            if self.state[i] == J_PENDING && self.mine(i) {
                self.state[i] = J_MISS;
                self.counts[J_MISS as usize] += 1;
                self.combo = 0;
                self.popups.push(Popup { text: "MISS", key: n.key, t: now, kind: J_MISS });
            }
        }
        self.popups.retain(|p| now - p.t < 0.8);
        if pos >= self.chart.dur - 0.05 {
            self.finished = true;
        }
    }

    pub fn judged(&self) -> u32 {
        self.counts[1] + self.counts[2] + self.counts[3] + self.counts[4]
    }

    pub fn accuracy(&self) -> f64 {
        let done = self.judged().max(1) as f64;
        let raw = (self.counts[1] as f64 + self.counts[2] as f64 * 0.7 + self.counts[3] as f64 * 0.4) / done;
        (raw - self.wrong as f64 / done * 0.25).clamp(0.0, 1.0)
    }

    pub fn grade(&self) -> &'static str {
        let a = self.accuracy();
        if self.counts[4] == 0 && self.wrong == 0 && a > 0.97 {
            "S+"
        } else if a >= 0.95 {
            "S"
        } else if a >= 0.88 {
            "A"
        } else if a >= 0.75 {
            "B"
        } else if a >= 0.6 {
            "C"
        } else {
            "D"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theory::{scale_index, SCALES};

    #[test]
    fn drill_and_judge() {
        let sc = &SCALES[scale_index("Major (Ionian)")];
        let cfg = DrillCfg { root_pc: 0, pattern: 0, rhythm: 0, bpm: 120.0, octaves: 1, bars: 4, bank: 0, program: 0 };
        let song = make_drill(&cfg, sc, "C major");
        // up & down one octave = 8 + 7 notes, first after a one-bar lead-in at 120 bpm
        assert_eq!(song.notes.len(), 15);
        assert!((song.notes[0].start - 2.0).abs() < 1e-9);
        assert_eq!(song.notes[0].key, 60);
        assert_eq!(song.notes[7].key, 72);
        let mut g = Game::new(Arc::new(song), 1, false, "t".into());
        g.press(60, 2.01, 1.0, 0.0); // 10 ms late → perfect
        g.press(62, 2.57, 1.0, 0.0); // 70 ms late → great
        g.press(65, 3.0, 1.0, 0.0); // wrong key → ✗
        g.tick(3.3, 1.0, 0.0); // E (3.0 s) now missed
        assert_eq!(g.counts[J_PERFECT as usize], 1);
        assert_eq!(g.counts[J_GREAT as usize], 1);
        assert_eq!(g.counts[J_MISS as usize], 1);
        assert_eq!(g.wrong, 1);
        assert_eq!(g.combo, 0);
        // every pattern / rhythm builds a non-empty chart in every scale
        for (si, s) in SCALES.iter().enumerate() {
            for p in 0..PATTERNS.len() {
                for r in 0..RHYTHMS.len() {
                    let c = DrillCfg { root_pc: (si % 12) as u8, pattern: p, rhythm: r, bpm: 100.0, octaves: 2, bars: 2, bank: 0, program: 0 };
                    let ch = make_drill(&c, s, "x");
                    assert!(!ch.notes.is_empty(), "{} {} {}", s.name, p, r);
                    assert!(ch.notes.iter().all(|n| n.end > n.start));
                }
            }
        }
    }
}
