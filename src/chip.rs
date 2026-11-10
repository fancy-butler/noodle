// Noodle — chiptune voices: NES, Game Boy, C64 SID, Sega Genesis (YM2612 FM),
// Master System (SN76489), Amiga (Paula) and SNES (S-DSP) flavours.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Each patch copies the parts of a chip you can hear: its waveforms, how
// coarse its pitch and volume steps are, how many voices it has, its filter
// and quirks. Hold more notes than the chip has voices and it does what
// chiptune composers did: arpeggiates the chord at frame rate.

#[derive(Clone, Copy, PartialEq)]
pub enum Osc {
    /// pulse with duty (0..1)
    Pulse(f32),
    /// NES triangle: 32-step, 4-bit
    NesTri,
    /// LFSR noise; true = short (metallic) mode
    Noise(bool),
    /// Game Boy wave channel: 32 × 4-bit samples
    GbWave,
    /// SID sawtooth
    SidSaw,
    /// SID pulse with pulse-width modulation (rate Hz, depth)
    SidPwm(f32, f32),
    /// 4-operator FM (index into FM)
    Fm(usize),
    /// 8-bit wavetable (index into tables), played without interpolation
    Table8(usize),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Quant {
    None,
    /// NES 2A03: 11-bit period at 1.789773 MHz (÷16 pulse, ÷32 triangle)
    Nes,
    /// Game Boy: 11-bit, f = 131072 / (2048 − x)
    Gb,
    /// SN76489: 10-bit, f = 3.579545 MHz / (32 n)
    Sms,
}

#[derive(Clone, Copy)]
pub struct Patch {
    pub name: &'static str,
    pub chip: &'static str,
    pub osc: Osc,
    pub voices: usize,
    pub a: f32,
    pub d: f32,
    pub s: f32,
    pub r: f32,
    /// volume steps (16 = 4-bit), 0 = smooth
    pub steps: u8,
    pub quant: Quant,
    /// resonant low-pass (cutoff Hz, resonance 0..1, env amount Hz)
    pub filter: Option<(f32, f32, f32)>,
    /// fixed output low-pass (Hz), 0 = none
    pub lp: f32,
    /// vibrato (rate Hz, depth semitones, delay s)
    pub vib: (f32, f32, f32),
    /// arpeggio speed (Hz) when more notes than voices
    pub arp_hz: f32,
    /// echo (time s, feedback, mix)
    pub echo: (f32, f32, f32),
    /// hard-pan voices alternately left/right (Amiga)
    pub hard_pan: bool,
    /// output bit depth (0 = float)
    pub bits: u8,
    pub level: f32,
}

const fn p(name: &'static str, chip: &'static str, osc: Osc, voices: usize) -> Patch {
    Patch { name, chip, osc, voices, a: 0.002, d: 0.0, s: 1.0, r: 0.04, steps: 16, quant: Quant::None, filter: None, lp: 0.0, vib: (0.0, 0.0, 0.0), arp_hz: 50.0, echo: (0.0, 0.0, 0.0), hard_pan: false, bits: 0, level: 0.5 }
}

pub const PATCHES: [Patch; 20] = [
    Patch { quant: Quant::Nes, arp_hz: 60.0, d: 0.6, s: 0.6, ..p("NES Pulse 12.5%", "NES", Osc::Pulse(0.125), 2) },
    Patch { quant: Quant::Nes, arp_hz: 60.0, d: 0.6, s: 0.6, ..p("NES Pulse 25%", "NES", Osc::Pulse(0.25), 2) },
    Patch { quant: Quant::Nes, arp_hz: 60.0, d: 0.6, s: 0.6, vib: (6.0, 0.15, 0.25), ..p("NES Pulse 50% lead", "NES", Osc::Pulse(0.5), 2) },
    Patch { quant: Quant::Nes, steps: 0, r: 0.01, level: 0.75, ..p("NES Triangle bass", "NES", Osc::NesTri, 1) },
    Patch { quant: Quant::Nes, d: 0.18, s: 0.0, r: 0.05, ..p("NES Noise drums", "NES", Osc::Noise(false), 1) },
    Patch { quant: Quant::Gb, arp_hz: 60.0, d: 0.5, s: 0.7, lp: 9000.0, ..p("Game Boy Pulse 25%", "Game Boy", Osc::Pulse(0.25), 2) },
    Patch { quant: Quant::Gb, lp: 9000.0, level: 0.6, ..p("Game Boy Wave", "Game Boy", Osc::GbWave, 1) },
    Patch { steps: 0, a: 0.004, d: 0.3, s: 0.7, r: 0.2, filter: Some((900.0, 0.55, 2600.0)), vib: (5.5, 0.12, 0.35), ..p("C64 SID Saw lead", "C64 SID", Osc::SidSaw, 3) },
    Patch { steps: 0, d: 0.25, s: 0.8, r: 0.08, filter: Some((700.0, 0.35, 900.0)), ..p("C64 SID PWM bass", "C64 SID", Osc::SidPwm(0.8, 0.35), 3) },
    Patch { steps: 0, d: 0.4, s: 0.5, r: 0.3, filter: Some((2200.0, 0.2, 0.0)), arp_hz: 50.0, ..p("C64 SID arpeggio", "C64 SID", Osc::SidPwm(3.0, 0.2), 1) },
    Patch { steps: 0, r: 0.08, bits: 9, level: 0.55, ..p("Genesis FM bass", "Sega Genesis", Osc::Fm(0), 6) },
    Patch { steps: 0, r: 0.15, bits: 9, vib: (5.5, 0.1, 0.4), ..p("Genesis FM brass", "Sega Genesis", Osc::Fm(1), 6) },
    Patch { steps: 0, r: 0.3, bits: 9, ..p("Genesis FM e-piano", "Sega Genesis", Osc::Fm(2), 6) },
    Patch { steps: 0, r: 0.1, bits: 9, vib: (6.0, 0.15, 0.3), ..p("Genesis FM lead", "Sega Genesis", Osc::Fm(3), 6) },
    Patch { quant: Quant::Sms, d: 0.5, s: 0.7, ..p("Master System square", "Sega Master System", Osc::Pulse(0.5), 3) },
    Patch { steps: 0, r: 0.12, lp: 4400.0, hard_pan: true, bits: 8, ..p("Amiga 8-bit bass", "Amiga", Osc::Table8(0), 4) },
    Patch { steps: 0, a: 0.06, r: 0.4, lp: 4400.0, hard_pan: true, bits: 8, vib: (5.0, 0.08, 0.3), ..p("Amiga 8-bit strings", "Amiga", Osc::Table8(1), 4) },
    Patch { steps: 0, a: 0.08, d: 0.0, s: 1.0, r: 0.6, lp: 6000.0, echo: (0.24, 0.45, 0.35), ..p("SNES strings", "SNES", Osc::Table8(1), 8) },
    Patch { steps: 0, a: 0.03, r: 0.3, lp: 5000.0, echo: (0.18, 0.35, 0.3), vib: (5.2, 0.12, 0.35), ..p("SNES flute", "SNES", Osc::Table8(2), 8) },
    Patch { steps: 0, a: 0.004, d: 0.8, s: 0.0, r: 0.6, lp: 7000.0, echo: (0.3, 0.4, 0.3), ..p("SNES harp", "SNES", Osc::Table8(3), 8) },
];

// ---------------------------------------------------------------- FM

#[derive(Clone, Copy)]
struct Op {
    ratio: f32,
    level: f32,
    a: f32,
    d: f32,
    s: f32,
    r: f32,
    /// velocity → level
    vel: f32,
}

#[derive(Clone, Copy)]
struct FmPatch {
    alg: u8,
    fb: f32,
    ops: [Op; 4],
}

const fn op(ratio: f32, level: f32, a: f32, d: f32, s: f32, r: f32) -> Op {
    Op { ratio, level, a, d, s, r, vel: 0.5 }
}

const FM: [FmPatch; 4] = [
    // bass: two modulator→carrier pairs, punchy decaying modulators
    FmPatch { alg: 4, fb: 0.55, ops: [op(1.0, 1.4, 0.001, 0.18, 0.15, 0.05), op(1.0, 1.0, 0.001, 0.9, 0.7, 0.06), op(2.0, 0.9, 0.001, 0.12, 0.0, 0.05), op(1.0, 0.6, 0.001, 0.6, 0.5, 0.06)] },
    // brass: slower modulator swell
    FmPatch { alg: 4, fb: 0.4, ops: [op(1.0, 1.6, 0.05, 0.4, 0.75, 0.12), op(1.0, 1.0, 0.03, 0.3, 0.9, 0.15), op(1.0, 1.1, 0.06, 0.4, 0.7, 0.12), op(1.0, 0.7, 0.03, 0.3, 0.9, 0.15)] },
    // e-piano: tine (high ratio, fast decay) + body
    FmPatch { alg: 4, fb: 0.0, ops: [op(14.0, 0.7, 0.001, 0.25, 0.0, 0.1), op(1.0, 0.7, 0.001, 2.5, 0.25, 0.3), op(1.0, 0.8, 0.001, 1.2, 0.2, 0.3), op(1.0, 0.8, 0.001, 2.0, 0.3, 0.3)] },
    // lead: one heavy-feedback chain (saw-ish), plus a carrier an octave up
    FmPatch { alg: 1, fb: 0.9, ops: [op(1.0, 1.2, 0.001, 0.5, 0.8, 0.1), op(2.0, 0.6, 0.001, 0.4, 0.6, 0.1), op(1.0, 0.9, 0.001, 0.4, 0.8, 0.1), op(1.0, 1.0, 0.002, 0.3, 0.9, 0.1)] },
];

// ---------------------------------------------------------------- tables

const GB_WAVE: [u8; 32] = [0, 2, 4, 6, 8, 10, 12, 14, 15, 15, 15, 14, 14, 13, 13, 12, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0, 0, 1, 1];

fn make_tables() -> Vec<Vec<f32>> {
    let n = 64;
    let q8 = |v: f32| ((v.clamp(-1.0, 1.0) * 127.0).round() / 127.0);
    let mut t = Vec::new();
    // 0: Amiga bass — square-ish with a soft corner
    t.push((0..n).map(|i| q8(if i < n / 2 { 0.9 } else { -0.9 } * (1.0 - 0.3 * ((i % 32) as f32 / 32.0)))).collect());
    // 1: strings — bright saw with a little 3rd-harmonic dip
    t.push((0..n).map(|i| { let x = i as f32 / n as f32; q8((1.0 - 2.0 * x) * 0.8 + 0.15 * (x * 6.0 * std::f32::consts::PI).sin()) }).collect());
    // 2: flute — mostly sine, a touch of 2nd and 3rd
    t.push((0..n).map(|i| { let x = i as f32 / n as f32 * std::f32::consts::TAU; q8(0.8 * x.sin() + 0.12 * (2.0 * x).sin() + 0.06 * (3.0 * x).sin()) }).collect());
    // 3: harp/pluck — triangle-ish with odd harmonics
    t.push((0..n).map(|i| { let x = i as f32 / n as f32 * std::f32::consts::TAU; q8(0.7 * x.sin() + 0.25 * (3.0 * x).sin() + 0.1 * (5.0 * x).sin()) }).collect());
    t
}

// ---------------------------------------------------------------- player

#[derive(Clone, Copy, Default)]
struct Env {
    stage: u8, // 0 off,1 a,2 d,3 s,4 r
    v: f32,
}

impl Env {
    fn on(&mut self) {
        self.stage = 1;
    }
    fn off(&mut self) {
        if self.stage != 0 {
            self.stage = 4;
        }
    }
    fn tick(&mut self, a: f32, d: f32, s: f32, r: f32, dt: f32) -> f32 {
        match self.stage {
            1 => {
                self.v += dt / a.max(0.0005);
                if self.v >= 1.0 {
                    self.v = 1.0;
                    self.stage = 2;
                }
            }
            2 => {
                if d <= 0.0 {
                    self.v = s;
                    self.stage = 3;
                } else {
                    self.v = s + (self.v - s) * (-dt * 4.0 / d).exp();
                    if (self.v - s).abs() < 0.001 {
                        self.stage = 3;
                    }
                }
            }
            3 => self.v = s,
            4 => {
                self.v *= (-dt * 5.0 / r.max(0.003)).exp();
                if self.v < 0.0005 {
                    self.v = 0.0;
                    self.stage = 0;
                }
            }
            _ => self.v = 0.0,
        }
        self.v
    }
}

#[derive(Clone, Copy, Default)]
struct V {
    key: i16, // -1 = free
    vel: f32,
    phase: f64,
    lfsr: u16,
    noise_out: f32,
    noise_acc: f64,
    env: Env,
    ops: [(f64, Env); 4],
    fb: [f32; 2],
    t_on: f32,
    pwm_phase: f32,
    flt: [f32; 2],
    lp: f32,
    pan: u8,
}

pub struct Player {
    pub patch: Patch,
    sr: f32,
    v: Vec<V>,
    held: Vec<(u8, u8)>,
    arp_i: usize,
    arp_t: f32,
    bend: f32,
    modw: f32,
    expr: f32,
    vol: f32,
    sustain: bool,
    tables: Vec<Vec<f32>>,
    echo_l: Vec<f32>,
    echo_r: Vec<f32>,
    echo_pos: usize,
    echo_lp: [f32; 2],
    out_lp: [f32; 2],
    vib_t: f32,
}

fn quantize(f: f32, q: Quant, tri: bool) -> f32 {
    match q {
        Quant::None => f,
        Quant::Nes => {
            let div = if tri { 32.0 } else { 16.0 };
            let t = (1_789_773.0 / (div * f) - 1.0).round().clamp(8.0, 2047.0);
            1_789_773.0 / (div * (t + 1.0))
        }
        Quant::Gb => {
            let x = (2048.0 - 131_072.0 / f).round().clamp(0.0, 2047.0);
            131_072.0 / (2048.0 - x)
        }
        Quant::Sms => {
            let n = (3_579_545.0 / (32.0 * f)).round().clamp(1.0, 1023.0);
            3_579_545.0 / (32.0 * n)
        }
    }
}

impl Player {
    pub fn new(patch: Patch, sr: f32) -> Self {
        let n = patch.voices.max(1);
        let elen = (sr * 0.6) as usize;
        let mut v = vec![V { key: -1, lfsr: 1, ..Default::default() }; n];
        for (i, x) in v.iter_mut().enumerate() {
            x.pan = (i % 4 == 1 || i % 4 == 2) as u8; // Paula: 0,3 left · 1,2 right
        }
        Player { patch, sr, v, held: Vec::new(), arp_i: 0, arp_t: 0.0, bend: 0.0, modw: 0.0, expr: 1.0, vol: 0.79, sustain: false, tables: make_tables(), echo_l: vec![0.0; elen], echo_r: vec![0.0; elen], echo_pos: 0, echo_lp: [0.0; 2], out_lp: [0.0; 2], vib_t: 0.0 }
    }

