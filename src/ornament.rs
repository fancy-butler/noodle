// Noodle — ornaments for tunes played on the pipes: Irish (uilleann) and Highland styles.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// The melody of a tune gets the grace notes a piper would put in, worked out from the tune itself
// (its bars, tempo and note lengths). Only the sound changes: the score keeps the plain notes.
//
// Irish (uilleann): cut = the next note up the chanter's scale, tap = the next note down; long roll
// note–cut–note–tap–note, short roll cut–note–tap–note, cran on bottom D/E (cuts by successive
// fingers, since nothing lies below to tap), cuts to separate repeated notes and on some beats.
// Highland (from pipe band embellishment guides): single gracenotes (High G, or D/E on low notes),
// doublings (High G then D below D, E on D, F on E; F on High G; High G on High A), grips
// (Low G – D gracenote – note), taorluaths (… – E gracenote, D and below), D throws, birls on Low A.
// Gracenotes are 20–30 ms, heard as clicks more than pitches. Grips, taorluaths, throws and birls
// are anticipatory: they take their time from the note before.

use crate::song::{Song, SongEv};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Style {
    Irish,
    Highland,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    Air,
    Jig,
    SlipJig,
    Reel,
    Other,
}

/// The ornamented copy of `song`: channel `ch`'s notes replaced by note + grace-note changes.
/// `tonic` is the chanter's tonic as it sounds now (uilleann D, Highland Low A).
pub fn apply(song: &Song, ch: u8, style: Style, tonic: u8) -> Song {
    let mut mel: Vec<(f64, f64, u8, u8)> = song.notes.iter().filter(|n| n.ch == ch).map(|n| (n.start, n.end, n.key, n.vel)).collect();
    mel.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    // one note at a time, as a chanter plays it: chords keep their top note
    let mut m: Vec<(f64, f64, u8, u8)> = Vec::new();
    for n in mel {
        if let Some(last) = m.last_mut() {
            if (n.0 - last.0).abs() < 0.01 {
                if n.2 > last.2 {
                    *last = n;
                }
                continue;
            }
            if last.1 > n.0 {
                last.1 = n.0;
            }
        }
        m.push(n);
    }
    let mut out = song.clone();
    if m.len() < 2 {
        return out;
    }
    let (kind, beat) = shape(song, &m);
    let bar0 = |t: f64| -> f64 { song.bars.iter().rev().find(|b| **b <= t + 0.02).copied().unwrap_or(song.bars.first().copied().unwrap_or(0.0)) };
    let on_beat = |t: f64| -> bool {
        let x = (t - bar0(t)).rem_euclid(beat);
        x < beat * 0.08 || beat - x < beat * 0.08
    };
    // every change of the chanter: (time, key, owner note's end)
    let mut segs: Vec<(f64, u8, f64)> = Vec::new();
    for i in 0..m.len() {
        let (s, e, k, _) = m[i];
        let d = e - s;
        let prev = if i > 0 { Some(m[i - 1]) } else { None };
        let legato_from_prev = prev.map(|p| s - p.1 < 0.03).unwrap_or(false);
        let repeated = prev.map(|p| p.2 == k && legato_from_prev).unwrap_or(false);
        let h = hash(i as u32, k);
        let orn = match style {
            Style::Irish => irish(kind, beat, s, d, k, tonic, repeated, on_beat(s), h),
            Style::Highland => highland(kind, beat, s, d, k, tonic, repeated, on_beat(s), prev.map(|p| (p.2, p.1 - p.0)), h),
        };
        // anticipatory graces take their time from the note before (at most half of it)
        let pre_total: f64 = orn.pre.iter().map(|g| g.1).sum();
        if !orn.pre.is_empty() && legato_from_prev && prev.map(|p| (p.1 - p.0) * 0.5 >= pre_total).unwrap_or(false) {
            let mut t = s - pre_total;
            for (g, len) in &orn.pre {
                segs.push((t, *g, e));
                t += len;
            }
        }
        if orn.on.is_empty() {
            segs.push((s, k, e));
        } else {
            for (g, at) in &orn.on {
                segs.push((s + at.min(d * 0.9), *g, e));
            }
        }
    }
    segs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    // each change sounds until the next one, or until its note ends (a gap = the chanter closed)
    let ends: Vec<f64> = (0..segs.len()).map(|j| match segs.get(j + 1) { Some(n) => n.0.min(segs[j].2).max(segs[j].0 + 0.004), None => segs[j].2 }).collect();
    let segs: Vec<(f64, u8)> = segs.into_iter().map(|x| (x.0, x.1)).collect();
    // rebuild that channel's events: each segment is a note; the next one starts before this one stops
    let mut evs: Vec<SongEv> = song.evs.iter().filter(|e| !(e.ch == ch && (e.st == 0x90 || e.st == 0x80))).copied().collect();
    let vel = m.iter().map(|x| x.3).max().unwrap_or(100).max(1);
    let mut mine: Vec<(f64, u8, SongEv)> = Vec::new();
    for (j, (t, k)) in segs.iter().enumerate() {
        let end = ends[j].max(*t + 0.004);
        mine.push((*t, 1, SongEv { t: *t, ch, st: 0x90, d1: *k, d2: vel }));
        mine.push((end, 0, SongEv { t: end, ch, st: 0x80, d1: *k, d2: 0 }));
    }
    // same time: the new note first, then the old one's off (legato, the chanter never drops)
    mine.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap().then(b.1.cmp(&a.1)));
    evs.extend(mine.into_iter().map(|x| x.2));
    evs.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
    out.evs = evs;
    out
}

