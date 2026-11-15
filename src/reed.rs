// Noodle — free-reed (accordion) synthesis: reed ranks, registers, musette
// tuning, bellows pressure and bellows-reversal dips.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

/// One reed bank (a "voice" in accordion terms).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Rank {
    /// 16' bassoon reed, an octave down
    L,
    /// 8' clarinet reed
    M,
    /// 8' tuned flat (musette)
    MFlat,
    /// 8' tuned sharp (musette)
    MSharp,
    /// 4' piccolo reed, an octave up
    H,
}

#[derive(Clone, Copy, Debug)]
pub struct Register {
    pub name: &'static str,
    pub ranks: &'static [Rank],
}

/// How wet the M reeds are tuned: beat rate (Hz) between the detuned reeds at A4.
#[derive(Clone, Copy, Debug)]
pub struct Tuning {
    pub beat_a4: f32,
    /// treble brightness: reed filter cutoff as a multiple of the fundamental
    pub bright: f32,
    /// tone chamber (cassotto): softer, rounder L/M reeds
    pub cassotto: bool,
}

#[derive(Clone, Copy, Default)]
struct Voice {
    id: u16,
    on: bool,
    alive: bool,
    bass: bool,
    freq: f32,
    ratios: [f32; 5],
    gains: [f32; 5],
    n: usize,
    ph: [f64; 5],
    env: f32,
    lp1: f32,
    lp2: f32,
    fc: f32,
    age: u32,
}

const MAXV: usize = 40;

pub struct Accordion {
    sr: f32,
    v: [Voice; MAXV],
    treble: Register,
    bass_note: &'static [Rank],
    tuning: Tuning,
    /// bellows pressure 0..1 (expression), smoothed
    pressure: f32,
    p_target: f32,
    /// reversal dip 0..1
    dip: f32,
    push: bool,
    clock: u32,
    dc: [f32; 2],
    noise: u32,
    pub level: f32,
}

