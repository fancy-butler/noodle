// Noodle — music theory helpers: scales, note spelling, chord naming.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

pub const NOTE_SHARP: [&str; 12] = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
pub const NOTE_FLAT: [&str; 12] = ["C", "D♭", "D", "E♭", "E", "F", "G♭", "G", "A♭", "A", "B♭", "B"];

/// Root choices shown in the UI: (display name, pitch class, letter index 0=C..6=B, accidental)
pub const ROOTS: [(&str, u8, u8, i8); 12] = [
    ("C", 0, 0, 0),
    ("C♯", 1, 0, 1),
    ("D", 2, 1, 0),
    ("E♭", 3, 2, -1),
    ("E", 4, 2, 0),
    ("F", 5, 3, 0),
    ("F♯", 6, 3, 1),
    ("G", 7, 4, 0),
    ("A♭", 8, 5, -1),
    ("A", 9, 5, 0),
    ("B♭", 10, 6, -1),
    ("B", 11, 6, 0),
];

pub struct Scale {
    pub name: &'static str,
    pub group: &'static str,
    pub steps: &'static [u8],
    pub hint: &'static str,
}

pub const SCALES: &[Scale] = &[
    Scale { name: "Major (Ionian)", group: "Irish / Scottish", steps: &[0, 2, 4, 5, 7, 9, 11], hint: "The backbone of Irish and Scottish tunes. D and G major sit right under a D whistle and the fiddle." },
    Scale { name: "Mixolydian", group: "Irish / Scottish", steps: &[0, 2, 4, 5, 7, 9, 10], hint: "Major with a flat 7th. All over reels and pipe tunes (A Mixolydian on a D whistle = same holes as D major)." },
    Scale { name: "Dorian", group: "Irish / Scottish", steps: &[0, 2, 3, 5, 7, 9, 10], hint: "Minor with a raised 6th. E Dorian and A Dorian are classic minor-ish session keys." },
    Scale { name: "Aeolian (Natural minor)", group: "Irish / Scottish", steps: &[0, 2, 3, 5, 7, 8, 10], hint: "Plain natural minor; E minor and B minor tunes on a D whistle." },
    Scale { name: "Major pentatonic", group: "Irish / Scottish", steps: &[0, 2, 4, 7, 9], hint: "Five notes, no 4th or 7th. Very old-school Scottish/Gaelic sound." },
    Scale { name: "Freygish (Ahava Rabboh)", group: "Klezmer", steps: &[0, 1, 4, 5, 7, 8, 10], hint: "THE klezmer mode: flat 2nd + major 3rd gives that augmented-2nd cry. Also called Phrygian dominant." },
    Scale { name: "Mi Sheberach (Ukrainian Dorian)", group: "Klezmer", steps: &[0, 2, 3, 6, 7, 9, 10], hint: "Minor with a raised 4th — the doina / terkisher sound." },
    Scale { name: "Harmonic minor", group: "Klezmer", steps: &[0, 2, 3, 5, 7, 8, 11], hint: "Natural minor with a raised 7th. Freygish is this scale started from its 5th." },
    Scale { name: "Adonai Malach (≈ Mixolydian)", group: "Klezmer", steps: &[0, 2, 4, 5, 7, 9, 10], hint: "Major with a flat 7th; in practice players often flatten the 3rd up high." },
    Scale { name: "Magen Avot (≈ natural minor)", group: "Klezmer", steps: &[0, 2, 3, 5, 7, 8, 10], hint: "Basically natural minor, used for slower, darker pieces." },
    Scale { name: "Natural minor", group: "Classic", steps: &[0, 2, 3, 5, 7, 8, 10], hint: "" },
    Scale { name: "Melodic minor", group: "Classic", steps: &[0, 2, 3, 5, 7, 9, 11], hint: "Jazz melodic minor (same up and down)." },
    Scale { name: "Phrygian", group: "Classic", steps: &[0, 1, 3, 5, 7, 8, 10], hint: "" },
    Scale { name: "Lydian", group: "Classic", steps: &[0, 2, 4, 6, 7, 9, 11], hint: "" },
    Scale { name: "Locrian", group: "Classic", steps: &[0, 1, 3, 5, 6, 8, 10], hint: "" },
    Scale { name: "Minor pentatonic", group: "Classic", steps: &[0, 3, 5, 7, 10], hint: "" },
    Scale { name: "Blues", group: "Classic", steps: &[0, 3, 5, 6, 7, 10], hint: "Minor pentatonic plus the flat 5 'blue note'." },
    Scale { name: "Double harmonic", group: "Classic", steps: &[0, 1, 4, 5, 7, 8, 11], hint: "Byzantine / Arabic Hijaz-Kar flavour." },
    Scale { name: "Whole tone", group: "Classic", steps: &[0, 2, 4, 6, 8, 10], hint: "" },
    Scale { name: "Chromatic", group: "Classic", steps: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], hint: "" },
];

