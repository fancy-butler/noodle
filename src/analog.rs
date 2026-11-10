// Noodle — "Synthwave": an analog-style polysynth for 80s sounds.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Per voice: two oscillators (saw, square, PWM, triangle, 7-voice supersaw),
// sub oscillator, noise, a 4-pole resonant ladder filter with its own
// envelope, amp envelope, LFO (vibrato / PWM / filter) and glide. Output
// goes through a stereo Juno-style chorus.

#[derive(Clone, Copy, PartialEq)]
pub enum Wave {
    Saw,
    Square,
    /// pulse with PWM from the LFO
    Pwm,
    Tri,
    /// 7 detuned saws
    Super,
    Off,
}

#[derive(Clone, Copy)]
pub struct Patch {
    pub name: &'static str,
    pub w1: Wave,
    pub w2: Wave,
    /// osc 2: semitones and detune (cents)
    pub semi2: f32,
    pub det2: f32,
    pub mix2: f32,
    pub sub: f32,
    pub noise: f32,
    /// supersaw spread (0..1)
    pub spread: f32,
    pub cutoff: f32,
    pub res: f32,
    pub env_amt: f32,
    pub keytrack: f32,
    pub f_adsr: (f32, f32, f32, f32),
    pub a_adsr: (f32, f32, f32, f32),
    pub vel_filter: f32,
    /// LFO rate Hz, vibrato semis, PWM depth, filter depth (octaves), vibrato delay s
    pub lfo: (f32, f32, f32, f32, f32),
    pub glide: f32,
    pub mono: bool,
    pub chorus: f32,
    pub level: f32,
}

const fn base(name: &'static str) -> Patch {
    Patch {
        name,
        w1: Wave::Saw,
        w2: Wave::Off,
        semi2: 0.0,
        det2: 0.0,
        mix2: 0.0,
        sub: 0.0,
        noise: 0.0,
        spread: 0.3,
        cutoff: 2000.0,
        res: 0.2,
        env_amt: 2.0,
        keytrack: 0.5,
        f_adsr: (0.005, 0.4, 0.4, 0.3),
        a_adsr: (0.005, 0.3, 0.8, 0.3),
        vel_filter: 0.5,
        lfo: (5.0, 0.0, 0.0, 0.0, 0.0),
        glide: 0.0,
        mono: false,
        chorus: 0.0,
        level: 0.5,
    }
}

