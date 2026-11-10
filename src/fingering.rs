// Noodle — fingering charts for the visual chanter / whistle / recorder / sopilka.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Every table here is transcribed from a reference chart the user supplied
// (see `source`). Nothing is derived from theory. Entries marked `check` were
// hard to read on the source chart and are flagged in the UI until confirmed.
//
// State strings, one character per hole in `holes` order (spaces ignored):
//   C = closed / pressed, O = open, H = half-covered

#[derive(Clone, Copy, PartialEq)]
pub enum Hole {
    /// finger hole on the front
    Front,
    /// thumb hole on the back (drawn to the side)
    Thumb,
    /// double hole (recorder): H = one of the pair covered
    Double,
    /// bottom end of the pipe: C = closed (resting on the knee, or ×-stopped)
    End,
    /// a key (C = pressed); drawn as a pill beside the previous hole
    Key,
    /// small key / trill key / bis key drawn between two holes
    Small,
}

pub struct Fing {
    pub midi: u8,
    pub states: &'static str,
    pub check: bool,
}

pub struct Chart {
    pub name: &'static str,
    pub source: &'static str,
    pub holes: &'static [(Hole, &'static str)],
    /// group break before these hole indices (gap between hands)
    pub breaks: &'static [usize],
    pub fings: &'static [Fing],
    pub note: &'static str,
    /// transposing instruments: (name, semitones sounding minus written).
    /// Chart notes are written pitch; empty = sounds as written.
    pub trans: &'static [(&'static str, i8)],
}

const fn f(midi: u8, states: &'static str) -> Fing {
    Fing { midi, states, check: false }
}
const fn q(midi: u8, states: &'static str) -> Fing {
    Fing { midi, states, check: true }
}

use Hole::*;

const PIPE8: &[(Hole, &str)] = &[
    (Thumb, "LH thumb"),
    (Front, "LH 1st"),
    (Front, "LH 2nd"),
    (Front, "LH 3rd"),
    (Front, "RH 1st"),
    (Front, "RH 2nd"),
    (Front, "RH 3rd"),
    (Front, "RH 4th"),
];

