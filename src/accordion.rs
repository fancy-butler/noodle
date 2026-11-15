// Noodle — button accordion definitions: treble layouts, basses, registers.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Where every layout comes from (Stanley's rule: no layouts from theory alone):
// - Irish B/C: the Mel Forum (melodeon.net) B/C chart with McComiskey basses,
//   as transcribed in melplot (github.com/zmoon/melplot, MIT). One typo in that
//   transcription (C row button 6 pull "B5") is corrected to B4, per the
//   standard row pattern (Wikipedia, "Diatonic button accordion": button 4 push
//   C′ / pull B).
// - Irish C#/D: the B/C chart a whole tone up (C#/D players use B/C fingering
//   "only in a higher key").
// - English D/G: Mel Forum standard 21-key D/G, via melplot.
// - French G/C: Castagnari's own "Sol/Do standard" 21-button chart
//   (castagnari.com/doc/sol-do.pdf). Basses: the Mel Forum D/G basses a fifth
//   down (the Castagnari bass chart couldn't be read) — flagged unverified.
// - Scottish B/C/C#: the B/C chart plus a C# row (C row a semitone up) —
//   assembled, flagged unverified until checked against a real chart.
// - Chromatic C-system / B-system / Finnish: rows of minor thirds, semitones on
//   the diagonal; C system C D# F# A on the outside row, then C# E G A#, then
//   D F G# B; B system mirrored with B on the outside and C on the third row;
//   Finnish = C system with the C#/E row outside (rows 4–5 repeat rows 1–2).
//   Sources: accordionwise.com, AccordionLab, Wikibooks, accordionists.info.
// - Stradella bass: columns in fifths; counterbass (major third), bass, major,
//   minor, seventh (no fifth), diminished.

use crate::reed::{Rank, Register, Tuning};
use eframe::egui::Key;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Button {
    pub push: u8,
    pub pull: u8,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BassSnd {
    Note(u8),
    Major(u8),
    Minor(u8),
}

#[derive(Clone, PartialEq, Debug)]
pub enum Bass {
    /// rows of (push, pull) bass buttons
    Diatonic(Vec<Vec<(BassSnd, BassSnd)>>),
    Stradella,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Kind {
    Diatonic,
    Chromatic,
}

#[derive(Clone, Copy)]
pub struct Look {
    pub body: [u8; 3],
    pub body2: [u8; 3],
    pub grille: [u8; 3],
    pub button: [u8; 3],
    pub accent: [u8; 3],
    pub bellows: [u8; 3],
}

pub struct Instrument {
    pub name: &'static str,
    pub family: &'static str,
    pub blurb: &'static str,
    pub source: &'static str,
    /// layout checked against a maker's or forum chart
    pub verified: bool,
    pub kind: Kind,
    /// treble rows, outside row first
    pub rows: Vec<Vec<Button>>,
    /// rows that only repeat others (chromatic rows 4–5), drawn dimmer
    pub dup_rows: usize,
    pub bass: Bass,
    pub registers: &'static [Register],
    pub tuning: Tuning,
    pub bass_ranks: &'static [Rank],
    pub look: Look,
}

pub const NUM: [Key; 12] = [Key::Num1, Key::Num2, Key::Num3, Key::Num4, Key::Num5, Key::Num6, Key::Num7, Key::Num8, Key::Num9, Key::Num0, Key::Minus, Key::Equals];
pub const QROW: [Key; 12] = [Key::Q, Key::W, Key::E, Key::R, Key::T, Key::Y, Key::U, Key::I, Key::O, Key::P, Key::OpenBracket, Key::CloseBracket];
pub const AROW: [Key; 11] = [Key::A, Key::S, Key::D, Key::F, Key::G, Key::H, Key::J, Key::K, Key::L, Key::Semicolon, Key::Quote];
pub const ZROW: [Key; 10] = [Key::Z, Key::X, Key::C, Key::V, Key::B, Key::N, Key::M, Key::Comma, Key::Period, Key::Slash];

pub fn key_label(k: Key) -> &'static str {
    match k {
        Key::Num1 => "1",
        Key::Num2 => "2",
        Key::Num3 => "3",
        Key::Num4 => "4",
        Key::Num5 => "5",
        Key::Num6 => "6",
        Key::Num7 => "7",
        Key::Num8 => "8",
        Key::Num9 => "9",
        Key::Num0 => "0",
        Key::Minus => "-",
        Key::Equals => "=",
        Key::OpenBracket => "[",
        Key::CloseBracket => "]",
        Key::Semicolon => ";",
        Key::Quote => "'",
        Key::Comma => ",",
        Key::Period => ".",
        Key::Slash => "/",
        _ => {
            // letters
            const L: [(Key, &str); 26] = [
                (Key::A, "A"), (Key::B, "B"), (Key::C, "C"), (Key::D, "D"), (Key::E, "E"), (Key::F, "F"), (Key::G, "G"), (Key::H, "H"), (Key::I, "I"), (Key::J, "J"), (Key::K, "K"), (Key::L, "L"), (Key::M, "M"),
                (Key::N, "N"), (Key::O, "O"), (Key::P, "P"), (Key::Q, "Q"), (Key::R, "R"), (Key::S, "S"), (Key::T, "T"), (Key::U, "U"), (Key::V, "V"), (Key::W, "W"), (Key::X, "X"), (Key::Y, "Y"), (Key::Z, "Z"),
            ];
            L.iter().find(|(x, _)| *x == k).map(|(_, s)| *s).unwrap_or("")
        }
    }
}

/// Computer key for treble row `r`, button `i`.
pub fn treble_key(inst: &Instrument, r: usize, i: usize) -> Option<Key> {
    match inst.kind {
        Kind::Diatonic => match r {
            0 => AROW.get(i).copied(),
            1 => QROW.get(i + 1).copied(),
            2 => NUM.get(i + 1).copied(),
            _ => None,
        },
        Kind::Chromatic => match r {
            0 => ZROW.get(i).copied(),
            1 => AROW.get(i).copied(),
            2 => QROW.get(i).copied(),
            _ => None,
        },
    }
}

/// The key row that plays basses.
pub fn bass_keys(inst: &Instrument) -> &'static [Key] {
    match (inst.kind, &inst.bass) {
        (Kind::Diatonic, Bass::Diatonic(_)) => &ZROW,
        (Kind::Diatonic, Bass::Stradella) => &ZROW,
        (Kind::Chromatic, _) => &NUM,
    }
}