pub const PATCHES: [Patch; 12] = [
    Patch { w1: Wave::Super, spread: 0.35, cutoff: 3500.0, res: 0.15, env_amt: 1.0, f_adsr: (0.01, 0.6, 0.6, 0.5), a_adsr: (0.01, 0.4, 0.9, 0.5), chorus: 0.6, level: 0.31, ..base("Sunset supersaw") },
    Patch { w1: Wave::Pwm, w2: Wave::Saw, det2: 7.0, mix2: 0.4, cutoff: 1400.0, res: 0.15, env_amt: 0.8, f_adsr: (0.6, 1.0, 0.7, 1.2), a_adsr: (0.5, 1.0, 0.9, 1.4), lfo: (0.6, 0.0, 0.35, 0.0, 0.0), chorus: 1.0, level: 0.15, ..base("Juno pad") },
    Patch { w1: Wave::Saw, w2: Wave::Saw, det2: 9.0, mix2: 0.9, cutoff: 500.0, res: 0.1, env_amt: 3.2, f_adsr: (0.06, 0.35, 0.55, 0.25), a_adsr: (0.03, 0.2, 0.85, 0.25), vel_filter: 0.7, chorus: 0.4, level: 0.18, ..base("Jupiter brass") },
    Patch { w1: Wave::Saw, w2: Wave::Square, semi2: 12.0, det2: 4.0, mix2: 0.35, cutoff: 1800.0, res: 0.3, env_amt: 1.6, f_adsr: (0.005, 0.3, 0.6, 0.3), a_adsr: (0.005, 0.2, 0.9, 0.3), lfo: (5.6, 0.18, 0.0, 0.0, 0.35), glide: 0.06, mono: true, chorus: 0.3, level: 0.32, ..base("Neon lead") },
    Patch { w1: Wave::Saw, sub: 0.7, cutoff: 260.0, res: 0.35, env_amt: 2.6, keytrack: 0.3, f_adsr: (0.002, 0.18, 0.15, 0.08), a_adsr: (0.002, 0.25, 0.7, 0.08), vel_filter: 0.6, level: 0.2, ..base("Outrun bass") },
    Patch { w1: Wave::Square, w2: Wave::Saw, det2: 5.0, mix2: 0.5, cutoff: 600.0, res: 0.45, env_amt: 3.5, f_adsr: (0.001, 0.15, 0.0, 0.12), a_adsr: (0.001, 0.25, 0.0, 0.2), level: 0.45, chorus: 0.5, ..base("Arp pluck") },
    Patch { w1: Wave::Saw, cutoff: 350.0, res: 0.7, env_amt: 3.0, f_adsr: (0.001, 0.22, 0.1, 0.15), a_adsr: (0.001, 0.3, 0.5, 0.15), level: 0.35, ..base("Upside-down arp") },
    Patch { w1: Wave::Saw, w2: Wave::Saw, det2: 12.0, mix2: 1.0, cutoff: 2600.0, res: 0.05, env_amt: 0.4, f_adsr: (0.4, 1.0, 0.8, 1.0), a_adsr: (0.35, 0.5, 0.9, 1.2), lfo: (5.0, 0.04, 0.0, 0.0, 0.5), chorus: 1.0, level: 0.19, ..base("Analog strings") },
    Patch { w1: Wave::Tri, w2: Wave::Square, semi2: 19.0, mix2: 0.25, cutoff: 5000.0, res: 0.1, env_amt: 0.5, f_adsr: (0.001, 0.8, 0.2, 0.8), a_adsr: (0.001, 1.4, 0.0, 1.0), chorus: 0.7, level: 0.27, ..base("Crystal bells") },
    Patch { w1: Wave::Pwm, cutoff: 900.0, res: 0.25, env_amt: 2.0, f_adsr: (0.002, 0.25, 0.3, 0.2), a_adsr: (0.002, 0.3, 0.7, 0.2), lfo: (3.0, 0.0, 0.3, 0.0, 0.0), chorus: 0.6, level: 0.17, ..base("PWM stab") },
    Patch { w1: Wave::Super, w2: Wave::Square, semi2: -12.0, mix2: 0.4, spread: 0.25, cutoff: 1200.0, res: 0.2, env_amt: 1.5, f_adsr: (1.5, 2.0, 0.8, 1.5), a_adsr: (1.2, 1.0, 0.9, 2.0), lfo: (0.25, 0.0, 0.0, 0.6, 0.0), chorus: 0.8, level: 0.5, ..base("Nightdrive pad") },
    Patch { w1: Wave::Square, sub: 0.5, noise: 0.05, cutoff: 700.0, res: 0.55, env_amt: 2.2, f_adsr: (0.002, 0.3, 0.25, 0.2), a_adsr: (0.002, 0.2, 0.8, 0.2), lfo: (6.0, 0.12, 0.0, 0.0, 0.3), glide: 0.12, mono: true, level: 0.25, ..base("Mono synth lead") },
];

#[derive(Clone, Copy, Default)]
struct Adsr {
    stage: u8,
    v: f32,
}

impl Adsr {
    fn tick(&mut self, (a, d, s, r): (f32, f32, f32, f32), dt: f32) -> f32 {
        match self.stage {
            1 => {
                self.v += dt / a.max(0.001);
                if self.v >= 1.0 {
                    self.v = 1.0;
                    self.stage = 2;
                }
            }
            2 => {
                self.v = s + (self.v - s) * (-dt * 4.0 / d.max(0.001)).exp();
                if (self.v - s).abs() < 1e-4 {
                    self.stage = 3;
                }
            }
            3 => self.v = s,
            4 => {
                self.v *= (-dt * 5.0 / r.max(0.003)).exp();
                if self.v < 1e-4 {
                    self.v = 0.0;
                    self.stage = 0;
                }
            }
            _ => {}
        }
        self.v
    }
}