pub const CHARTS: &[Chart] = &[
    Chart {
        name: "Great Highland chanter",
        source: "Northern Border Caledonia Pipe Band chart",
        holes: PIPE8,
        breaks: &[4],
        note: "Pipe music's C and F are played as C♯ and F♯.",
        trans: &[],
        fings: &[
            f(67, "CCCC CCCC"),
            f(69, "CCCC CCCO"),
            f(71, "CCCC CCOO"),
            f(73, "CCCC COOC"),
            f(74, "CCCC OOOC"),
            f(76, "CCCO CCCO"),
            f(78, "CCOO CCCO"),
            f(79, "COOO CCCO"),
            f(81, "OOOC CCCO"),
        ],
    },
    Chart {
        name: "Scottish smallpipes",
        source: "Scottish small pipe fingering chart",
        holes: PIPE8,
        breaks: &[4],
        note: "",
        trans: &[],
        fings: &[
            f(67, "CCCC CCCC"),
            f(69, "CCCC CCCO"),
            f(71, "CCCC CCOO"),
            f(73, "CCCC COOC"),
            f(74, "CCCC OOOC"),
            f(76, "CCCO CCCO"),
            f(78, "CCOO CCCO"),
            f(79, "COOO CCCO"),
            f(81, "OOOC CCCO"),
        ],
    },
    Chart {
        name: "Leicestershire smallpipes (covered)",
        source: "Basic gamut for Leicestershire smallpipes in D — covered fingering",
        holes: &[(Thumb, "T"), (Front, "1"), (Front, "2"), (Front, "3"), (Front, "4"), (Front, "5"), (Front, "6"), (Front, "7")],
        breaks: &[4, 7],
        note: "Covered (closed) fingering: one finger lifts per note.",
        trans: &[],
        fings: &[
            f(61, "CCCC CCC C"),
            f(62, "CCCC CCC O"),
            f(64, "CCCC CCO O"),
            f(66, "CCCC COC O"),
            f(67, "CCCC OCC O"),
            f(69, "CCCO CCC O"),
            f(71, "CCOC CCC O"),
            f(73, "COCC CCC O"),
            f(74, "OCCC CCC O"),
        ],
    },
    Chart {
        name: "Leicestershire smallpipes (open)",
        source: "Basic gamut for Leicestershire smallpipes in D — open fingering",
        holes: &[(Thumb, "T"), (Front, "1"), (Front, "2"), (Front, "3"), (Front, "4"), (Front, "5"), (Front, "6"), (Front, "7")],
        breaks: &[4, 7],
        note: "",
        trans: &[],
        fings: &[
            f(61, "CCCC CCC C"),
            f(62, "CCCC CCC O"),
            f(64, "CCCC CCO O"),
            f(66, "CCCC COO O"),
            f(67, "CCCC OOO O"),
            f(69, "CCCO OOO O"),
            f(71, "CCOO OOO O"),
            f(73, "COOO OOC O"),
            f(74, "OOOO OOC O"),
        ],
    },
    Chart {
        name: "Uilleann chanter",
        source: "Bigley Music Workshop uilleann pipes fingering chart",
        holes: &[
            (Thumb, "Thumb"),
            (Front, "LH 1st"),
            (Front, "LH 2nd"),
            (Front, "LH 3rd"),
            (Front, "RH 1st"),
            (Front, "RH 2nd"),
            (Front, "RH 3rd"),
            (Front, "RH 4th"),
            (End, "On the knee"),
            (Key, "C key"),
        ],
        breaks: &[4],
        note: "Second octave: stop the chanter on the knee and squeeze the bag harder.",
        trans: &[],
        fings: &[
            f(62, "CCCC CCCC O O"),
            f(63, "CCCC CCCO C O"),
            f(64, "CCCC CCOO C O"),
            f(66, "CCCC COCC C O"),
            f(67, "CCCC OOCC C O"),
            f(69, "CCCO CCCC C O"),
            f(71, "CCOO CCCC C O"),
            f(72, "COCC COCC C O"),
            f(73, "COCC CCCC C O"),
            f(74, "OCCC CCCC C O"),
            f(75, "CCCC CCCO C O"),
            f(76, "CCCC CCOO C O"),
            f(78, "CCCC COCO C O"),
            f(79, "CCCC OOCC C O"),
            f(81, "CCCO CCCC C O"),
            f(83, "CCOO CCCC C O"),
            q(84, "CCOO CCCC C C"),
            q(85, "CHOO CCCC C O"),
            f(86, "COCC COCC C O"),
        ],
    },
    Chart {
        name: "Recorder (soprano, German)",
        source: "Recorder fingering chart (after Yamaha)",
        holes: &[(Thumb, "Thumb"), (Front, "1"), (Front, "2"), (Front, "3"), (Front, "4"), (Front, "5"), (Double, "6"), (Double, "7")],
        breaks: &[4],
        note: "German fingering. The thumb is half-holed (pinched) for the top octave.",
        trans: &[],
        fings: &[
            f(72, "C CCCCC C C"),
            q(73, "C CCCCC C H"),
            f(74, "C CCCCC C O"),
            q(75, "C CCCCC H O"),
            f(76, "C CCCCC O O"),
            f(77, "C CCCCO O O"),
            f(78, "C CCCOC C C"),
            f(79, "C CCCOO O O"),
            f(80, "C CCOCC C O"),
            f(81, "C CCOOO O O"),
            f(82, "C COCCO O O"),
            f(83, "C COOOO O O"),
            f(84, "C OCOOO O O"),
            q(85, "C OOOOO O O"),
            f(86, "O OCOOO O O"),
            q(87, "O OCCCC C O"),
            f(88, "H CCCCC O O"),
            f(89, "H CCCCO O O"),
            q(90, "H CCCOC O C"),
            f(91, "H CCCOO O O"),
            f(92, "H CCCOC C C"),
            f(93, "H CCOOO O O"),
            f(94, "H CCOCC C O"),
            f(95, "H CCOCC O O"),
        ],
    },
    Chart {
        name: "Sopilka (prima)",
        source: "Аплікатура сопілки прима (Kyiv chart)",
        holes: &[
            (Thumb, "Upper thumb"),
            (Front, "1"),
            (Front, "2"),
            (Front, "3"),
            (Front, "4"),
            (Thumb, "Lower thumb"),
            (Front, "5"),
            (Front, "6"),
            (Front, "7"),
            (Front, "8"),
            (End, "Bottom end (×)"),
        ],
        breaks: &[5],
        note: "10-hole chromatic sopilka. × = close the bottom end.",
        trans: &[],
        fings: &[
            f(60, "C CCCC C CCCC O"),
            f(61, "C CCCC C CCCO O"),
            f(62, "C CCCC C CCOO O"),
            f(63, "C CCCC O CCOO O"),
            f(64, "C CCCC C COOO O"),
            f(65, "C CCCC C OOOO O"),
            f(66, "C CCCO C OOOO O"),
            f(67, "C CCOO C OOOO O"),
            f(68, "C COCC C CCCC O"),
            f(69, "C COOO C OOOO O"),
            f(70, "O OOOO C CCCC O"),
            f(71, "C OOOO C CCCC O"),
            f(72, "C CCCC C CCCC O"),
            f(73, "C CCCC C CCCO O"),
            f(74, "C CCCC C CCOO O"),
            f(75, "C CCCC O CCOO O"),
            f(76, "C CCCC C COOO O"),
            f(77, "C CCCC C OOOO O"),
            f(78, "C CCCO C OOOO O"),
            f(79, "C CCOO C OOOO O"),
            f(80, "C COCC C OOOO C"),
            f(81, "C COOO C OOOO O"),
            f(82, "O OOOO C CCCC O"),
            f(83, "C OOOO C CCCC O"),
            f(84, "C CCCC C CCCC O"),
            f(85, "C CCCC C CCCO O"),
            q(86, "C CCOO C OCCO C"),
            q(87, "C COCO C OCOO C"),
            f(88, "C CCCC C CCCC C"),
            f(89, "C CCCC C OOOO O"),
            f(90, "C CCCO C OOOO O"),
            f(91, "C CCCC C CCCC C"),
        ],
    },
    Chart {
        name: "Tin whistle",
        source: "The Whistle Shop whistle fingering chart",
        holes: &[(Front, "LH 1st"), (Front, "LH 2nd"), (Front, "LH 3rd"), (Front, "RH 1st"), (Front, "RH 2nd"), (Front, "RH 3rd")],
        breaks: &[3],
        note: "Upper octave: same fingering, blow harder. High C natural: OXO XXX.",
        trans: &[("D whistle", 0), ("E♭ whistle", 1), ("E whistle", 2), ("F whistle", 3), ("F♯ whistle", 4), ("G whistle", 5), ("A whistle", -5), ("B♭ whistle", -4), ("B whistle", -3), ("C whistle", -2), ("Low D whistle", -12)],
        fings: &[
            f(74, "CCC CCC"),
            f(76, "CCC CCO"),
            f(78, "CCC COO"),
            f(79, "CCC OOO"),
            f(81, "CCO OOO"),
            f(83, "COO OOO"),
            f(84, "OCC OOO"),
            f(85, "OOO OOO"),
            f(86, "OCC CCC"),
            f(88, "CCC CCO"),
            f(90, "CCC COO"),
            f(91, "CCC OOO"),
            f(93, "CCO OOO"),
            f(95, "COO OOO"),
            f(96, "OCO CCC"),
            f(97, "OOO OOO"),
        ],
    },
    Chart {
        name: "Flute (Boehm, B foot)",
        source: "Flute fingering chart (Dummies-style key diagram)",
        holes: &[
            (Thumb, "B thumb"),
            (Key, "B♭ thumb lever"),
            (Front, "LH 1st"),
            (Front, "LH 2nd"),
            (Front, "LH 3rd"),
            (Key, "G♯ key"),
            (Front, "RH 1st"),
            (Key, "B♭ side lever"),
            (Small, "D trill"),
            (Front, "RH 2nd"),
            (Small, "D♯ trill"),
            (Front, "RH 3rd"),
            (Key, "E♭ key"),
            (Key, "C♯ roller"),
            (Key, "C roller"),
            (Key, "B roller"),
            (Key, "gizmo"),
        ],
        breaks: &[6],
        note: "A♯/B♭ shown as thumb + 1 + RH 1. The chart's alternatives: B♭ thumb lever + 1, or 1 + side lever.",
        trans: &[],
        fings: &[
            f(59, "CO CCC O COOCOC OCCCO"),
            f(60, "CO CCC O COOCOC OCCOO"),
            f(61, "CO CCC O COOCOC OCOOO"),
            f(62, "CO CCC O COOCOC OOOOO"),
            f(63, "CO CCC O COOCOC COOOO"),
            f(64, "CO CCC O COOCOO COOOO"),
            f(65, "CO CCC O COOOOO COOOO"),
            f(66, "CO CCC O OOOOOC COOOO"),
            f(67, "CO CCC O OOOOOO COOOO"),
            f(68, "CO CCC C OOOOOO COOOO"),
            f(69, "CO CCO O OOOOOO COOOO"),
            f(70, "CO COO O COOOOO COOOO"),
            f(71, "CO COO O OOOOOO COOOO"),
            f(72, "OO COO O OOOOOO COOOO"),
            f(73, "OO OOO O OOOOOO COOOO"),
            f(74, "CO OCC O COOCOC OOOOO"),
            f(75, "CO OCC O COOCOC COOOO"),
            f(76, "CO CCC O COOCOO COOOO"),
            f(77, "CO CCC O COOOOO COOOO"),
            f(78, "CO CCC O OOOOOC COOOO"),
            f(79, "CO CCC O OOOOOO COOOO"),
            f(80, "CO CCC C OOOOOO COOOO"),
            f(81, "CO CCO O OOOOOO COOOO"),
            f(82, "CO COO O COOOOO COOOO"),
            f(83, "CO COO O OOOOOO COOOO"),
            f(84, "OO COO O OOOOOO COOOO"),
            f(85, "OO OOO O OOOOOO COOOO"),
            f(86, "CO OCC O OOOOOO COOOO"),
            f(87, "CO CCC C COOCOC COOOO"),
            f(88, "CO CCO O COOCOO COOOO"),
            f(89, "CO COC O COOOOO COOOO"),
            f(90, "CO COC O OOOOOC COOOO"),
            f(91, "OO CCC O OOOOOO COOOO"),
            f(92, "OO OCC C OOOOOO COOOO"),
            f(93, "CO OCO O COOOOO COOOO"),
            f(94, "CO OOO O COCOOO COOOO"),
            f(95, "CO COC O OOOOCO COOOO"),
            f(96, "OO CCC C COOOOO OOOOC"),
            q(97, "OO OCO O COOOOO OCCOO"),
            q(98, "CO OOC O COOCOO OCCOO"),
            f(99, "CO CCC C COCOOC COOOO"),
            q(100, "OO CCO O OOOOCC CCOOO"),
        ],
    },
    Chart {
        name: "Saxophone",
        source: "Saxophone fingering chart (checked against McGill Music Sax School chart)",
        holes: &[
            (Thumb, "octave key"),
            (Front, "LH 1st"),
            (Key, "palm D"),
            (Key, "palm E♭"),
            (Key, "palm F"),
            (Small, "bis"),
            (Front, "LH 2nd"),
            (Front, "LH 3rd"),
            (Key, "G♯"),
            (Key, "low C♯"),
            (Key, "low B"),
            (Key, "low B♭"),
            (Front, "RH 1st"),
            (Key, "side B♭"),
            (Key, "side E"),
            (Front, "RH 2nd"),
            (Front, "RH 3rd"),
            (Key, "E♭"),
            (Key, "low C"),
        ],
        breaks: &[12],
        note: "Same fingering on every sax; pick yours so the keyboard's concert pitch maps to your written note.",
        trans: &[("Alto (E♭)", -9), ("Tenor (B♭)", -14), ("Soprano (B♭)", -2), ("Baritone (E♭)", -21), ("Written pitch (EWI / Aerophone sax)", 0)],
        fings: &[
            f(58, "O C OOO O CC OOOC C OO CC OC"),
            f(59, "O C OOO O CC OOCO C OO CC OC"),
            f(60, "O C OOO O CC OOOO C OO CC OC"),
            f(61, "O C OOO O CC OCOO C OO CC OC"),
            f(62, "O C OOO O CC OOOO C OO CC OO"),
            f(63, "O C OOO O CC OOOO C OO CC CO"),
            f(64, "O C OOO O CC OOOO C OO CO OO"),
            f(65, "O C OOO O CC OOOO C OO OO OO"),
            f(66, "O C OOO O CC OOOO O OO CO OO"),
            f(67, "O C OOO O CC OOOO O OO OO OO"),
            f(68, "O C OOO O CC COOO O OO OO OO"),
            f(69, "O C OOO O CO OOOO O OO OO OO"),
            f(70, "O C OOO O CO OOOO O CO OO OO"),
            f(71, "O C OOO O OO OOOO O OO OO OO"),
            f(72, "O O OOO O CO OOOO O OO OO OO"),
            f(73, "O O OOO O OO OOOO O OO OO OO"),
            f(74, "C C OOO O CC OOOO C OO CC OO"),
            f(75, "C C OOO O CC OOOO C OO CC CO"),
            f(76, "C C OOO O CC OOOO C OO CO OO"),
            f(77, "C C OOO O CC OOOO C OO OO OO"),
            f(78, "C C OOO O CC OOOO O OO CO OO"),
            f(79, "C C OOO O CC OOOO O OO OO OO"),
            f(80, "C C OOO O CC COOO O OO OO OO"),
            f(81, "C C OOO O CO OOOO O OO OO OO"),
            f(82, "C C OOO O CO OOOO O CO OO OO"),
            f(83, "C C OOO O OO OOOO O OO OO OO"),
            f(84, "C O OOO O CO OOOO O OO OO OO"),
            f(85, "C O OOO O OO OOOO O OO OO OO"),
            f(86, "C O COO O OO OOOO O OO OO OO"),
            f(87, "C O CCO O OO OOOO O OO OO OO"),
            f(88, "C O CCO O OO OOOO O OC OO OO"),
            f(89, "C O CCC O OO OOOO O OC OO OO"),
        ],
    },
];