    fn assign(&mut self) {
        // voices play the newest notes; if there are more notes than voices the last voice arpeggiates the rest
        let n = self.v.len();
        let h = self.held.len();
        let want: Vec<(u8, u8)> = if h <= n {
            self.held.clone()
        } else {
            let mut w: Vec<(u8, u8)> = self.held[h - (n - 1)..].to_vec();
            let pool = &self.held[..h - (n - 1)];
            w.insert(0, pool[self.arp_i % pool.len()]);
            w
        };
        // keep voices already on a wanted key
        let mut used = vec![false; n];
        let mut pending = Vec::new();
        for &(k, vel) in &want {
            if let Some(i) = self.v.iter().position(|v| v.key == k as i16 && v.env.stage != 4 && v.env.stage != 0) {
                used[i] = true;
            } else {
                pending.push((k, vel));
            }
        }
        for (k, vel) in pending {
            // free voice, else a releasing one, else steal a voice not wanted
            let i = (0..n).find(|&i| !used[i] && self.v[i].env.stage == 0).or_else(|| (0..n).find(|&i| !used[i] && self.v[i].env.stage == 4)).or_else(|| (0..n).find(|&i| !used[i]));
            if let Some(i) = i {
                used[i] = true;
                let retrig = self.v[i].env.stage == 0 || self.v[i].env.stage == 4;
                let v = &mut self.v[i];
                v.key = k as i16;
                v.vel = vel as f32 / 127.0;
                if retrig || h <= n {
                    v.env.on();
                    v.t_on = 0.0;
                    for o in v.ops.iter_mut() {
                        o.1.on();
                        o.1.v = 0.0;
                    }
                    if matches!(self.patch.osc, Osc::Fm(_)) {
                        v.phase = 0.0;
                        for o in v.ops.iter_mut() {
                            o.0 = 0.0;
                        }
                    }
                }
            }
        }
        // voices whose note is no longer wanted release
        let sustain = self.sustain;
        for (i, v) in self.v.iter_mut().enumerate() {
            if !used[i] && v.key >= 0 && v.env.stage != 4 && v.env.stage != 0 && !(sustain && h == 0) {
                v.env.off();
                for o in v.ops.iter_mut() {
                    o.1.off();
                }
            }
        }
    }