#[derive(Clone, Copy, Default)]
struct Voice {
    key: i16,
    vel: f32,
    hz: f32,
    target_hz: f32,
    ph: [f64; 9],
    sub_ph: f64,
    amp: Adsr,
    flt: Adsr,
    lad: [f32; 4],
    t: f32,
    noise: u32,
}

#[inline]
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let t = t / dt;
        t + t - t * t - 1.0
    } else if t > 1.0 - dt {
        let t = (t - 1.0) / dt;
        t * t + t + t + 1.0
    } else {
        0.0
    }
}

fn saw(ph: f64, dt: f64) -> f32 {
    (2.0 * ph - 1.0 - blep(ph, dt)) as f32
}

fn square(ph: f64, dt: f64, w: f64) -> f32 {
    let mut v = if ph < w { 1.0 } else { -1.0 };
    v += blep(ph, dt);
    v -= blep((ph + 1.0 - w).fract(), dt);
    v as f32
}

const SUPER_DET: [f64; 7] = [0.0, -0.11, 0.11, -0.06, 0.06, -0.02, 0.02];

pub struct Player {
    pub patch: Patch,
    sr: f32,
    v: Vec<Voice>,
    held: Vec<(u8, u8)>,
    bend: f32,
    modw: f32,
    expr: f32,
    vol: f32,
    sustain: bool,
    lfo_ph: f32,
    // chorus
    cbuf: Vec<f32>,
    cpos: usize,
    cph: f32,
    last_hz: f32,
}

impl Player {
    pub fn new(patch: Patch, sr: f32) -> Self {
        let n = if patch.mono { 1 } else { 12 };
        Player {
            patch,
            sr,
            v: vec![Voice { key: -1, noise: 0x1234_5678, ..Default::default() }; n],
            held: Vec::new(),
            bend: 0.0,
            modw: 0.0,
            expr: 1.0,
            vol: 0.79,
            sustain: false,
            lfo_ph: 0.0,
            cbuf: vec![0.0; (sr * 0.05) as usize],
            cpos: 0,
            cph: 0.0,
            last_hz: 0.0,
        }
    }

    fn hz(key: u8) -> f32 {
        440.0 * 2f32.powf((key as f32 - 69.0) / 12.0)
    }

    fn trigger(&mut self, key: u8, vel: u8) {
        let target = Self::hz(key);
        if self.patch.mono {
            let v = &mut self.v[0];
            let legato = v.amp.stage != 0 && v.amp.stage != 4 && self.held.len() > 1;
            v.key = key as i16;
            v.vel = vel as f32 / 127.0;
            v.target_hz = target;
            if self.patch.glide <= 0.0 || v.hz <= 0.0 {
                v.hz = target;
            }
            if !legato {
                v.amp.stage = 1;
                v.flt.stage = 1;
                v.t = 0.0;
            }
            return;
        }
        // free voice, else the quietest releasing one, else the oldest
        let i = self.v.iter().position(|v| v.amp.stage == 0).or_else(|| self.v.iter().enumerate().filter(|(_, v)| v.amp.stage == 4).min_by(|a, b| a.1.amp.v.partial_cmp(&b.1.amp.v).unwrap()).map(|x| x.0)).unwrap_or_else(|| self.v.iter().enumerate().max_by(|a, b| a.1.t.partial_cmp(&b.1.t).unwrap()).map(|x| x.0).unwrap_or(0));
        let v = &mut self.v[i];
        v.key = key as i16;
        v.vel = vel as f32 / 127.0;
        v.target_hz = target;
        v.hz = if self.patch.glide > 0.0 && self.last_hz > 0.0 { self.last_hz } else { target };
        v.amp.stage = 1;
        v.flt.stage = 1;
        v.flt.v = 0.0;
        v.t = 0.0;
        // random start phases: the analog "life"
        for (k, p) in v.ph.iter_mut().enumerate() {
            v.noise = v.noise.wrapping_mul(1664525).wrapping_add(1013904223 + k as u32);
            *p = (v.noise >> 8) as f64 / (1u32 << 24) as f64;
        }
        self.last_hz = target;
    }