/// Parsed states (0 open, 1 closed, 2 half) for a chart entry.
pub fn parse(states: &str) -> Vec<u8> {
    states
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| match c {
            'C' => 1,
            'H' => 2,
            _ => 0,
        })
        .collect()
}

/// Find the fingering for `key`. With `any_octave`, falls back to the same
/// pitch class at the nearest octave on the chart. Returns (entry, exact?).
pub fn lookup(chart: &Chart, key: u8, any_octave: bool, tr: i8) -> Option<(&Fing, bool)> {
    let w = key as i32 - tr as i32;
    if !(0..128).contains(&w) {
        return None;
    }
    let key = w as u8;
    if let Some(e) = chart.fings.iter().find(|e| e.midi == key) {
        return Some((e, true));
    }
    if !any_octave {
        return None;
    }
    chart.fings.iter().filter(|e| e.midi % 12 == key % 12).min_by_key(|e| (e.midi as i32 - key as i32).abs()).map(|e| (e, false))
}

/// Which chart goes with a bagpipe (pipes::PIPES index), if any.
/// Which chart (and transposition) fits a sound, from its name. None = no fingering for it.
pub fn chart_for_sound(name: &str) -> Option<(usize, usize)> {
    let n = name.to_lowercase();
    let find = |prefix: &str| CHARTS.iter().position(|c| c.name.starts_with(prefix));
    let trans = |chart: usize, word: &str| CHARTS[chart].trans.iter().position(|t| t.0.to_lowercase().starts_with(word)).unwrap_or(0);
    if n.contains("uilleann") {
        return find("Uilleann").map(|c| (c, 0));
    }
    if n.contains("smallpipe") {
        return find("Scottish smallpipes").map(|c| (c, 0));
    }
    if n.contains("bagpipe") || n.contains("highland") || n.contains("chanter") {
        return find("Great Highland").map(|c| (c, 0));
    }
    if n.contains("sopilka") {
        return find("Sopilka").map(|c| (c, 0));
    }
    if n.contains("whistle") && !n.contains("train") && !n.contains("ball") {
        let c = find("Tin whistle")?;
        let t = if n.contains("low") { trans(c, "low d") } else { 0 };
        return Some((c, t));
    }
    if n.contains("recorder") && !n.contains("alto") && !n.contains("bass") {
        // soprano and tenor recorders share the C fingering
        return find("Recorder").map(|c| (c, 0));
    }
    if (n.contains("flute") || n.contains("piccolo")) && !n.contains("pan") && !n.contains("shakuhachi") {
        return find("Flute").map(|c| (c, 0));
    }
    if n.contains("sax") {
        let c = find("Saxophone")?;
        let t = ["alto", "tenor", "soprano", "bari"].iter().find(|w| n.contains(*w)).map(|w| trans(c, w)).unwrap_or(0);
        return Some((c, t));
    }
    None
}