    pub fn note_on(&mut self, key: u8, vel: u8) {
        if vel == 0 {
            return self.note_off(key);
        }
        self.held.retain(|h| h.0 != key);
        self.held.push((key, vel));
        self.assign();
    }

    pub fn note_off(&mut self, key: u8) {
        self.held.retain(|h| h.0 != key);
        if !self.sustain || !self.held.is_empty() {
            self.assign();
        }
    }

    pub fn cc(&mut self, n: u8, v: u8) {
        let x = v as f32 / 127.0;
        match n {
            1 => self.modw = x,
            7 => self.vol = x * x,
            11 => self.expr = x.powf(1.6),
            64 => {
                self.sustain = v >= 64;
                if !self.sustain {
                    self.assign();
                }
            }
            120 | 123 => self.all_off(n == 120),
            _ => {}
        }
    }

    pub fn bend(&mut self, v14: i32) {
        self.bend = (v14 - 8192) as f32 / 8192.0 * 2.0;
    }

    pub fn all_off(&mut self, hard: bool) {
        self.held.clear();
        self.sustain = false;
        for v in self.v.iter_mut() {
            if hard {
                v.env = Env::default();
                v.key = -1;
            } else {
                v.env.off();
                for o in v.ops.iter_mut() {
                    o.1.off();
                }
            }
        }
    }

    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len();
        let dt = 1.0 / self.sr;
        let pt = self.patch;
        // chord arpeggio
        if self.held.len() > self.v.len() {
            self.arp_t += n as f32 * dt;
            let step = 1.0 / pt.arp_hz.max(1.0);
            if self.arp_t >= step {
                self.arp_t -= step;
                self.arp_i = self.arp_i.wrapping_add(1);
                self.assign();
            }
        }
        if self.v.iter().all(|v| v.env.stage == 0) && pt.echo.2 <= 0.0 {
            return;
        }
        self.vib_t += n as f32 * dt;
        let gain = pt.level * self.vol * self.expr;
        let lp_coef = if pt.lp > 0.0 { 1.0 - (-std::f32::consts::TAU * pt.lp / self.sr).exp() } else { 1.0 };
        let fm = if let Osc::Fm(i) = pt.osc { Some(FM[i.min(FM.len() - 1)]) } else { None };
        let tables = &self.tables;
        let sr = self.sr;
        let bend = self.bend;
        let modw = self.modw;
        for v in self.v.iter_mut() {
            if v.env.stage == 0 || v.key < 0 {
                continue;
            }
            v.t_on += n as f32 * dt;
            let vib_d = if v.t_on > pt.vib.2 { pt.vib.1 } else { 0.0 } + modw * 0.4;
            let vib = (v.t_on * pt.vib.0.max(5.0) * std::f32::consts::TAU).sin() * vib_d;
            let tri = pt.osc == Osc::NesTri;
            let f0 = 440.0 * 2f32.powf((v.key as f32 - 69.0 + bend + vib) / 12.0);
            let f = quantize(f0, pt.quant, tri);
            let inc = (f / sr) as f64;
            let vel_gain = if pt.steps == 16 { 0.4 + 0.6 * v.vel } else { 0.3 + 0.7 * v.vel };
            let (pl, pr) = if pt.hard_pan { if v.pan == 0 { (1.0, 0.25) } else { (0.25, 1.0) } } else { (1.0, 1.0) };
            for i in 0..n {
                let mut e = v.env.tick(pt.a, pt.d, pt.s, pt.r, dt);
                if pt.steps > 0 && pt.osc != Osc::NesTri {
                    e = (e * (pt.steps - 1) as f32).round() / (pt.steps - 1) as f32;
                }
                if pt.osc == Osc::NesTri {
                    // the NES triangle has no volume control: on or off
                    e = if v.env.stage == 0 { 0.0 } else if v.env.stage == 4 { (e > 0.05) as u8 as f32 } else { 1.0 };
                }
                let ph = v.phase as f32;
                let s = match pt.osc {
                    Osc::Pulse(d) => if ph < d { 1.0 } else { -1.0 },
                    Osc::NesTri => {
                        let step = (ph * 32.0) as i32;
                        let lvl = if step < 16 { 15 - step } else { step - 16 };
                        lvl as f32 / 7.5 - 1.0
                    }
                    Osc::Noise(short) => {
                        // the LFSR runs on its own clock: one of the 2A03's 16 noise periods,
                        // brighter as you go up the keyboard (never tied to the note's pitch)
                        const PERIODS: [f64; 16] = [4.0, 8.0, 16.0, 32.0, 64.0, 96.0, 128.0, 160.0, 202.0, 254.0, 380.0, 508.0, 762.0, 1016.0, 2034.0, 4068.0];
                        let idx = 15 - ((v.key as i32 - 36).clamp(0, 47) / 3) as usize;
                        v.noise_acc += 1_789_773.0 / PERIODS[idx] / sr as f64;
                        let clocks = v.noise_acc.floor() as i32;
                        v.noise_acc -= clocks as f64;
                        for _ in 0..clocks.max(0).min(4096) {
                            let tap = if short { 6 } else { 1 };
                            let b = (v.lfsr ^ (v.lfsr >> tap)) & 1;
                            v.lfsr = (v.lfsr >> 1) | (b << 14);
                        }
                        v.noise_out = if v.lfsr & 1 == 0 { 1.0 } else { -1.0 };
                        v.noise_out
                    }
                    Osc::GbWave => GB_WAVE[(ph * 32.0) as usize % 32] as f32 / 7.5 - 1.0,
                    Osc::SidSaw => 2.0 * ph - 1.0,
                    Osc::SidPwm(rate, depth) => {
                        v.pwm_phase = (v.pwm_phase + rate * dt).fract();
                        let w = 0.5 + depth * 0.45 * (v.pwm_phase * std::f32::consts::TAU).sin();
                        if ph < w { 1.0 } else { -1.0 }
                    }
                    Osc::Table8(t) => {
                        let tb = &tables[t.min(tables.len() - 1)];
                        tb[(ph * tb.len() as f32) as usize % tb.len()]
                    }
                    Osc::Fm(_) => {
                        let fp = fm.unwrap();
                        let mut o = [0f32; 4];
                        let mut ev = [0f32; 4];
                        for k in 0..4 {
                            let opd = fp.ops[k];
                            ev[k] = v.ops[k].1.tick(opd.a, opd.d, opd.s, opd.r, dt) * opd.level * (1.0 - opd.vel + opd.vel * v.vel);
                        }
                        let sinp = |phase: f64, m: f32| ((phase as f32 + m) * std::f32::consts::TAU).sin();
                        let fbm = (v.fb[0] + v.fb[1]) * 0.5 * fp.fb;
                        o[0] = sinp(v.ops[0].0, fbm) * ev[0];
                        v.fb = [v.fb[1], o[0]];
                        let out = match fp.alg {
                            0 => {
                                o[1] = sinp(v.ops[1].0, o[0]) * ev[1];
                                o[2] = sinp(v.ops[2].0, o[1]) * ev[2];
                                sinp(v.ops[3].0, o[2]) * ev[3]
                            }
                            1 => {
                                o[1] = sinp(v.ops[1].0, 0.0) * ev[1];
                                o[2] = sinp(v.ops[2].0, o[0] + o[1]) * ev[2];
                                sinp(v.ops[3].0, o[2]) * ev[3]
                            }
                            2 => {
                                o[1] = sinp(v.ops[1].0, 0.0) * ev[1];
                                o[2] = sinp(v.ops[2].0, o[1]) * ev[2];
                                sinp(v.ops[3].0, o[0] + o[2]) * ev[3]
                            }
                            3 => {
                                o[1] = sinp(v.ops[1].0, o[0]) * ev[1];
                                o[2] = sinp(v.ops[2].0, 0.0) * ev[2];
                                sinp(v.ops[3].0, o[1] + o[2]) * ev[3]
                            }
                            4 => {
                                o[1] = sinp(v.ops[1].0, o[0]) * ev[1];
                                o[2] = sinp(v.ops[2].0, 0.0) * ev[2];
                                (o[1] + sinp(v.ops[3].0, o[2]) * ev[3]) * 0.5
                            }
                            5 => {
                                o[1] = sinp(v.ops[1].0, o[0]) * ev[1];
                                o[2] = sinp(v.ops[2].0, o[0]) * ev[2];
                                (o[1] + o[2] + sinp(v.ops[3].0, o[0]) * ev[3]) / 3.0
                            }
                            6 => {
                                o[1] = sinp(v.ops[1].0, o[0]) * ev[1];
                                (o[1] + sinp(v.ops[2].0, 0.0) * ev[2] + sinp(v.ops[3].0, 0.0) * ev[3]) / 3.0
                            }
                            _ => (o[0] + sinp(v.ops[1].0, 0.0) * ev[1] + sinp(v.ops[2].0, 0.0) * ev[2] + sinp(v.ops[3].0, 0.0) * ev[3]) / 4.0,
                        };
                        for k in 0..4 {
                            v.ops[k].0 = (v.ops[k].0 + inc * fp.ops[k].ratio as f64).fract();
                        }
                        if v.ops.iter().all(|o| o.1.stage == 0) {
                            v.env.stage = 0;
                        }
                        out
                    }
                };
                let mut x = s * e * vel_gain;
                if let Some((fc, res, envamt)) = pt.filter {
                    // SID-ish 12 dB resonant low-pass, cutoff opens with the envelope
                    let cut = (fc + envamt * e).min(sr * 0.45);
                    let g = (std::f32::consts::PI * cut / sr).tan();
                    let k = 2.0 - 1.85 * res;
                    let a1 = 1.0 / (1.0 + g * (g + k));
                    let v1 = a1 * (v.flt[0] + g * (x - v.flt[1]));
                    let v2 = v.flt[1] + g * v1;
                    v.flt[0] = 2.0 * v1 - v.flt[0];
                    v.flt[1] = 2.0 * v2 - v.flt[1];
                    x = v2;
                }
                v.lp += (x - v.lp) * lp_coef;
                let x = v.lp * gain;
                out_l[i] += x * pl;
                out_r[i] += x * pr;
                if !matches!(pt.osc, Osc::Fm(_)) {
                    v.phase = (v.phase + inc).fract();
                }
            }
            if v.env.stage == 0 {
                v.key = -1;
            }
        }
        // DAC bit depth (Genesis ladder / Amiga 8-bit)
        if pt.bits > 0 {
            let q = (1u32 << (pt.bits - 1)) as f32;
            for i in 0..n {
                out_l[i] = (out_l[i] * q).round() / q;
                out_r[i] = (out_r[i] * q).round() / q;
            }
        }
        // SNES echo
        if pt.echo.2 > 0.0 {
            let len = ((pt.echo.0 * self.sr) as usize).clamp(1, self.echo_l.len() - 1);
            for i in 0..n {
                let rp = (self.echo_pos + self.echo_l.len() - len) % self.echo_l.len();
                let (el, er) = (self.echo_l[rp], self.echo_r[rp]);
                // the echo's FIR filter darkens each repeat
                self.echo_lp[0] += (el - self.echo_lp[0]) * 0.35;
                self.echo_lp[1] += (er - self.echo_lp[1]) * 0.35;
                self.echo_l[self.echo_pos] = out_l[i] + self.echo_lp[1] * pt.echo.1;
                self.echo_r[self.echo_pos] = out_r[i] + self.echo_lp[0] * pt.echo.1;
                self.echo_pos = (self.echo_pos + 1) % self.echo_l.len();
                out_l[i] += self.echo_lp[0] * pt.echo.2;
                out_r[i] += self.echo_lp[1] * pt.echo.2;
            }
        }
        let _ = self.out_lp;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nes_pitch_is_quantised_like_hardware() {
        // A4 on a real NES pulse: period 253 → 440.4 Hz
        let f = quantize(440.0, Quant::Nes, false);
        assert!((f - 440.4).abs() < 0.2, "{f}");
    }

