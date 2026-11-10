// Noodle — instruments that aren't SoundFont presets: sample libraries (SFZ /
// Decent Sampler) and chip voices. A MIDI channel can hold one of these
// instead of a SoundFont program; the engine sends that channel's notes here.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use std::sync::Arc;

pub enum InstDef {
    Sampler(Arc<crate::sampler::Instrument>),
    Chip(crate::chip::Patch),
    ChipDrums(usize),
    Analog(crate::analog::Patch),
}

pub enum Native {
    Sampler(crate::sampler::Player),
    Chip(Box<crate::chip::Player>),
    Drums(crate::chip::DrumPlayer),
    Analog(Box<crate::analog::Player>),
}

impl Native {
    pub fn new(def: &InstDef, sr: f32) -> Self {
        match def {
            InstDef::Sampler(i) => Native::Sampler(crate::sampler::Player::new(i.clone(), sr)),
            InstDef::Chip(p) => Native::Chip(Box::new(crate::chip::Player::new(*p, sr))),
            InstDef::ChipDrums(k) => Native::Drums(crate::chip::DrumPlayer::new(*k, sr)),
            InstDef::Analog(p) => Native::Analog(Box::new(crate::analog::Player::new(*p, sr))),
        }
    }

    pub fn midi(&mut self, st: i32, d1: i32, d2: i32) {
        let (k, v) = (d1.clamp(0, 127) as u8, d2.clamp(0, 127) as u8);
        match st & 0xF0 {
            0x90 if v > 0 => self.note_on(k, v),
            0x80 | 0x90 => self.note_off(k),
            0xB0 => match self {
                Native::Sampler(p) => p.cc(k, v),
                Native::Chip(p) => p.cc(k, v),
                Native::Drums(p) => p.cc(k, v),
                Native::Analog(p) => p.cc(k, v),
            },
            0xE0 => {
                let v14 = (d2 << 7) | d1;
                match self {
                    Native::Sampler(p) => p.bend(v14),
                    Native::Chip(p) => p.bend(v14),
                    Native::Drums(_) => {}
                    Native::Analog(p) => p.bend(v14),
                }
            }
            _ => {}
        }
    }

    pub fn note_on(&mut self, k: u8, v: u8) {
        match self {
            Native::Sampler(p) => p.note_on(k, v),
            Native::Chip(p) => p.note_on(k, v),
            Native::Drums(p) => p.note_on(k, v),
            Native::Analog(p) => p.note_on(k, v),
        }
    }

    pub fn note_off(&mut self, k: u8) {
        match self {
            Native::Sampler(p) => p.note_off(k),
            Native::Chip(p) => p.note_off(k),
            Native::Drums(_) => {}
            Native::Analog(p) => p.note_off(k),
        }
    }

    pub fn all_off(&mut self, hard: bool) {
        match self {
            Native::Sampler(p) => p.all_off(hard),
            Native::Chip(p) => p.all_off(hard),
            Native::Drums(p) => p.all_off(),
            Native::Analog(p) => p.all_off(hard),
        }
    }

    pub fn render(&mut self, l: &mut [f32], r: &mut [f32]) {
        match self {
            Native::Sampler(p) => p.render(l, r),
            Native::Chip(p) => p.render(l, r),
            Native::Drums(p) => p.render(l, r),
            Native::Analog(p) => p.render(l, r),
        }
    }
}

// ---------------------------------------------------------------- reverb
// Freeverb-style (8 combs + 4 allpasses per side) for native instruments,
// which don't get the SoundFont synth's built-in reverb.

struct Comb {
    buf: Vec<f32>,
    i: usize,
    store: f32,
}

struct Allpass {
    buf: Vec<f32>,
    i: usize,
}

pub struct Reverb {
    combs: [Vec<Comb>; 2],
    aps: [Vec<Allpass>; 2],
}

const COMBS: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const APS: [usize; 4] = [556, 441, 341, 225];

impl Reverb {
    pub fn new(sr: f32) -> Self {
        let sc = sr / 44100.0;
        let mk = |spread: usize| -> (Vec<Comb>, Vec<Allpass>) {
            (
                COMBS.iter().map(|&n| Comb { buf: vec![0.0; ((n + spread) as f32 * sc) as usize], i: 0, store: 0.0 }).collect(),
                APS.iter().map(|&n| Allpass { buf: vec![0.0; ((n + spread) as f32 * sc) as usize], i: 0 }).collect(),
            )
        };
        let (cl, al) = mk(0);
        let (cr, ar) = mk(23);
        Reverb { combs: [cl, cr], aps: [al, ar] }
    }

    /// Adds `wet` reverb of (l,r) back into (l,r).
    pub fn process(&mut self, l: &mut [f32], r: &mut [f32], wet: f32) {
        if wet <= 0.001 {
            return;
        }
        let (fb, damp) = (0.84f32, 0.25f32);
        for i in 0..l.len() {
            let input = (l[i] + r[i]) * 0.015;
            for (c, out) in [(0usize, &mut l[i]), (1, &mut r[i])] {
                let mut acc = 0.0;
                for cb in self.combs[c].iter_mut() {
                    let y = cb.buf[cb.i];
                    cb.store = y * (1.0 - damp) + cb.store * damp;
                    cb.buf[cb.i] = input + cb.store * fb;
                    cb.i = (cb.i + 1) % cb.buf.len();
                    acc += y;
                }
                for ap in self.aps[c].iter_mut() {
                    let b = ap.buf[ap.i];
                    ap.buf[ap.i] = acc + b * 0.5;
                    ap.i = (ap.i + 1) % ap.buf.len();
                    acc = b - acc;
                }
                *out += acc * wet * 3.0;
            }
        }
    }
}