    pub fn note_on(&mut self, key: u8, vel: u8) {
        if vel == 0 {
            return self.note_off(key);
        }
        self.held.retain(|h| h.0 != key);
        self.held.push((key, vel));
        self.trigger(key, vel);
    }

    pub fn note_off(&mut self, key: u8) {
        self.held.retain(|h| h.0 != key);
        if self.patch.mono {
            if let Some(&(k, vel)) = self.held.last() {
                if self.v[0].key == key as i16 {
                    self.trigger(k, vel);
                }
                return;
            }
        }
        if self.sustain {
            return;
        }
        for v in self.v.iter_mut().filter(|v| v.key == key as i16 && v.amp.stage != 0) {
            v.amp.stage = 4;
            v.flt.stage = 4;
        }
    }

    pub fn cc(&mut self, n: u8, val: u8) {
        let x = val as f32 / 127.0;
        match n {
            1 => self.modw = x,
            7 => self.vol = x * x,
            11 => self.expr = x.powf(1.6),
            64 => {
                self.sustain = val >= 64;
                if !self.sustain {
                    let held: Vec<u8> = self.held.iter().map(|h| h.0).collect();
                    for v in self.v.iter_mut().filter(|v| v.amp.stage != 0 && !held.contains(&(v.key as u8))) {
                        v.amp.stage = 4;
                        v.flt.stage = 4;
                    }
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
                v.amp = Adsr::default();
                v.key = -1;
            } else if v.amp.stage != 0 {
                v.amp.stage = 4;
                v.flt.stage = 4;
            }
        }
    }

    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len();
        let pt = self.patch;
        let sr = self.sr;
        let dt = 1.0 / sr;
        let any = self.v.iter().any(|v| v.amp.stage != 0);
        if !any && pt.chorus <= 0.0 {
            return;
        }
        let gain = pt.level * self.vol * self.expr;
        let mut mono = vec![0f32; n];
        if any {
            let lfo_inc = pt.lfo.0 * dt;
            let glide_k = if pt.glide > 0.0 { (-dt / pt.glide).exp() } else { 0.0 };
            for v in self.v.iter_mut() {
                if v.amp.stage == 0 {
                    continue;
                }
                let mut lph = self.lfo_ph;
                for i in 0..n {
                    lph = (lph + lfo_inc).fract();
                    let lfo = (lph * std::f32::consts::TAU).sin();
                    v.t += dt;
                    v.hz = v.target_hz + (v.hz - v.target_hz) * glide_k;
                    let vib_amt = if v.t > pt.lfo.4 { pt.lfo.1 } else { pt.lfo.1 * v.t / pt.lfo.4.max(0.001) } + self.modw * 0.35;
                    let hz = v.hz * 2f32.powf((self.bend + lfo * vib_amt) / 12.0);
                    let inc = (hz / sr) as f64;
                    let pw = (0.5 + lfo * pt.lfo.2 * 0.42) as f64;
                    let osc = |w: Wave, ph: &mut [f64; 9], base: usize, inc: f64| -> f32 {
                        match w {
                            Wave::Off => 0.0,
                            Wave::Saw => {
                                let s = saw(ph[base], inc);
                                ph[base] = (ph[base] + inc).fract();
                                s
                            }
                            Wave::Square => {
                                let s = square(ph[base], inc, 0.5);
                                ph[base] = (ph[base] + inc).fract();
                                s
                            }
                            Wave::Pwm => {
                                let s = square(ph[base], inc, pw);
                                ph[base] = (ph[base] + inc).fract();
                                s
                            }
                            Wave::Tri => {
                                let p = ph[base] as f32;
                                ph[base] = (ph[base] + inc).fract();
                                4.0 * (p - 0.5).abs() - 1.0
                            }
                            Wave::Super => {
                                let mut s = 0.0;
                                for k in 0..7 {
                                    let d = 1.0 + SUPER_DET[k] * pt.spread as f64 * 0.5;
                                    let ik = inc * d;
                                    s += saw(ph[k], ik) * if k == 0 { 1.0 } else { 0.62 };
                                    ph[k] = (ph[k] + ik).fract();
                                }
                                s * 0.3
                            }
                        }
                    };
                    let s1 = osc(pt.w1, &mut v.ph, 0, inc);
                    let inc2 = inc * 2f64.powf(((pt.semi2 + pt.det2 / 100.0) / 12.0) as f64);
                    let s2 = if pt.w2 == Wave::Off { 0.0 } else { osc(pt.w2, &mut v.ph, if pt.w1 == Wave::Super { 7 } else { 1 }, inc2) };
                    v.sub_ph = (v.sub_ph + inc * 0.5).fract();
                    let sub = if v.sub_ph < 0.5 { 1.0 } else { -1.0 };
                    v.noise = v.noise.wrapping_mul(1664525).wrapping_add(1013904223);
                    let nz = (v.noise >> 9) as f32 / (1u32 << 22) as f32 - 1.0;
                    let x = s1 + s2 * pt.mix2 + sub * pt.sub + nz * pt.noise;
                    // filter: cutoff × envelope × keytrack × velocity × LFO, 4-pole ladder
                    let fe = v.flt.tick(pt.f_adsr, dt);
                    let oct = fe * pt.env_amt * (1.0 - pt.vel_filter + pt.vel_filter * v.vel) + (v.key as f32 - 60.0) / 12.0 * pt.keytrack + lfo * pt.lfo.3;
                    let fc = (pt.cutoff * 2f32.powf(oct)).clamp(20.0, sr * 0.45);
                    let g = 1.0 - (-std::f32::consts::TAU * fc / sr).exp();
                    let fb = pt.res * 3.9;
                    let inp = (x - fb * v.lad[3]).tanh();
                    v.lad[0] += g * (inp - v.lad[0]);
                    v.lad[1] += g * (v.lad[0] - v.lad[1]);
                    v.lad[2] += g * (v.lad[1] - v.lad[2]);
                    v.lad[3] += g * (v.lad[2] - v.lad[3]);
                    let a = v.amp.tick(pt.a_adsr, dt);
                    mono[i] += v.lad[3] * a * (0.4 + 0.6 * v.vel) * (1.0 + pt.res * 0.8);
                }
                if v.amp.stage == 0 {
                    v.key = -1;
                }
            }
            self.lfo_ph = (self.lfo_ph + lfo_inc * n as f32).fract();
        }
        // Juno-style chorus: two delay lines swept in opposite directions
        let len = self.cbuf.len();
        for i in 0..n {
            let x = mono[i] * gain;
            self.cbuf[self.cpos] = x;
            let (l, r) = if pt.chorus > 0.0 {
                self.cph = (self.cph + 0.5 * dt).fract();
                let m = (self.cph * std::f32::consts::TAU).sin();
                let tap = |d: f32, buf: &Vec<f32>, pos: usize| {
                    let ds = d * sr;
                    let i0 = ds as usize;
                    let fr = ds - i0 as f32;
                    let a = buf[(pos + len - i0) % len];
                    let b = buf[(pos + len - i0 - 1) % len];
                    a + (b - a) * fr
                };
                let dl = tap(0.0035 + 0.0018 * m, &self.cbuf, self.cpos);
                let dr = tap(0.0035 - 0.0018 * m, &self.cbuf, self.cpos);
                let c = pt.chorus * 0.7;
                (x * (1.0 - c * 0.4) + dl * c, x * (1.0 - c * 0.4) + dr * c)
            } else {
                (x, x)
            };
            self.cpos = (self.cpos + 1) % len;
            out_l[i] += l;
            out_r[i] += r;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_patch_plays() {
        for pt in PATCHES {
            let mut p = Player::new(pt, 48000.0);
            for k in [57, 60, 64] {
                p.note_on(k, 100);
            }
            let mut peak = 0f32;
            for _ in 0..20 {
                let (mut l, mut r) = (vec![0.0; 960], vec![0.0; 960]);
                p.render(&mut l, &mut r);
                peak = l.iter().chain(r.iter()).fold(peak, |a, b| a.max(b.abs()));
            }
            assert!(peak > 0.02 && peak < 1.5 && peak.is_finite(), "{}: {peak}", pt.name);
        }
    }
}