pub const NAMES: [&str; 12] = ["C", "C♯", "D", "E♭", "E", "F", "F♯", "G", "A♭", "A", "B♭", "B"];
pub const NAMES_FLAT: [&str; 12] = ["C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭", "B"];
pub const NAMES_SHARP: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];

pub fn note_name(n: u8, sharps: bool) -> String {
    let t = if sharps { NAMES_SHARP } else { NAMES };
    format!("{}{}", t[(n % 12) as usize], n as i32 / 12 - 1)
}

/// "C#4" / "Eb5" → MIDI note.
pub fn parse_note(s: &str) -> u8 {
    let b = s.as_bytes();
    let pc: i32 = match b[0].to_ascii_uppercase() {
        b'C' => 0,
        b'D' => 2,
        b'E' => 4,
        b'F' => 5,
        b'G' => 7,
        b'A' => 9,
        _ => 11,
    };
    let mut i = 1;
    let mut acc = 0;
    while i < b.len() && (b[i] == b'#' || b[i] == b'b') {
        acc += if b[i] == b'#' { 1 } else { -1 };
        i += 1;
    }
    let oct: i32 = s[i..].parse().unwrap_or(4);
    ((oct + 1) * 12 + pc + acc).clamp(0, 127) as u8
}

/// "E3|A3 G3|B3 …" → buttons (push|pull), shifted by `tr` semitones.
fn row(s: &str, tr: i32) -> Vec<Button> {
    s.split_whitespace()
        .map(|b| {
            let (p, q) = b.split_once('|').unwrap_or((b, b));
            let sh = |x: u8| (x as i32 + tr).clamp(0, 127) as u8;
            Button { push: sh(parse_note(p)), pull: sh(parse_note(q)) }
        })
        .collect()
}