const LETTER_PC: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
const LETTERS: [&str; 7] = ["C", "D", "E", "F", "G", "A", "B"];

/// Pitch-class bitmask (bit 0 = C) for a scale on a root.
pub fn scale_mask(root_pc: u8, scale: &Scale) -> u16 {
    let mut m = 0u16;
    for s in scale.steps {
        m |= 1 << ((root_pc + s) % 12);
    }
    m
}

/// How a pitch class is written: letter index (0=C) and accidental (-2..2).
#[derive(Clone, Copy)]
pub struct Spell {
    pub letter: u8,
    pub acc: i8,
}

/// A spelling table for all 12 pitch classes in a given context.
#[derive(Clone, Copy)]
pub struct Speller {
    pub table: [Spell; 12],
    /// which pitch classes are "in key" (for accidental display: in-key notes still show their accidental,
    /// since we don't draw key signatures — learners see exactly what to play)
    pub in_key: u16,
}

fn sharp_spell(pc: u8) -> Spell {
    const T: [(u8, i8); 12] = [(0, 0), (0, 1), (1, 0), (1, 1), (2, 0), (3, 0), (3, 1), (4, 0), (4, 1), (5, 0), (5, 1), (6, 0)];
    Spell { letter: T[pc as usize].0, acc: T[pc as usize].1 }
}
fn flat_spell(pc: u8) -> Spell {
    const T: [(u8, i8); 12] = [(0, 0), (1, -1), (1, 0), (2, -1), (2, 0), (3, 0), (4, -1), (4, 0), (5, -1), (5, 0), (6, -1), (6, 0)];
    Spell { letter: T[pc as usize].0, acc: T[pc as usize].1 }
}

impl Speller {
    pub fn default_sharps() -> Self {
        let mut table = [Spell { letter: 0, acc: 0 }; 12];
        for pc in 0..12 {
            table[pc] = sharp_spell(pc as u8);
        }
        Speller { table, in_key: 0xFFF }
    }

    /// Proper spelling for a scale: 7-note scales get one letter per degree
    /// (D harmonic minor → D E F G A B♭ C♯). Everything else falls back to
    /// sharps or flats depending on the root.
    pub fn for_scale(root_idx: usize, scale: &Scale) -> Self {
        let (_, root_pc, root_letter, _) = ROOTS[root_idx];
        let flats = matches!(root_pc, 3 | 5 | 8 | 10) || (scale.steps.contains(&3) && matches!(root_pc, 0 | 2 | 7));
        let mut table = [Spell { letter: 0, acc: 0 }; 12];
        for pc in 0..12u8 {
            table[pc as usize] = if flats { flat_spell(pc) } else { sharp_spell(pc) };
        }
        if scale.steps.len() == 7 {
            for (i, s) in scale.steps.iter().enumerate() {
                let pc = (root_pc + s) % 12;
                let letter = (root_letter as usize + i) % 7;
                let mut acc = pc as i32 - LETTER_PC[letter];
                while acc > 6 {
                    acc -= 12;
                }
                while acc < -6 {
                    acc += 12;
                }
                if acc.abs() <= 2 {
                    table[pc as usize] = Spell { letter: letter as u8, acc: acc as i8 };
                }
            }
        }
        Speller { table, in_key: scale_mask(root_pc, scale) }
    }

    /// Diatonic staff position (C-1 = 0, middle C = 28) and accidental for a MIDI key.
    pub fn place(&self, key: u8) -> (i32, i8) {
        let sp = self.table[(key % 12) as usize];
        // octave of the *written* note (B♯ / C♭ cross octave boundaries)
        let natural = key as i32 - sp.acc as i32;
        let octave = natural.div_euclid(12) - 1; // MIDI 60 → octave 4
        (octave * 7 + sp.letter as i32, sp.acc)
    }

    pub fn name(&self, pc: u8) -> String {
        let sp = self.table[(pc % 12) as usize];
        format!("{}{}", LETTERS[sp.letter as usize], acc_str(sp.acc))
    }
}

pub fn acc_str(acc: i8) -> &'static str {
    match acc {
        -2 => "𝄫",
        -1 => "♭",
        1 => "♯",
        2 => "𝄪",
        _ => "",
    }
}

/// Staff position helper: middle C (60) → 0, each diatonic step = 1.
pub fn staff_step(speller: &Speller, key: u8) -> (i32, i8) {
    let (d, acc) = speller.place(key);
    let (dc, _) = Speller::default_sharps().place(60);
    (d - dc, acc)
}

pub fn note_name_oct(key: u8) -> String {
    format!("{}{}", NOTE_SHARP[(key % 12) as usize], key as i32 / 12 - 1)
}

