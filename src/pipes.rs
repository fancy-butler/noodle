// Noodle — bagpipe presets, shruti box patterns, just intonation.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Drone sets are sensible defaults; real sets vary by maker and player, so the
// UI lets every drone be switched and retuned.

pub struct Pipe {
    pub name: &'static str,
    /// chanter tonic (MIDI note) the drones and just intonation are built on
    pub tonic: u8,
    /// (semitones relative to tonic, label, on by default)
    pub drones: &'static [(i8, &'static str, bool)],
    /// scale name (theory::SCALES) that lights up the chanter's notes
    pub scale: &'static str,
    /// drone timbre: 0 = bright/loud, 1 = medium, 2 = soft/sweet
    pub tone: u8,
    pub note: &'static str,
}

pub const PIPES: &[Pipe] = &[
    Pipe {
        name: "Great Highland Bagpipe",
        tonic: 69,
        drones: &[(-12, "Tenor", true), (-12, "Tenor", true), (-24, "Bass", true)],
        scale: "Mixolydian",
        tone: 0,
        note: "Low A tonic; modern pipe pitch is well sharp of concert A (closer to B♭) — use Tune.",
    },
    Pipe {
        name: "Scottish smallpipes",
        tonic: 69,
        drones: &[(-12, "Tenor", true), (-24, "Bass", true), (-5, "Baritone (5th)", false)],
        scale: "Mixolydian",
        tone: 1,
        note: "Bellows-blown, concert A. The fifth drone is optional on many sets.",
    },
    Pipe {
        name: "Border pipes",
        tonic: 69,
        drones: &[(-12, "Tenor", true), (-12, "Tenor", true), (-24, "Bass", true)],
        scale: "Mixolydian",
        tone: 1,
        note: "Between Highland and smallpipes in volume.",
    },
    Pipe {
        name: "Uilleann pipes",
        tonic: 62,
        drones: &[(0, "Tenor", true), (-12, "Baritone", true), (-24, "Bass", true)],
        scale: "Major (Ionian)",
        tone: 2,
        note: "Concert D set. Two-octave chanter; regulators (chords) come later.",
    },
    Pipe {
        name: "Northumbrian smallpipes",
        tonic: 67,
        drones: &[(-12, "G", true), (-5, "D", true), (-24, "Low G", false)],
        scale: "Major (Ionian)",
        tone: 2,
        note: "Closed, staccato fingering; drone tuning varies by set — adjust as needed.",
    },
    Pipe {
        name: "Galician gaita",
        tonic: 72,
        drones: &[(-24, "Ronco (bass)", true), (-12, "Ronquilla", false)],
        scale: "Major (Ionian)",
        tone: 0,
        note: "Gaita in C; also common in D and B♭ — set the key with Scale.",
    },
    Pipe {
        name: "Asturian gaita",
        tonic: 72,
        drones: &[(-24, "Roncón", true)],
        scale: "Major (Ionian)",
        tone: 0,
        note: "Single bass drone.",
    },
    Pipe {
        name: "Breton biniou kozh",
        tonic: 82,
        drones: &[(-24, "Drone", true)],
        scale: "Major (Ionian)",
        tone: 0,
        note: "Very high and piercing; traditionally paired with the bombarde.",
    },
    Pipe {
        name: "Bulgarian gaida",
        tonic: 62,
        drones: &[(-24, "Ruchilo (bass)", true)],
        scale: "Mixolydian",
        tone: 1,
        note: "Single bass drone; ornament-heavy style.",
    },
    Pipe {
        name: "Medieval market pipes",
        tonic: 69,
        drones: &[(-12, "Tenor", true), (-24, "Bass", true), (-5, "Fifth", false)],
        scale: "Mixolydian",
        tone: 0,
        note: "Marktsackpfeife-style reconstruction: loud, mouth-blown, separate drone stocks. Tunings vary by maker.",
    },
    Pipe {
        name: "Hümmelchen",
        tonic: 67,
        drones: &[(-12, "Tenor", true), (-24, "Bass", false), (-5, "Fifth", true)],
        scale: "Mixolydian",
        tone: 2,
        note: "Small, quiet Renaissance German pipe (after Praetorius). Reconstructions vary.",
    },
    Pipe {
        name: "Leicestershire smallpipes",
        tonic: 62,
        drones: &[(-12, "Tenor", true), (-24, "Bass", true), (-5, "Fifth", false)],
        scale: "Major (Ionian)",
        tone: 2,
        note: "Set in D. Drone tunings vary by maker — switch and retune them to match yours.",
    },
];