/// "G+|D+ G|D E-|…" → bass buttons (+ major chord, - minor, bare = bass note).
fn bass_row(s: &str, tr: i32) -> Vec<(BassSnd, BassSnd)> {
    let one = |t: &str| -> BassSnd {
        let pc = ((parse_note(&format!("{}4", t.trim_end_matches(['+', '-']))) as i32 + tr).rem_euclid(12)) as u8;
        if t.ends_with('+') {
            BassSnd::Major(pc)
        } else if t.ends_with('-') {
            BassSnd::Minor(pc)
        } else {
            BassSnd::Note(pc)
        }
    };
    s.split_whitespace()
        .map(|b| {
            let (p, q) = b.split_once('|').unwrap_or((b, b));
            (one(p), one(q))
        })
        .collect()
}

/// Chromatic rows: semitones on the diagonal, minor thirds along a row.
fn chromatic(base: u8, dir: i32, len: usize, rows: usize) -> Vec<Vec<Button>> {
    (0..rows)
        .map(|r| {
            (0..len)
                .map(|i| {
                    let n = (base as i32 + 3 * i as i32 + dir * r as i32).clamp(0, 127) as u8;
                    Button { push: n, pull: n }
                })
                .collect()
        })
        .collect()
}

/// Notes a bass button sounds: bass notes in E2..D♯3, chords in E3..D♯4.
pub fn bass_notes(s: BassSnd) -> (Vec<u8>, bool) {
    let low = |pc: u8| 40 + ((pc as i32 - 4).rem_euclid(12)) as u8;
    let mid = |pc: u8| 52 + ((pc as i32 - 4).rem_euclid(12)) as u8;
    match s {
        BassSnd::Note(pc) => (vec![low(pc)], false),
        BassSnd::Major(pc) => (vec![mid(pc), mid((pc + 4) % 12), mid((pc + 7) % 12)], true),
        BassSnd::Minor(pc) => (vec![mid(pc), mid((pc + 3) % 12), mid((pc + 7) % 12)], true),
    }
}

/// Stradella rows (from the bellows out): counterbass, bass, major, minor, 7th, dim.
pub const STRADELLA_ROWS: [&str; 6] = ["Counterbass", "Bass", "Major", "Minor", "7th", "Dim"];

pub fn stradella_notes(root_pc: u8, row: usize) -> (Vec<u8>, bool) {
    let low = |pc: u8| 40 + ((pc as i32 - 4).rem_euclid(12)) as u8;
    let mid = |pc: u8| 52 + ((pc as i32 - 4).rem_euclid(12)) as u8;
    let r = root_pc % 12;
    let ch = |iv: &[u8]| -> Vec<u8> { iv.iter().map(|i| mid((r + i) % 12)).collect() };
    match row {
        0 => (vec![low((r + 4) % 12)], false),
        1 => (vec![low(r)], false),
        2 => (ch(&[0, 4, 7]), true),
        3 => (ch(&[0, 3, 7]), true),
        4 => (ch(&[0, 4, 10]), true),
        _ => (ch(&[0, 3, 9]), true),
    }
}

pub fn stradella_label(root_pc: u8, row: usize) -> String {
    let n = NAMES[(root_pc % 12) as usize];
    match row {
        0 => NAMES[((root_pc + 4) % 12) as usize].to_lowercase(),
        1 => n.to_string(),
        2 => format!("{n}M"),
        3 => format!("{n}m"),
        4 => format!("{n}7"),
        _ => format!("{n}°"),
    }
}

/// Column of fifths around `center` (pitch class), `n` columns.
pub fn fifths(center: u8, n: usize) -> Vec<u8> {
    let half = n as i32 / 2;
    (0..n as i32).map(|k| ((center as i32 + 7 * (k - half)).rem_euclid(12)) as u8).collect()
}

// ---------------------------------------------------------------- registers

