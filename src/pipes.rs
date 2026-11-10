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
];

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
        for p in PIPES {
            assert!(crate::theory::SCALES.iter().any(|s| s.name == p.scale), "{}", p.name);
        }
    }
}