/// Synthesised chanter voice per pipe type (same order as PIPES).
///
/// The tone is built from harmonic levels, in dB relative to the fundamental, for
/// harmonics 1–8, then a steady fall-off. Where marked "measured", the levels follow
/// K. & S. MacAdam, "Tone quality and harmonics of bagpipes, flutes and whistles"
/// (2013), which measured sustained notes across each chanter's range:
///   Highland — 3rd, 4th and 5th harmonics ~15 dB over the fundamental, no odd/even pattern
///   Border — 2nd to 6th 5–10 dB over the fundamental, then a steady decline
///   Uilleann — 1st octave: 2nd ~13 dB over, then ~3 dB less per harmonic;
///              2nd octave: fundamental and 3rd strongest, ~5 dB less per harmonic after
///   Northumbrian — odd harmonics 10–15 dB over the even ones; 1st, 3rd, 5th about equal
///   Scottish smallpipes — 3rd, 5th, 7th ~13 dB over the fundamental, the weakest of the eight
/// The rest are estimates from their bore (conical: all harmonics; cylindrical: odd ones
/// strong) and reports of loudness, not measurements.
#[derive(Clone, Copy)]
pub struct Chanter {
    /// cylindrical bore: loudest at the top of the hand; conical: loudest at the bottom
    /// (Common Stock, LBPS, "Dynamics in piping", 2001)
    pub cylindrical: bool,
    /// harmonics 1–8, dB relative to the fundamental
    pub harm: [f32; 8],
    /// second octave, where it differs (uilleann)
    pub harm_hi: Option<[f32; 8]>,
    /// dB lost per harmonic above the 8th
    pub tail: f32,
    /// reed noise
    pub noise: f32,
    /// loudness relative to the Highland chanter
    pub level: f32,
    pub measured: bool,
}

const fn ch(cylindrical: bool, harm: [f32; 8], tail: f32, noise: f32, level: f32, measured: bool) -> Chanter {
    Chanter { cylindrical, harm, harm_hi: None, tail, noise, level, measured }
}