use Rank::*;
pub const REG_DIATONIC: [Register; 6] = [
    Register { name: "Swing · M M+", ranks: &[M, MSharp] },
    Register { name: "Dry · M", ranks: &[M] },
    Register { name: "Wet · M− M M+", ranks: &[MFlat, M, MSharp] },
    Register { name: "Bassoon · L M", ranks: &[L, M] },
    Register { name: "Bright · M H", ranks: &[M, H] },
    Register { name: "Full · L M M+ H", ranks: &[L, M, MSharp, H] },
];
pub const REG_SCOTTISH: [Register; 6] = [
    Register { name: "Scottish musette · M− M M+", ranks: &[MFlat, M, MSharp] },
    Register { name: "Clarinet · M", ranks: &[M] },
    Register { name: "Violin · M M+", ranks: &[M, MSharp] },
    Register { name: "Bandoneon · L M", ranks: &[L, M] },
    Register { name: "Organ · L H", ranks: &[L, H] },
    Register { name: "Master · L M− M M+ H", ranks: &[L, MFlat, M, MSharp, H] },
];
pub const REG_CHROMATIC: [Register; 10] = [
    Register { name: "Musette · M− M M+", ranks: &[MFlat, M, MSharp] },
    Register { name: "Clarinet · M", ranks: &[M] },
    Register { name: "Bassoon · L", ranks: &[L] },
    Register { name: "Piccolo · H", ranks: &[H] },
    Register { name: "Bandoneon · L M", ranks: &[L, M] },
    Register { name: "Violin · M M+", ranks: &[M, MSharp] },
    Register { name: "Oboe · M H", ranks: &[M, H] },
    Register { name: "Organ · L H", ranks: &[L, H] },
    Register { name: "Harmonium · L M H", ranks: &[L, M, H] },
    Register { name: "Master · L M− M M+ H", ranks: &[L, MFlat, M, MSharp, H] },
];
pub const REG_BAYAN: [Register; 7] = [
    Register { name: "Bayan · L M", ranks: &[L, M] },
    Register { name: "Clarinet · M", ranks: &[M] },
    Register { name: "Bassoon · L", ranks: &[L] },
    Register { name: "Oboe · M H", ranks: &[M, H] },
    Register { name: "Organ · L H", ranks: &[L, H] },
    Register { name: "Tutti · L M H", ranks: &[L, M, H] },
    Register { name: "Piccolo · H", ranks: &[H] },
];

const BASS_LM: &[Rank] = &[L, M];