pub fn chart_for_pipe(pipe: usize) -> Option<usize> {
    match pipe {
        0 => Some(0), // Great Highland
        1 => Some(1), // Scottish smallpipes
        3 => Some(4), // Uilleann
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tables_are_consistent() {
        for c in CHARTS {
            for e in c.fings {
                assert_eq!(parse(e.states).len(), c.holes.len(), "{} midi {}", c.name, e.midi);
            }
            let mut m: Vec<u8> = c.fings.iter().map(|e| e.midi).collect();
            let n = m.len();
            m.dedup();
            assert_eq!(m.len(), n, "{}: duplicate notes", c.name);
        }
        let ghb = &CHARTS[0];
        assert_eq!(parse(lookup(ghb, 81, false, 0).unwrap().0.states), vec![0, 0, 0, 1, 1, 1, 1, 0]);
        // any-octave fallback: A3 on a GHB maps to low A
        assert_eq!(lookup(ghb, 57, true, 0).unwrap().0.midi, 69);
        assert!(lookup(ghb, 57, false, 0).is_none());
        // alto sax: concert C♯4 is written B♭4 (side-B♭ fingering, 1+2)
        let sax = CHARTS.iter().find(|c| c.name == "Saxophone").unwrap();
        assert_eq!(lookup(sax, 61, false, -9).unwrap().0.midi, 70);
        // whistle: D whistle bottom note is D5
        let wh = CHARTS.iter().find(|c| c.name == "Tin whistle").unwrap();
        assert_eq!(parse(lookup(wh, 74, false, 0).unwrap().0.states), vec![1; 6]);
        // the panel follows the sound
        let sax = CHARTS.iter().position(|c| c.name == "Saxophone").unwrap();
        assert_eq!(chart_for_sound("Tenor Sax"), Some((sax, 1)));
        assert_eq!(chart_for_sound("Flute (vibrato)").map(|x| CHARTS[x.0].name), Some("Flute (Boehm, B foot)"));
        assert_eq!(chart_for_sound("Pan Flute"), None);
        assert_eq!(chart_for_sound("Yamaha Grand Piano"), None);
        assert_eq!(chart_for_sound("Bagpipe in G (with drones)").map(|x| x.0), Some(0));
    }
}