pub const CHANTERS: [Chanter; 12] = [
    // Great Highland: 3rd–5th ~15 dB over the fundamental, loud
    ch(false, [0.0, 10.0, 15.0, 15.0, 15.0, 11.0, 8.0, 5.0], 3.0, 0.020, 1.00, true),
    // Scottish smallpipes: odd harmonics ~13 dB up, fundamental weakest; quiet
    ch(true, [0.0, 3.0, 13.0, 3.0, 13.0, 3.0, 13.0, 2.0], 4.0, 0.010, 0.55, true),
    // Border: 2nd–6th 5–10 dB up, then steady decline
    ch(false, [0.0, 8.0, 9.0, 6.0, 8.0, 6.0, 3.0, 0.0], 3.5, 0.014, 0.80, true),
    // Uilleann: 1st octave 2nd ~13 dB up then −3 dB/harmonic; 2nd octave below
    Chanter {
        cylindrical: false,
        harm: [0.0, 13.0, 10.0, 7.0, 4.0, 1.0, -2.0, -5.0],
        harm_hi: Some([0.0, -2.0, 0.0, -5.0, -10.0, -15.0, -20.0, -25.0]),
        tail: 3.0,
        noise: 0.010,
        level: 0.65,
        measured: true,
    },
    // Northumbrian: odd 10–15 dB over even, 1st/3rd/5th about equal; quiet, pure
    ch(true, [0.0, -12.0, 0.0, -12.0, -1.0, -14.0, -4.0, -16.0], 5.0, 0.006, 0.45, true),
    // Galician gaita (estimate): conical, bright and strong
    ch(false, [0.0, 9.0, 13.0, 12.0, 11.0, 9.0, 7.0, 4.0], 3.0, 0.016, 0.95, false),
    // Asturian gaita (estimate)
    ch(false, [0.0, 8.0, 12.0, 12.0, 10.0, 8.0, 5.0, 2.0], 3.0, 0.016, 0.95, false),
    // Breton biniou kozh (estimate): very high, piercing
    ch(false, [0.0, 6.0, 8.0, 6.0, 3.0, 0.0, -3.0, -6.0], 4.0, 0.022, 0.90, false),
    // Bulgarian gaida (estimate): cylindrical, buzzy single reed
    ch(true, [0.0, 4.0, 12.0, 5.0, 11.0, 5.0, 9.0, 4.0], 3.5, 0.024, 0.80, false),
    // Medieval market pipes (estimate): raw, buzzy
    ch(true, [0.0, 5.0, 12.0, 6.0, 10.0, 5.0, 8.0, 3.0], 3.5, 0.020, 0.90, false),
    // Hümmelchen (estimate): small and soft
    ch(true, [0.0, -6.0, 4.0, -8.0, 2.0, -10.0, -2.0, -12.0], 6.0, 0.008, 0.45, false),
    // Leicestershire smallpipes (estimate from the Scottish smallpipe measurement): cylindrical,
    // slightly conical lower half — a little more even-harmonic body
    ch(true, [0.0, 5.0, 12.0, 5.0, 12.0, 4.0, 10.0, 2.0], 4.5, 0.010, 0.55, false),
];

/// Uilleann regulators: three stopped pipes with sprung keys, each sounding one note at a time
/// (the highest open key). Semitones from the chanter's bottom note, top key first. For a D set:
/// tenor C B A G F♯, baritone A G F♯ D, bass C B A G an octave below the tenor
/// (chiffandfipple.com "Tuning the regs"; uilleannobsession.com FAQ).
pub const REGS: [(&str, &[i8]); 3] = [("Tenor", &[10, 9, 7, 5, 4]), ("Baritone", &[7, 5, 4, 0]), ("Bass", &[-2, -3, -5, -7])];

/// The rows a wrist strikes across the three regulators (key index on each, or -1):
/// C/A/C, B/G/B (G), A/F♯/A (D), G/D/G, F♯ alone.
pub const REG_ROWS: [[i8; 3]; 5] = [[0, 0, 0], [1, 1, 1], [2, 2, 2], [3, 3, 3], [4, -1, -1]];

/// Voice number for the regulators' tone in `harmonics`.
pub const REG_VOICE: usize = 100;

/// Regulator tone (estimate, not measured): the uilleann chanter's low-octave spectrum with the
/// even harmonics pulled down, as a pipe stopped at the bottom favours odd ones; a little softer.
pub const REGULATOR: Chanter = ch(false, [0.0, 7.0, 10.0, 1.0, 4.0, -5.0, -2.0, -11.0], 3.5, 0.008, 0.5, false);

/// One cycle's harmonic amplitudes (linear, fundamental = 1) for `voice` at `freq`,
/// up to the band limit. `upper` = the chanter's second octave.
pub fn harmonics(voice: usize, freq: f32, sr: f32, upper: bool, out: &mut Vec<f32>) {
    let v = if voice == REG_VOICE { &REGULATOR } else { &CHANTERS[voice.min(CHANTERS.len() - 1)] };
    let table = if upper { v.harm_hi.unwrap_or(v.harm) } else { v.harm };
    out.clear();
    let n_max = ((0.45 * sr / freq.max(20.0)) as usize).clamp(1, 160);
    for n in 1..=n_max {
        let db = if n <= 8 { table[n - 1] } else { table[7] - v.tail * (n - 8) as f32 };
        // a gentle top end: nothing a real chanter radiates much above ~9 kHz
        let f = freq * n as f32;
        let air = if f > 9000.0 { -12.0 * ((f - 9000.0) / 3000.0) } else { 0.0 };
        let a = 10f32.powf((db + air) / 20.0);
        if a < 1e-4 && n > 8 {
            break;
        }
        out.push(a);
    }
}