/// The tune's kind and the beat (seconds) ornaments line up with.
fn shape(song: &Song, m: &[(f64, f64, u8, u8)]) -> (Kind, f64) {
    let q = 60.0 / song.qpm.max(20.0) as f64;
    let mut ds: Vec<f64> = m.iter().map(|x| x.1 - x.0).collect();
    ds.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let md = ds[ds.len() / 2];
    let bar = if song.bars.len() >= 3 {
        let mut b: Vec<f64> = song.bars.windows(2).map(|w| w[1] - w[0]).collect();
        b.sort_by(|a, b| a.partial_cmp(b).unwrap());
        b[b.len() / 2]
    } else {
        4.0 * q
    };
    let bar_q = bar / q;
    if md >= 0.42 {
        return (Kind::Air, q);
    }
    if (bar_q - 3.0).abs() < 0.3 && (md - bar / 6.0).abs() < bar / 12.0 {
        return (Kind::Jig, bar / 2.0);
    }
    if (bar_q - 4.5).abs() < 0.3 {
        return (Kind::SlipJig, bar / 3.0);
    }
    if (bar_q - 4.0).abs() < 0.3 || (bar_q - 2.0).abs() < 0.3 {
        return (Kind::Reel, q);
    }
    (Kind::Other, q)
}

struct Orn {
    /// before the beat, taking time from the note before: (key, length)
    pre: Vec<(u8, f64)>,
    /// from the beat: (key, offset from the note's start); the last entry is the melody note again
    on: Vec<(u8, f64)>,
}

fn none() -> Orn {
    Orn { pre: Vec::new(), on: Vec::new() }
}

fn hash(i: u32, k: u8) -> u32 {
    let mut x = i.wrapping_mul(2654435761) ^ (k as u32).wrapping_mul(40503);
    x ^= x >> 13;
    x = x.wrapping_mul(0x5bd1e995);
    x ^ (x >> 15)
}

/// Degrees of the chanter's scale (major on the tonic, both octaves).
fn irish_scale(tonic: u8) -> Vec<u8> {
    let mut v = Vec::new();
    for o in 0..3 {
        for s in [0u8, 2, 4, 5, 7, 9, 11] {
            v.push(tonic + o * 12 + s);
        }
    }
    v
}

fn above(scale: &[u8], k: u8) -> u8 {
    scale.iter().copied().find(|x| *x > k).unwrap_or(k + 2)
}
fn below(scale: &[u8], k: u8) -> u8 {
    scale.iter().rev().copied().find(|x| *x < k).unwrap_or(k.saturating_sub(2))
}