pub fn instruments() -> Vec<Instrument> {
    let bc_c = "E3|A3 G3|B3 C4|D4 E4|F4 G4|A4 C5|B4 E5|D5 G5|F5 C6|A5 E6|B5";
    let bc_b = "D#3|G#3 F#3|A#3 B3|C#4 D#4|E4 F#4|G#4 B4|A#4 D#5|C#5 F#5|E5 B5|G#5 D#6|A#5 F#6|C#6";
    let bc_bass = ["G+|D+ G|D C+|G+ C|G", "E+|A+ E|A D+|F+ D|F"];
    let dg_g = "F5|Eb5 D4|F#4 G4|A4 B4|C5 D5|E5 G5|F#5 B5|A5 D6|C6 G6|E6 B6|F#6";
    let dg_d = "G#4|Bb4 A3|C#4 D4|E4 F#4|G4 A4|B4 D5|C#5 F#5|E5 A5|G5 D6|B5 F#6|C#6 A6|E6";
    let dg_bass = ["D+|A+ D|A G+|D+ G|D", "B+|E- B|E C+|C+ C|C"];
    let irish = Tuning { beat_a4: 1.6, bright: 7.0, cassotto: false };
    vec![
        Instrument {
            name: "Irish B/C · 2-row",
            family: "Irish",
            blurb: "The Irish box: two rows a semitone apart (B outside, C inside), 8 basses. Fluid, fiddle-like phrasing; nearly dry tuning.",
            source: "Mel Forum B/C chart with McComiskey basses (via melplot, MIT)",
            verified: true,
            kind: Kind::Diatonic,
            rows: vec![row(bc_b, 0), row(bc_c, 0)],
            dup_rows: 0,
            bass: Bass::Diatonic(bc_bass.iter().map(|r| bass_row(r, 0)).collect()),
            registers: &REG_DIATONIC,
            tuning: irish,
            bass_ranks: BASS_LM,
            look: Look { body: [0x9b, 0x12, 0x1c], body2: [0xd8, 0x3a, 0x3e], grille: [0xd9, 0xdd, 0xe2], button: [0xf4, 0xee, 0xe0], accent: [0x1c, 0x1c, 0x1c], bellows: [0x22, 0x22, 0x26] },
        },
        Instrument {
            name: "Irish C#/D · 2-row",
            family: "Irish",
            blurb: "Press-and-draw: the B/C system a whole tone up. Fewer buttons for D tunes, more bellows changes, more lift.",
            source: "Mel Forum B/C chart, a whole tone up",
            verified: true,
            kind: Kind::Diatonic,
            rows: vec![row(bc_b, 2), row(bc_c, 2)],
            dup_rows: 0,
            bass: Bass::Diatonic(bc_bass.iter().map(|r| bass_row(r, 2)).collect()),
            registers: &REG_DIATONIC,
            tuning: irish,
            bass_ranks: BASS_LM,
            look: Look { body: [0x14, 0x14, 0x18], body2: [0x3a, 0x3a, 0x44], grille: [0xe8, 0xe8, 0xee], button: [0xf6, 0xf2, 0xe6], accent: [0xc8, 0x9b, 0x3c], bellows: [0x1a, 0x1a, 0x1e] },
        },
        Instrument {
            name: "Scottish B/C/C# · 3-row",
            family: "Scottish",
            blurb: "The British chromatic: a B/C box with a third C♯ row and Stradella basses, in wet Scottish musette tuning.",
            source: "B/C chart + C♯ row (C row a semitone up) — assembled, not yet checked against a maker's chart",
            verified: false,
            kind: Kind::Diatonic,
            rows: vec![row(bc_b, 0), row(bc_c, 0), row(bc_c, 1)],
            dup_rows: 0,
            bass: Bass::Stradella,
            registers: &REG_SCOTTISH,
            tuning: Tuning { beat_a4: 5.5, bright: 7.5, cassotto: false },
            bass_ranks: BASS_LM,
            look: Look { body: [0xb0, 0x10, 0x16], body2: [0xe8, 0x30, 0x2a], grille: [0xe4, 0xe6, 0xea], button: [0xf2, 0xf2, 0xf2], accent: [0x10, 0x10, 0x10], bellows: [0x7a, 0x0c, 0x12] },
        },
        Instrument {
            name: "French G/C · 2-row 21",
            family: "French",
            blurb: "The diato of the bal folk: G outside, C inside, accidentals on the first buttons, 8 basses, light tremolo, wood case.",
            source: "Treble: Castagnari \"Sol/Do standard\" chart. Basses: Mel Forum D/G basses a fifth down — not yet checked against a G/C chart",
            verified: false,
            kind: Kind::Diatonic,
            rows: vec![
                row("Eb4|C#4 D3|F#3 G3|A3 B3|C4 D4|E4 G4|F#4 B4|A4 D5|C5 G5|E5 B5|F#5 D6|A5", 0),
                row("G#4|Bb4 G3|B3 C4|D4 E4|F4 G4|A4 C5|B4 E5|D5 G5|F5 C6|A5 E6|B5", 0),
            ],
            dup_rows: 0,
            bass: Bass::Diatonic(dg_bass.iter().map(|r| bass_row(r, -7)).collect()),
            registers: &REG_DIATONIC,
            tuning: Tuning { beat_a4: 2.8, bright: 6.0, cassotto: false },
            bass_ranks: BASS_LM,
            look: Look { body: [0x5a, 0x34, 0x1c], body2: [0x8e, 0x5a, 0x32], grille: [0x2e, 0x1c, 0x10], button: [0xee, 0xe2, 0xc8], accent: [0xc9, 0xa2, 0x5a], bellows: [0x1e, 0x2a, 0x4a] },
        },
        Instrument {
            name: "English D/G · 2-row 21",
            family: "English",
            blurb: "The morris and session melodeon: D outside, G inside, accidentals on the first buttons, 8 basses. Bouncy push-pull.",
            source: "Mel Forum standard 21-key D/G (via melplot, MIT)",
            verified: true,
            kind: Kind::Diatonic,
            rows: vec![row(dg_d, 0), row(dg_g, 0)],
            dup_rows: 0,
            bass: Bass::Diatonic(dg_bass.iter().map(|r| bass_row(r, 0)).collect()),
            registers: &REG_DIATONIC,
            tuning: Tuning { beat_a4: 2.2, bright: 7.0, cassotto: false },
            bass_ranks: BASS_LM,
            look: Look { body: [0x6e, 0x22, 0x1a], body2: [0xa8, 0x44, 0x2c], grille: [0x2a, 0x12, 0x0e], button: [0xf2, 0xea, 0xd6], accent: [0xd4, 0xb0, 0x6a], bellows: [0x24, 0x40, 0x2a] },
        },
        Instrument {
            name: "Chromatic C-griff",
            family: "Chromatic",
            blurb: "C system (C-griff): C, E♭, F♯, A on the outside row, semitones running across the diagonal. Paris musette country.",
            source: "C system rows (accordionwise, AccordionLab, Wikibooks)",
            verified: true,
            kind: Kind::Chromatic,
            rows: chromatic(48, 1, 12, 5),
            dup_rows: 2,
            bass: Bass::Stradella,
            registers: &REG_CHROMATIC,
            tuning: Tuning { beat_a4: 7.0, bright: 7.0, cassotto: false },
            bass_ranks: BASS_LM,
            look: Look { body: [0xe8, 0xe2, 0xd2], body2: [0xff, 0xfb, 0xf0], grille: [0x30, 0x30, 0x36], button: [0xf8, 0xf6, 0xf0], accent: [0x22, 0x22, 0x22], bellows: [0x14, 0x16, 0x2e] },
        },
        Instrument {
            name: "Chromatic B-griff",
            family: "Chromatic",
            blurb: "B system (B-griff): B on the outside row, C on the third, mirrored diagonals. Vienna, the Balkans, Eastern Europe.",
            source: "B system rows (Wikibooks, AccordionLab)",
            verified: true,
            kind: Kind::Chromatic,
            rows: chromatic(47, -1, 12, 5),
            dup_rows: 2,
            bass: Bass::Stradella,
            registers: &REG_CHROMATIC,
            tuning: Tuning { beat_a4: 4.0, bright: 7.0, cassotto: false },
            bass_ranks: BASS_LM,
            look: Look { body: [0x10, 0x10, 0x14], body2: [0x30, 0x30, 0x3a], grille: [0xc0, 0xc4, 0xcc], button: [0xf0, 0xf0, 0xf0], accent: [0xb8, 0x2a, 0x2a], bellows: [0x5a, 0x10, 0x14] },
        },
        Instrument {
            name: "Bayan",
            family: "Chromatic",
            blurb: "The Russian concert button accordion: B system, dry tuning, tone chamber, deep bassoon reeds.",
            source: "B system rows (bayan treble is B system)",
            verified: true,
            kind: Kind::Chromatic,
            rows: chromatic(47, -1, 12, 5),
            dup_rows: 2,
            bass: Bass::Stradella,
            registers: &REG_BAYAN,
            tuning: Tuning { beat_a4: 0.0, bright: 6.0, cassotto: true },
            bass_ranks: &[L, M],
            look: Look { body: [0x08, 0x08, 0x0a], body2: [0x22, 0x22, 0x28], grille: [0x14, 0x14, 0x18], button: [0xe6, 0xe0, 0xd4], accent: [0xcf, 0xa8, 0x48], bellows: [0x0c, 0x0c, 0x0e] },
        },
        Instrument {
            name: "Finnish C-system",
            family: "Chromatic",
            blurb: "Finnish 5-row: the C system shifted so the C♯/E row is outside and C sits on the third row.",
            source: "Finnish C system (accordionists.info, Johann Pascher's chart)",
            verified: true,
            kind: Kind::Chromatic,
            rows: chromatic(49, 1, 12, 5),
            dup_rows: 2,
            bass: Bass::Stradella,
            registers: &REG_CHROMATIC,
            tuning: Tuning { beat_a4: 3.0, bright: 7.0, cassotto: false },
            bass_ranks: BASS_LM,
            look: Look { body: [0x12, 0x3c, 0x7a], body2: [0x3a, 0x7a, 0xc8], grille: [0xe2, 0xe6, 0xee], button: [0xf6, 0xf6, 0xfa], accent: [0xf0, 0xf0, 0xf0], bellows: [0x0e, 0x22, 0x48] },
        },
    ]
}