/// Shruti box drone patterns: semitones relative to Sa.
pub const SHRUTI: &[(&str, &[i8])] = &[
    ("Sa – Pa – Sa'", &[0, 7, 12]),
    ("Sa – Ma – Sa'", &[0, 5, 12]),
    ("Sa – Ni – Sa'", &[0, 11, 12]),
    ("Sa – Pa", &[0, 7]),
    ("Sa only", &[0, 12]),
];

pub const CHORD_TYPES: [&str; 4] = ["Triads", "Sevenths", "Power (1-5-8)", "Single notes"];

/// 5-limit just intonation, cents away from equal temperament, by semitone above the tonic.
pub const JUST_CENTS: [f32; 12] = [0.0, 11.7, 3.9, 15.6, -13.7, -2.0, -9.8, 2.0, 13.7, -15.6, 17.6, -11.7];

/// Build a chord on `key` from the scale mask (stacked thirds within the scale).
/// Falls back to a major triad when no scale is set, or the bare note if it's outside the scale.
pub fn chord_for(key: u8, mask: u16, kind: u8) -> Vec<u8> {
    if kind == 3 {
        return vec![key];
    }
    if kind == 2 {
        return [key as i32, key as i32 + 7, key as i32 + 12].iter().filter(|k| **k < 128).map(|k| *k as u8).collect();
    }
    if mask == 0 {
        // no scale chosen: plain major triad (dominant 7th for "Sevenths")
        let mut v = vec![key as i32, key as i32 + 4, key as i32 + 7];
        if kind == 1 {
            v.push(key as i32 + 10);
        }
        return v.into_iter().filter(|k| *k < 128).map(|k| k as u8).collect();
    }
    let in_scale = |k: i32| mask & (1 << (k.rem_euclid(12))) != 0;
    if !in_scale(key as i32) {
        return vec![key];
    }
    // scale tones above the root, in order
    let mut above = Vec::new();
    let mut k = key as i32;
    while above.len() < 6 && k < 127 {
        k += 1;
        if in_scale(k) {
            above.push(k);
        }
    }
    let mut out = vec![key as i32];
    if above.len() >= 4 {
        out.push(above[1]); // 3rd
        out.push(above[3]); // 5th
        if kind == 1 && above.len() >= 6 {
            out.push(above[5]); // 7th
        }
    }
    out.into_iter().filter(|k| *k < 128).map(|k| k as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chords_in_key() {
        let cmaj = crate::theory::scale_mask(0, &crate::theory::SCALES[crate::theory::scale_index("Major (Ionian)")]);
        assert_eq!(chord_for(60, cmaj, 0), vec![60, 64, 67]); // C
        assert_eq!(chord_for(62, cmaj, 0), vec![62, 65, 69]); // Dm
        assert_eq!(chord_for(71, cmaj, 0), vec![71, 74, 77]); // B dim
        assert_eq!(chord_for(67, cmaj, 1), vec![67, 71, 74, 77]); // G7
        assert_eq!(chord_for(61, cmaj, 0), vec![61]); // outside the key: single note
        // D harmonic minor V chord is A major
        let dhm = crate::theory::scale_mask(2, &crate::theory::SCALES[crate::theory::scale_index("Harmonic minor")]);
        assert_eq!(chord_for(69, dhm, 0), vec![69, 73, 76]);
        assert_eq!(chord_for(62, 0, 0), vec![62, 66, 69]); // no scale: D major
        assert_eq!(CHANTERS.len(), PIPES.len());
        for p in PIPES {
            assert!(crate::theory::SCALES.iter().any(|s| s.name == p.scale), "{}", p.name);
        }
    }
}