#[allow(clippy::too_many_arguments)]
fn irish(kind: Kind, beat: f64, _s: f64, d: f64, k: u8, tonic: u8, repeated: bool, on_beat: bool, h: u32) -> Orn {
    let sc = irish_scale(tonic);
    let g = if kind == Kind::Air { 0.040 } else { 0.026 };
    let cut = above(&sc, k);
    let tap = below(&sc, k);
    let bottom = k == tonic || k == tonic + 2; // D and E: nothing (or almost nothing) to tap below
    let long = match kind {
        Kind::Jig | Kind::SlipJig => d >= beat * 0.9,     // a dotted crotchet
        Kind::Reel => d >= beat * 1.4,                     // a dotted crotchet in a reel
        Kind::Air => d >= beat * 1.9 && h % 3 == 0,         // now and then on a long note
        Kind::Other => d >= beat * 1.4,
    };
    let short = kind == Kind::Reel && d >= beat * 0.9 && d < beat * 1.4;
    let mut o = none();
    if (long || short) && bottom && k == tonic {
        // cran: D, then cuts by the G, F♯ and E fingers between repeats of D
        let step = (d * 0.16).min(0.075);
        o.on.push((k, 0.0));
        for (i, gk) in [tonic + 5, tonic + 4, tonic + 2].iter().enumerate() {
            let at = step * (i as f64 + 1.0);
            o.on.push((*gk, at));
            o.on.push((k, at + g * 0.8));
        }
        return o;
    }
    if long && !bottom {
        o.on = vec![(k, 0.0), (cut, d * 0.30), (k, d * 0.30 + g), (tap, d * 0.60), (k, d * 0.60 + g)];
        return o;
    }
    if short && !bottom {
        o.on = vec![(cut, 0.0), (k, g), (tap, d * 0.5), (k, d * 0.5 + g)];
        return o;
    }
    let cut_it = repeated || (on_beat && h % 10 < if kind == Kind::Air { 5 } else { 4 });
    if cut_it && d > g * 2.5 {
        o.on = vec![(cut, 0.0), (k, g)];
    }
    o
}