    #[test]
    fn every_patch_sounds_and_arpeggiates() {
        for pt in PATCHES {
            let mut p = Player::new(pt, 48000.0);
            for k in [60, 64, 67, 71] {
                p.note_on(k, 100);
            }
            let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
            let mut peak = 0f32;
            for _ in 0..5 {
                l.fill(0.0);
                r.fill(0.0);
                p.render(&mut l, &mut r);
                peak = l.iter().chain(r.iter()).fold(peak, |a, b| a.max(b.abs()));
            }
            assert!(peak > 0.02 && peak < 2.0, "{}: peak {peak}", pt.name);
            // never more sounding voices than the chip has
            assert!(p.v.iter().filter(|v| v.env.stage != 0).count() <= pt.voices, "{}", pt.name);
            for k in [60, 64, 67, 71] {
                p.note_off(k);
            }
        }
    }
}

#[cfg(test)]
mod noise_tests {
    use super::*;
    /// How strongly the signal repeats at the played note's period (1.0 = a pure tone, ~0 = noise).
    pub fn periodicity(key: u8) -> f32 {
        let pt = PATCHES.iter().find(|p| p.name == "NES Noise drums").copied().unwrap();
        let pt = Patch { d: 5.0, s: 1.0, ..pt };
        let mut p = Player::new(pt, 48000.0);
        p.note_on(key, 120);
        let mut x = Vec::new();
        for _ in 0..20 {
            let (mut l, mut r) = (vec![0.0; 960], vec![0.0; 960]);
            p.render(&mut l, &mut r);
            x.extend(l);
        }
        let lag = (48000.0 / (440.0 * 2f32.powf((key as f32 - 69.0) / 12.0))).round() as usize;
        let n = x.len() - lag;
        let num: f32 = (0..n).map(|i| x[i] * x[i + lag]).sum();
        let den: f32 = (0..n).map(|i| x[i] * x[i]).sum();
        num / den.max(1e-9)
    }
    #[test]
    fn noise_has_no_hidden_tone() {
        for k in [48u8, 60, 72] {
            let c = periodicity(k);
            println!("key {k}: correlation at the note's period {c:.3}");
            assert!(c.abs() < 0.2, "noise at key {k} repeats at the note's pitch ({c:.3})");
        }
    }
}
