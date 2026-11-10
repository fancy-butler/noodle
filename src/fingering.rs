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
    /// a key (C = pressed)
    Key,
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
pub fn lookup(chart: &Chart, key: u8, any_octave: bool) -> Option<(&Fing, bool)> {
    if let Some(e) = chart.fings.iter().find(|e| e.midi == key) {
        return Some((e, true));
    }
    if !any_octave {
        return None;
    }
    chart.fings.iter().filter(|e| e.midi % 12 == key % 12).min_by_key(|e| (e.midi as i32 - key as i32).abs()).map(|e| (e, false))
}

/// Which chart goes with a bagpipe (pipes::PIPES index), if any.
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
        assert_eq!(parse(lookup(ghb, 81, false).unwrap().0.states), vec![0, 0, 0, 1, 1, 1, 1, 0]);
        // any-octave fallback: A3 on a GHB maps to low A
        assert_eq!(lookup(ghb, 57, true).unwrap().0.midi, 69);
        assert!(lookup(ghb, 57, false).is_none());
    }
}