#[allow(clippy::too_many_arguments)]
fn highland(kind: Kind, beat: f64, _s: f64, d: f64, k: u8, t: u8, repeated: bool, on_beat: bool, prev: Option<(u8, f64)>, h: u32) -> Orn {
    // chanter notes from Low A = t
    let (lg, la, b, c, dd, e, f, hg, ha) = (t - 2, t, t + 2, t + 4, t + 5, t + 7, t + 9, t + 10, t + 12);
    let g = 0.024;
    let sliver = 0.034; // the melody note between a doubling's two gracenotes
    let mut o = none();
    if d < g * 3.0 {
        return o;
    }
    let long = d >= beat * 0.9;
    // a doubling's second gracenote
    let second = |k: u8| -> Option<u8> {
        if k <= c {
            Some(dd)
        } else if k == dd {
            Some(e)
        } else if k == e {
            Some(f)
        } else if k == f {
            Some(hg)
        } else {
            None
        }
    };
    // anticipatory figures (on a beat, every few beats, coming down to a low note)
    let from_above = prev.map(|p| p.0 > k).unwrap_or(false);
    let prev_len = prev.map(|p| p.1).unwrap_or(0.0);
    if on_beat && prev_len > 0.12 {
        if k == dd && long && h % 3 == 0 {
            // D throw (light): Low G, D gracenote to C, then D
            o.pre = vec![(lg, g * 1.2), (dd, g), (c, g * 1.4)];
            return o;
        }
        if k == la && from_above && h % 5 == 0 && kind != Kind::Air {
            // G gracenote birl: High G gracenote, Low G strike twice, on Low A
            o.pre = vec![(hg, g), (la, g * 0.6), (lg, g * 0.8), (la, g * 0.6), (lg, g * 0.8)];
            return o;
        }
        if k <= dd && from_above && h % 4 == 0 {
            // taorluath: Low G, D gracenote on Low G, E gracenote to the note
            o.pre = vec![(lg, g * 1.3), (dd, g), (lg, g * 0.8), (e, g)];
            return o;
        }
        if from_above && h % 4 == 1 {
            // grip (leumluath): Low G, D gracenote on Low G, then the note
            o.pre = vec![(lg, g * 1.3), (dd, g), (lg, g * 0.9)];
            return o;
        }
    }
    if on_beat && (long || kind == Kind::Air) {
        // doubling: High G gracenote, the note, the second gracenote, the note
        if k == hg {
            o.on = vec![(f, 0.0), (k, g)];
        } else if k == ha {
            o.on = vec![(hg, 0.0), (k, g)];
        } else if let Some(sg) = second(k) {
            o.on = vec![(hg, 0.0), (k, g), (sg, g + sliver), (k, g * 2.0 + sliver)];
        }
        return o;
    }
    if repeated || on_beat {
        // single gracenote: High G (on High G itself, an A gracenote; on High A, a G gracenote)
        let gn = if k == hg {
            ha
        } else if k == ha {
            hg
        } else if repeated && k <= b && h % 2 == 0 {
            dd
        } else {
            hg
        };
        o.on = vec![(gn, 0.0), (k, g)];
    }
    o
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::song::SongNote;

    fn tune(keys: &[(u8, f64)], qpm: f32, bar: f64) -> Song {
        let mut t = 0.0;
        let mut notes = Vec::new();
        let mut evs = Vec::new();
        for (k, l) in keys {
            notes.push(SongNote { start: t, end: t + l, key: *k, vel: 100, ch: 0 });
            evs.push(SongEv { t, ch: 0, st: 0x90, d1: *k, d2: 100 });
            evs.push(SongEv { t: t + l, ch: 0, st: 0x80, d1: *k, d2: 0 });
            t += l;
        }
        let bars = (0..((t / bar) as usize + 1)).map(|i| i as f64 * bar).collect();
        Song { name: "t".into(), notes, evs, dur: t, channels: 1, programs: [-1; 16], notes_per_ch: [0; 16], own_sound: false, qpm, bars }
    }

    #[test]
    fn jig_gets_rolls_and_crans_and_keeps_its_length() {
        // 6/8 at 120 dotted-crotchets… eighth 0.2 s: D3 (cran), G3 (roll), then eighths
        let e = 0.2;
        let s = tune(&[(62, 3.0 * e), (67, 3.0 * e), (69, e), (71, e), (69, e), (67, e), (71, e), (74, e)], 150.0, 6.0 * e);
        let o = apply(&s, 0, Style::Irish, 62);
        let ons: Vec<(f64, u8)> = o.evs.iter().filter(|x| x.st == 0x90).map(|x| (x.t, x.d1)).collect();
        // a cran's cuts G F♯ E on D in the first beat
        assert!(ons.iter().any(|x| x.1 == 67 && x.0 < 0.3) && ons.iter().any(|x| x.1 == 64 && x.0 < 0.3), "{ons:?}");
        // a roll on G: cut A, tap F♯ inside the second beat
        assert!(ons.iter().any(|x| x.1 == 69 && x.0 > 0.6 && x.0 < 1.2) && ons.iter().any(|x| x.1 == 66 && x.0 > 0.6 && x.0 < 1.2), "{ons:?}");
        let last = o.evs.iter().filter(|x| x.st == 0x80).map(|x| x.t).fold(0.0, f64::max);
        assert!((last - s.dur).abs() < 0.01);
        // never two chanter notes at once longer than an instant
        let mut held: Option<(u8, f64)> = None;
        for x in &o.evs {
            if x.st == 0x90 {
                held = Some((x.d1, x.t));
            }
        }
        assert!(held.is_some());
    }

    #[test]
    fn highland_doublings_and_grips() {
        let q = 0.5; // a 4/4 march, crotchet beats
        let s = tune(&[(76, q), (69, q), (71, q), (74, q), (81, q), (69, q), (69, q), (73, q)], 120.0, 4.0 * q);
        let o = apply(&s, 0, Style::Highland, 69);
        let ons: Vec<u8> = o.evs.iter().filter(|x| x.st == 0x90).map(|x| x.d1).collect();
        assert!(ons.contains(&79), "High G gracenotes: {ons:?}");
        assert!(ons.iter().filter(|k| **k == 79).count() >= 3);
    }
}