#[inline]
fn blep(t: f64, dt: f64) -> f64 {
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

#[inline]
fn saw(ph: f64, dt: f64) -> f64 {
    2.0 * ph - 1.0 - blep(ph, dt)
}

/// A free reed: mostly a narrow pulse (the reed chops the air), some saw body.
#[inline]
fn reed(ph: f64, dt: f64) -> f32 {
    let d = 0.3;
    let p2 = (ph + 1.0 - d).fract();
    let pulse = saw(ph, dt) - saw(p2, dt);
    (0.62 * pulse + 0.38 * saw(ph, dt)) as f32
}

fn mtof(n: f32) -> f32 {
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

impl Accordion {
    pub fn new(sr: f32, treble: Register, bass_note: &'static [Rank], tuning: Tuning) -> Self {
        Accordion { sr, v: [Voice::default(); MAXV], treble, bass_note, tuning, pressure: 0.85, p_target: 0.85, dip: 0.0, push: true, clock: 0, dc: [0.0; 2], noise: 0x1234_5678, level: 0.1 }
    }

    pub fn set_register(&mut self, r: Register) {
        self.treble = r;
    }
    pub fn set_tuning(&mut self, t: Tuning) {
        self.tuning = t;
    }
    pub fn set_bass_ranks(&mut self, r: &'static [Rank]) {
        self.bass_note = r;
    }

    /// Bellows direction. A change makes the brief dip you hear when a player reverses.
    pub fn bellows(&mut self, push: bool) {
        if push != self.push {
            self.push = push;
            self.dip = 1.0;
        }
    }

    /// Bellows pressure / expression, 0..127.
    pub fn pressure(&mut self, v: u8) {
        self.p_target = (v as f32 / 127.0).clamp(0.0, 1.0);
    }

    fn ranks_for(&self, freq: f32, ranks: &[Rank]) -> ([f32; 5], [f32; 5], usize) {
        let beat = self.tuning.beat_a4 * (freq / 440.0).sqrt().clamp(0.5, 2.0);
        let det = 1.0 + beat / freq;
        let mut r = [1.0f32; 5];
        let mut g = [0.0f32; 5];
        let mut n = 0;
        for k in ranks.iter().take(5) {
            let (ratio, gain) = match k {
                Rank::L => (0.5, 0.9),
                Rank::M => (1.0, 1.0),
                Rank::MFlat => (1.0 / det, 0.9),
                Rank::MSharp => (det, 0.9),
                Rank::H => (2.0, 0.55),
            };
            r[n] = ratio;
            g[n] = gain;
            n += 1;
        }
        let norm = 1.0 / (g[..n].iter().map(|x| x * x).sum::<f32>()).sqrt().max(0.5);
        for x in g[..n].iter_mut() {
            *x *= norm;
        }
        (r, g, n)
    }

    fn slot(&mut self) -> usize {
        if let Some(i) = self.v.iter().position(|v| !v.alive) {
            return i;
        }
        // steal the oldest released voice, else the oldest
        let pick = |rel: bool| self.v.iter().enumerate().filter(|(_, v)| !rel || !v.on).min_by_key(|(_, v)| v.age).map(|(i, _)| i);
        pick(true).or_else(|| pick(false)).unwrap_or(0)
    }

    fn start(&mut self, id: u16, note: f32, bass: bool, ranks: &[Rank], bright: f32) {
        let i = self.slot();
        let freq = mtof(note);
        let (ratios, gains, n) = self.ranks_for(freq, ranks);
        self.clock += 1;
        // each voice's reeds start at different phases, like real reeds
        let mut ph = [0.0f64; 5];
        for p in ph.iter_mut() {
            self.noise ^= self.noise << 13;
            self.noise ^= self.noise >> 17;
            self.noise ^= self.noise << 5;
            *p = (self.noise as f64) / (u32::MAX as f64);
        }
        let fc = (freq * bright).clamp(400.0, 9000.0);
        self.v[i] = Voice { id, on: true, alive: true, bass, freq, ratios, gains, n, ph, env: 0.0, lp1: 0.0, lp2: 0.0, fc, age: self.clock };
    }

    /// Treble button / key down. `id` lets the same pitch on two buttons be released separately.
    pub fn treble_on(&mut self, id: u16, note: u8) {
        let r = self.treble.ranks;
        let b = self.tuning.bright * if self.tuning.cassotto { 0.6 } else { 1.0 };
        self.start(id, note as f32, false, r, b);
    }

    /// Bass button: a bass note (bass reeds) or a chord (chord reeds).
    pub fn bass_on(&mut self, id: u16, notes: &[u8], chord: bool) {
        let ranks: &'static [Rank] = if chord { &[Rank::M, Rank::H] } else { self.bass_note };
        for &n in notes {
            self.start(id, n as f32, true, ranks, if chord { 5.0 } else { 4.0 });
        }
    }

    pub fn off(&mut self, id: u16) {
        for v in self.v.iter_mut().filter(|v| v.on && v.id == id) {
            v.on = false;
        }
    }

    pub fn all_off(&mut self, hard: bool) {
        for v in self.v.iter_mut() {
            v.on = false;
            if hard {
                v.alive = false;
            }
        }
    }

    pub fn sounding(&self) -> usize {
        self.v.iter().filter(|v| v.alive).count()
    }

    pub fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        let sr = self.sr;
        let att_t = 1.0 - (-1.0 / (0.018 * sr)).exp();
        let att_b = 1.0 - (-1.0 / (0.035 * sr)).exp();
        let rel = 1.0 - (-1.0 / (0.045 * sr)).exp();
        let p_s = 1.0 - (-1.0 / (0.03 * sr)).exp();
        let dip_d = (-1.0 / (0.03 * sr)).exp();
        if !self.v.iter().any(|v| v.alive) {
            self.pressure = self.p_target;
            self.dip = 0.0;
            return;
        }
        for i in 0..l.len() {
            self.pressure += (self.p_target - self.pressure) * p_s;
            self.dip *= dip_d;
            // air: louder with pressure, a little brighter too
            let air = (self.pressure * (1.0 - 0.75 * self.dip)).max(0.0);
            let amp = air.powf(1.4);
            let mut out = 0.0f32;
            for v in self.v.iter_mut().filter(|v| v.alive) {
                let target = if v.on { 1.0 } else { 0.0 };
                let k = if v.on { if v.bass { att_b } else { att_t } } else { rel };
                v.env += (target - v.env) * k;
                if !v.on && v.env < 0.0005 {
                    v.alive = false;
                    continue;
                }
                let mut s = 0.0f32;
                for j in 0..v.n {
                    let dt = (v.freq * v.ratios[j]) as f64 / sr as f64;
                    v.ph[j] += dt;
                    if v.ph[j] >= 1.0 {
                        v.ph[j] -= 1.0;
                    }
                    s += reed(v.ph[j], dt) * v.gains[j];
                }
                // reed + reed-block tone: two gentle low-passes; pressure opens it up
                let fc = v.fc * (0.6 + 0.5 * air);
                let a = 1.0 - (-std::f32::consts::TAU * fc / sr).exp();
                v.lp1 += (s - v.lp1) * a;
                v.lp2 += (v.lp1 - v.lp2) * a;
                let tone = 0.55 * v.lp2 + 0.45 * v.lp1;
                out += tone * v.env * if v.bass { 0.8 } else { 1.0 };
            }
            // DC blocker
            let y = out - self.dc[0] + 0.995 * self.dc[1];
            self.dc[0] = out;
            self.dc[1] = y;
            let s = y * amp * self.level;
            l[i] += s;
            r[i] += s;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MUS: Register = Register { name: "Musette", ranks: &[Rank::MFlat, Rank::M, Rank::MSharp] };

    #[test]
    fn plays_releases_and_beats() {
        let t = Tuning { beat_a4: 6.0, bright: 7.0, cassotto: false };
        let mut a = Accordion::new(48000.0, MUS, &[Rank::L, Rank::M], t);
        a.treble_on(1, 69);
        let mut l = vec![0.0; 48000];
        let mut r = vec![0.0; 48000];
        a.render(&mut l, &mut r);
        let peak = l.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!(peak > 0.05 && peak < 1.0, "peak {peak}");
        // musette: the loudness should wobble at roughly the beat rate
        let win = 480;
        let rms: Vec<f32> = l[9600..].chunks(win).map(|c| (c.iter().map(|x| x * x).sum::<f32>() / win as f32).sqrt()).collect();
        let (mn, mx) = rms.iter().fold((1.0f32, 0.0f32), |(a, b), x| (a.min(*x), b.max(*x)));
        assert!(mx > mn * 1.3, "no beating: {mn} {mx}");
        a.off(1);
        let mut l2 = vec![0.0; 24000];
        let mut r2 = vec![0.0; 24000];
        a.render(&mut l2, &mut r2);
        assert_eq!(a.sounding(), 0);
        assert!(l2[20000..].iter().all(|x| x.abs() < 1e-3));
    }

    #[test]
    fn bellows_reversal_dips() {
        let t = Tuning { beat_a4: 0.0, bright: 7.0, cassotto: false };
        let mut a = Accordion::new(48000.0, Register { name: "M", ranks: &[Rank::M] }, &[Rank::M], t);
        a.treble_on(1, 60);
        let mut l = vec![0.0; 24000];
        let mut r = vec![0.0; 24000];
        a.render(&mut l, &mut r);
        let before = l[20000..24000].iter().map(|x| x * x).sum::<f32>();
        a.bellows(false);
        let mut l2 = vec![0.0; 4000];
        let mut r2 = vec![0.0; 4000];
        a.render(&mut l2, &mut r2);
        let after = l2.iter().map(|x| x * x).sum::<f32>();
        assert!(after < before * 0.7, "{after} vs {before}");
    }
}