/// Every (row, button, direction) that sounds `note`. direction: true = push.
pub fn find(inst: &Instrument, note: u8) -> Vec<(usize, usize, bool)> {
    let mut out = Vec::new();
    for (r, row) in inst.rows.iter().enumerate() {
        for (i, b) in row.iter().enumerate() {
            if b.push == note {
                out.push((r, i, true));
            }
            if b.pull == note && (inst.kind == Kind::Diatonic) {
                out.push((r, i, false));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_notes() {
        assert_eq!(parse_note("C4"), 60);
        assert_eq!(parse_note("D#3"), 51);
        assert_eq!(parse_note("Bb4"), 70);
        assert_eq!(parse_note("A3"), 57);
    }

    #[test]
    fn bc_matches_the_chart() {
        let all = instruments();
        let bc = &all[0];
        assert_eq!(bc.rows[0].len(), 11);
        assert_eq!(bc.rows[1].len(), 10);
        // C row button 3: push C4, pull D4 (the 'home' button)
        assert_eq!(bc.rows[1][2], Button { push: 60, pull: 62 });
        // C row button 6: push C5, pull B4
        assert_eq!(bc.rows[1][5], Button { push: 72, pull: 71 });
        // C#/D is the same a whole tone up
        assert_eq!(all[1].rows[1][2], Button { push: 62, pull: 64 });
        // every row ascends overall
        for inst in &all {
            for r in &inst.rows {
                let p: Vec<u8> = r.iter().map(|b| b.push).collect();
                assert!(p[p.len() - 1] > p[1], "{}", inst.name);
            }
        }
    }

    #[test]
    fn castagnari_gc() {
        let gc = instruments().into_iter().find(|i| i.name.starts_with("French G/C")).unwrap();
        // outer SOL row: RE/FA#, SOL/LA, SI/DO …
        assert_eq!(gc.rows[0][1], Button { push: parse_note("D3"), pull: parse_note("F#3") });
        assert_eq!(gc.rows[0][2], Button { push: parse_note("G3"), pull: parse_note("A3") });
        // inner DO row: DO/RE
        assert_eq!(gc.rows[1][2], Button { push: 60, pull: 62 });
    }

    #[test]
    fn chromatic_systems() {
        let all = instruments();
        let c = all.iter().find(|i| i.name == "Chromatic C-griff").unwrap();
        let pcs = |r: usize, inst: &Instrument| -> Vec<u8> { inst.rows[r].iter().take(4).map(|b| b.push % 12).collect() };
        assert_eq!(pcs(0, c), vec![0, 3, 6, 9]); // C D# F# A outside
        assert_eq!(pcs(1, c), vec![1, 4, 7, 10]); // C# E G A#
        assert_eq!(pcs(2, c), vec![2, 5, 8, 11]); // D F G# B
        let b = all.iter().find(|i| i.name == "Chromatic B-griff").unwrap();
        assert_eq!(pcs(0, b), vec![11, 2, 5, 8]); // B D F G# outside
        assert!(pcs(2, b).contains(&0)); // C on the third row
        let f = all.iter().find(|i| i.name == "Finnish C-system").unwrap();
        assert!(pcs(0, f).contains(&4)); // E on the first row
        assert!(pcs(2, f).contains(&0)); // C on the third row
        // every semitone in range is reachable on the first three rows
        for inst in [c, b, f] {
            let mut have = [false; 128];
            for r in &inst.rows[..3] {
                for x in r {
                    have[x.push as usize] = true;
                }
            }
            let lo = inst.rows[..3].iter().flatten().map(|x| x.push).min().unwrap() + 2;
            let hi = inst.rows[..3].iter().flatten().map(|x| x.push).max().unwrap() - 2;
            assert!((lo..=hi).all(|n| have[n as usize]), "{}", inst.name);
        }
    }

    #[test]
    fn stradella() {
        assert_eq!(stradella_notes(0, 1).0, vec![48]); // C bass
        assert_eq!(stradella_notes(0, 0).0, vec![40]); // counterbass E2
        assert_eq!(stradella_notes(7, 2).0.len(), 3);
        assert_eq!(fifths(0, 5), vec![10, 5, 0, 7, 2]);
    }
}