const CHORDS: &[(&[u8], &str)] = &[
    (&[0, 4, 7], ""),
    (&[0, 3, 7], "m"),
    (&[0, 3, 6], "dim"),
    (&[0, 4, 8], "aug"),
    (&[0, 2, 7], "sus2"),
    (&[0, 5, 7], "sus4"),
    (&[0, 4, 7, 10], "7"),
    (&[0, 4, 10], "7"),
    (&[0, 4, 7, 11], "maj7"),
    (&[0, 4, 11], "maj7"),
    (&[0, 3, 7, 10], "m7"),
    (&[0, 3, 10], "m7"),
    (&[0, 3, 7, 11], "m(maj7)"),
    (&[0, 3, 6, 10], "m7♭5"),
    (&[0, 3, 6, 9], "dim7"),
    (&[0, 4, 7, 9], "6"),
    (&[0, 3, 7, 9], "m6"),
    (&[0, 2, 4, 7], "add9"),
    (&[0, 2, 3, 7], "m(add9)"),
    (&[0, 2, 4, 7, 10], "9"),
    (&[0, 2, 4, 10], "9"),
    (&[0, 2, 4, 7, 11], "maj9"),
    (&[0, 2, 3, 7, 10], "m9"),
    (&[0, 1, 4, 7, 10], "7♭9"),
    (&[0, 4, 5, 7, 10], "7sus4"),
    (&[0, 5, 7, 10], "7sus4"),
    (&[0, 7], "5"),
];

const INTERVALS: [&str; 12] = [
    "Unison / octave",
    "Minor 2nd",
    "Major 2nd",
    "Minor 3rd",
    "Major 3rd",
    "Perfect 4th",
    "Tritone",
    "Perfect 5th",
    "Minor 6th",
    "Major 6th",
    "Minor 7th",
    "Major 7th",
];

/// Name what's being held. Returns (big label, small detail).
pub fn name_chord(keys: &[u8], speller: &Speller) -> Option<(String, String)> {
    if keys.is_empty() {
        return None;
    }
    let mut sorted: Vec<u8> = keys.to_vec();
    sorted.sort();
    sorted.dedup();
    let bass = sorted[0] % 12;
    let mut pcs: Vec<u8> = sorted.iter().map(|k| k % 12).collect();
    pcs.sort();
    pcs.dedup();
    let names: Vec<String> = sorted.iter().map(|k| speller.name(*k)).collect();
    let detail = names.join(" ");
    if pcs.len() == 1 {
        if sorted.len() == 1 {
            return Some((note_name_oct_spelled(sorted[0], speller), String::new()));
        }
        return Some((speller.name(pcs[0]), "octaves".into()));
    }
    if sorted.len() == 2 {
        let iv = ((sorted[1] - sorted[0]) % 12) as usize;
        let lbl = if sorted[1] - sorted[0] > 12 { format!("{} (compound)", INTERVALS[iv]) } else { INTERVALS[iv].to_string() };
        return Some((lbl, detail));
    }
    // try bass as root first, then others
    let mut roots = vec![bass];
    roots.extend(pcs.iter().copied().filter(|p| *p != bass));
    for root in roots {
        let mut iv: Vec<u8> = pcs.iter().map(|p| (p + 12 - root) % 12).collect();
        iv.sort();
        for (tpl, suffix) in CHORDS {
            if iv.as_slice() == *tpl {
                let mut s = format!("{}{}", speller.name(root), suffix);
                if root != bass {
                    s.push('/');
                    s.push_str(&speller.name(bass));
                }
                return Some((s, detail));
            }
        }
    }
    Some(("?".into(), detail))
}

fn note_name_oct_spelled(key: u8, speller: &Speller) -> String {
    let sp = speller.table[(key % 12) as usize];
    let natural = key as i32 - sp.acc as i32;
    format!("{}{}{}", LETTERS[sp.letter as usize], acc_str(sp.acc), natural.div_euclid(12) - 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn spelling() {
        // D harmonic minor
        let s = Speller::for_scale(2, &SCALES[7]);
        let names: Vec<String> = SCALES[7].steps.iter().map(|x| s.name((2 + x) % 12)).collect();
        assert_eq!(names.join(" "), "D E F G A B♭ C♯");
        // E freygish
        let s = Speller::for_scale(4, &SCALES[5]);
        let names: Vec<String> = SCALES[5].steps.iter().map(|x| s.name((4 + x) % 12)).collect();
        assert_eq!(names.join(" "), "E F G♯ A B C D");
        // middle C on staff = 0, E4 = 2, B3 = -1
        let d = Speller::default_sharps();
        assert_eq!(staff_step(&d, 60).0, 0);
        assert_eq!(staff_step(&d, 64).0, 2);
        assert_eq!(staff_step(&d, 59).0, -1);
        assert_eq!(staff_step(&d, 72).0, 7);
    }
    #[test]
    fn chords() {
        let d = Speller::default_sharps();
        assert_eq!(name_chord(&[60, 64, 67], &d).unwrap().0, "C");
        assert_eq!(name_chord(&[64, 67, 72], &d).unwrap().0, "C/E");
        assert_eq!(name_chord(&[57, 60, 64, 67], &d).unwrap().0, "Am7");
        assert_eq!(name_chord(&[60, 67], &d).unwrap().0, "Perfect 5th");
    }
}
