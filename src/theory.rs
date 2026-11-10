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
    /// other names: klezmer, maqam, raga/thaat, jazz nicknames…
    pub aka: &'static str,
}

pub const SCALES: &[Scale] = &[
    // ---- everyday
    Scale { name: "Major (Ionian)", group: "Everyday", steps: &[0, 2, 4, 5, 7, 9, 11], hint: "Do-re-mi. Most pop, folk, hymns and kids' songs.", aka: "Bilawal (thaat)" },
    Scale { name: "Natural minor (Aeolian)", group: "Everyday", steps: &[0, 2, 3, 5, 7, 8, 10], hint: "The plain, sad-sounding minor; tons of rock and ballads.", aka: "Asavari (thaat) · ≈ Magen Avot" },
    Scale { name: "Harmonic minor", group: "Everyday", steps: &[0, 2, 3, 5, 7, 8, 11], hint: "Minor with a raised 7th: that classical / Middle-Eastern pull back to the root.", aka: "≈ Nahawand (maqam)" },
    Scale { name: "Melodic minor", group: "Everyday", steps: &[0, 2, 3, 5, 7, 9, 11], hint: "Jazz minor (same up and down). Parent of lots of jazz scales.", aka: "Jazz minor" },
    Scale { name: "Major pentatonic", group: "Everyday", steps: &[0, 2, 4, 7, 9], hint: "Five notes, no clashes. Folk, country, Celtic, Chinese and West African tunes all live here.", aka: "Chinese gong mode" },
    Scale { name: "Minor pentatonic", group: "Everyday", steps: &[0, 3, 5, 7, 10], hint: "The rock / blues solo box.", aka: "" },
    Scale { name: "Blues", group: "Everyday", steps: &[0, 3, 5, 6, 7, 10], hint: "Minor pentatonic + the flat-5 blue note.", aka: "" },
    Scale { name: "Major blues", group: "Everyday", steps: &[0, 2, 3, 4, 7, 9], hint: "Major pentatonic + the minor 3rd you slide off. Country, gospel, Motown.", aka: "" },
    // ---- church modes
    Scale { name: "Dorian", group: "Modes", steps: &[0, 2, 3, 5, 7, 9, 10], hint: "Minor with a bright 6th. Funk, Santana, 'So What', lots of folk.", aka: "Kafi (thaat)" },
    Scale { name: "Phrygian", group: "Modes", steps: &[0, 1, 3, 5, 7, 8, 10], hint: "Minor with a flat 2nd — dark, Spanish, metal.", aka: "Bhairavi (thaat) · ≈ Kurd (maqam)" },
    Scale { name: "Lydian", group: "Modes", steps: &[0, 2, 4, 6, 7, 9, 11], hint: "Major with a raised 4th — dreamy, film-score floaty.", aka: "Kalyan / Yaman (thaat)" },
    Scale { name: "Mixolydian", group: "Modes", steps: &[0, 2, 4, 5, 7, 9, 10], hint: "Major with a flat 7th — rock, blues, bagpipes.", aka: "Khamaj (thaat) · ≈ Adonai Malach" },
    Scale { name: "Locrian", group: "Modes", steps: &[0, 1, 3, 5, 6, 8, 10], hint: "Diminished root chord; unstable, mostly a jazz/metal colour.", aka: "" },
    // ---- Irish / Scottish
    Scale { name: "Irish/Scottish: Major (Ionian)", group: "Irish / Scottish", steps: &[0, 2, 4, 5, 7, 9, 11], hint: "Backbone of Irish and Scottish tunes. D and G major sit right under a D whistle and the fiddle.", aka: "" },
    Scale { name: "Irish/Scottish: Mixolydian", group: "Irish / Scottish", steps: &[0, 2, 4, 5, 7, 9, 10], hint: "Major with a flat 7th. All over reels and pipe tunes (A Mixolydian on a D whistle = same holes as D major).", aka: "" },
    Scale { name: "Irish/Scottish: Dorian", group: "Irish / Scottish", steps: &[0, 2, 3, 5, 7, 9, 10], hint: "Minor with a raised 6th. E Dorian and A Dorian are classic minor-ish session keys.", aka: "" },
    Scale { name: "Irish/Scottish: Aeolian", group: "Irish / Scottish", steps: &[0, 2, 3, 5, 7, 8, 10], hint: "Plain natural minor; E minor and B minor tunes on a D whistle.", aka: "" },
    Scale { name: "Irish/Scottish: Pentatonic", group: "Irish / Scottish", steps: &[0, 2, 4, 7, 9], hint: "No 4th or 7th. Very old-school Scottish / Gaelic sound.", aka: "" },
    // ---- klezmer
    Scale { name: "Freygish (Ahava Rabboh)", group: "Klezmer", steps: &[0, 1, 4, 5, 7, 8, 10], hint: "THE klezmer mode: flat 2nd + major 3rd gives that augmented-2nd cry. Also called Phrygian dominant.", aka: "Phrygian dominant · ≈ Hijaz · Spanish Gypsy" },
    Scale { name: "Mi Sheberach (Ukrainian Dorian)", group: "Klezmer", steps: &[0, 2, 3, 6, 7, 9, 10], hint: "Minor with a raised 4th — the doina / terkisher sound. Also called Dorian ♯4 / Romanian minor.", aka: "Dorian ♯4 · Romanian minor · Altered Dorian" },
    Scale { name: "Klezmer harmonic minor", group: "Klezmer", steps: &[0, 2, 3, 5, 7, 8, 11], hint: "Natural minor with a raised 7th. Freygish is this scale started from its 5th.", aka: "Harmonic minor" },
    Scale { name: "Adonai Malach (≈ Mixolydian)", group: "Klezmer", steps: &[0, 2, 4, 5, 7, 9, 10], hint: "Major with a flat 7th; in practice players often flatten the 3rd up high.", aka: "Mixolydian" },
    Scale { name: "Magen Avot (≈ natural minor)", group: "Klezmer", steps: &[0, 2, 3, 5, 7, 8, 10], hint: "Basically natural minor, used for slower, darker pieces.", aka: "Aeolian / natural minor" },
    // ---- jazz
    Scale { name: "Bebop dominant", group: "Jazz", steps: &[0, 2, 4, 5, 7, 9, 10, 11], hint: "Mixolydian + a passing major 7th so chord tones land on the beat.", aka: "" },
    Scale { name: "Bebop major", group: "Jazz", steps: &[0, 2, 4, 5, 7, 8, 9, 11], hint: "Major + a passing ♯5/♭6.", aka: "" },
    Scale { name: "Bebop minor (Dorian)", group: "Jazz", steps: &[0, 2, 3, 4, 5, 7, 9, 10], hint: "Dorian + a passing major 3rd.", aka: "" },
    Scale { name: "Lydian dominant", group: "Jazz", steps: &[0, 2, 4, 6, 7, 9, 10], hint: "Mixolydian with a ♯4. Over 7♯11 chords; also 'the Simpsons scale'.", aka: "Overtone scale · Acoustic scale" },
    Scale { name: "Altered (Super Locrian)", group: "Jazz", steps: &[0, 1, 3, 4, 6, 8, 10], hint: "Every tension at once, over an altered dominant (7alt).", aka: "Diminished whole-tone" },
    Scale { name: "Lydian augmented", group: "Jazz", steps: &[0, 2, 4, 6, 8, 9, 11], hint: "Lydian with a ♯5. Over maj7♯5.", aka: "" },
    Scale { name: "Dorian ♭2 (Phrygian ♮6)", group: "Jazz", steps: &[0, 1, 3, 5, 7, 9, 10], hint: "2nd mode of melodic minor. Over sus♭9 chords.", aka: "" },
    Scale { name: "Mixolydian ♭6", group: "Jazz", steps: &[0, 2, 4, 5, 7, 8, 10], hint: "Major-ish bottom, minor-ish top. Also called Aeolian dominant / Hindu.", aka: "Aeolian dominant · Hindu" },
    Scale { name: "Locrian ♮2 (half-diminished)", group: "Jazz", steps: &[0, 2, 3, 5, 6, 8, 10], hint: "Over m7♭5 chords.", aka: "Aeolian ♭5" },
    Scale { name: "Diminished (half-whole)", group: "Jazz", steps: &[0, 1, 3, 4, 6, 7, 9, 10], hint: "Symmetric 8-note scale over 7♭9 chords.", aka: "Octatonic · dominant diminished" },
    Scale { name: "Diminished (whole-half)", group: "Jazz", steps: &[0, 2, 3, 5, 6, 8, 9, 11], hint: "Symmetric 8-note scale over dim7 chords.", aka: "Octatonic" },
    Scale { name: "Harmonic major", group: "Jazz", steps: &[0, 2, 4, 5, 7, 8, 11], hint: "Major with a ♭6. Bittersweet.", aka: "" },
    // ---- harmonic minor modes
    Scale { name: "Phrygian dominant", group: "Harmonic-minor modes", steps: &[0, 1, 4, 5, 7, 8, 10], hint: "Same as Freygish. Flamenco, Arabic Hijaz (approx.), metal.", aka: "Freygish · Ahava Rabboh · ≈ Hijaz · Spanish Gypsy" },
    Scale { name: "Locrian ♮6", group: "Harmonic-minor modes", steps: &[0, 1, 3, 5, 6, 9, 10], hint: "2nd mode of harmonic minor.", aka: "" },
    Scale { name: "Dorian ♯4", group: "Harmonic-minor modes", steps: &[0, 2, 3, 6, 7, 9, 10], hint: "4th mode of harmonic minor — the doina / terkisher sound.", aka: "Mi Sheberach · Ukrainian Dorian · Romanian minor" },
    Scale { name: "Ionian ♯5", group: "Harmonic-minor modes", steps: &[0, 2, 4, 5, 8, 9, 11], hint: "3rd mode of harmonic minor.", aka: "" },
    Scale { name: "Lydian ♯2", group: "Harmonic-minor modes", steps: &[0, 3, 4, 6, 7, 9, 11], hint: "6th mode of harmonic minor. Exotic, bright.", aka: "" },
    Scale { name: "Ultralocrian", group: "Harmonic-minor modes", steps: &[0, 1, 3, 4, 6, 8, 9], hint: "7th mode of harmonic minor (diminished-7th flavour).", aka: "" },
    // ---- Middle East / Balkans / Romani / Spain
    Scale { name: "Double harmonic (Byzantine)", group: "Middle East / Balkans / Spain", steps: &[0, 1, 4, 5, 7, 8, 11], hint: "Two augmented 2nds. Arabic Hijaz Kar, Indian Bhairav. 12-tone approximation — real maqamat use quarter tones.", aka: "Double harmonic major · ≈ Hijaz Kar · Bhairav · Arabic" },
    Scale { name: "Hungarian minor (Gypsy minor)", group: "Middle East / Balkans / Spain", steps: &[0, 2, 3, 6, 7, 8, 11], hint: "Harmonic minor with a ♯4.", aka: "Hungarian double minor · Double harmonic minor" },
    Scale { name: "Hungarian major", group: "Middle East / Balkans / Spain", steps: &[0, 3, 4, 6, 7, 9, 10], hint: "♯2, ♯4, ♭7 — unusual and bright.", aka: "" },
    Scale { name: "Neapolitan minor", group: "Middle East / Balkans / Spain", steps: &[0, 1, 3, 5, 7, 8, 11], hint: "Harmonic minor with a ♭2.", aka: "" },
    Scale { name: "Neapolitan major", group: "Middle East / Balkans / Spain", steps: &[0, 1, 3, 5, 7, 9, 11], hint: "Melodic minor with a ♭2.", aka: "" },
    Scale { name: "Persian", group: "Middle East / Balkans / Spain", steps: &[0, 1, 4, 5, 6, 8, 11], hint: "Very chromatic, ♭5 and ♭6.", aka: "" },
    Scale { name: "Spanish 8-tone (flamenco)", group: "Middle East / Balkans / Spain", steps: &[0, 1, 3, 4, 5, 6, 8, 10], hint: "Phrygian with both 3rds (and a ♭5) — flamenco guitar territory.", aka: "Spanish Phrygian" },
    // ---- East Asia / Indonesia
    Scale { name: "Hirajoshi", group: "East Asia", steps: &[0, 2, 3, 7, 8], hint: "Japanese koto tuning.", aka: "" },
    Scale { name: "In (Miyako-bushi)", group: "East Asia", steps: &[0, 1, 5, 7, 8], hint: "Japanese — the 'Sakura' sound.", aka: "" },
    Scale { name: "Insen", group: "East Asia", steps: &[0, 1, 5, 7, 10], hint: "Japanese, shakuhachi-ish.", aka: "" },
    Scale { name: "Iwato", group: "East Asia", steps: &[0, 1, 5, 6, 10], hint: "Japanese, very dark.", aka: "" },
    Scale { name: "Kumoi", group: "East Asia", steps: &[0, 2, 3, 7, 9], hint: "Japanese, gentle minor.", aka: "" },
    Scale { name: "Yo", group: "East Asia", steps: &[0, 2, 5, 7, 9], hint: "Japanese folk (no semitones).", aka: "" },
    Scale { name: "Ryukyu (Okinawa)", group: "East Asia", steps: &[0, 4, 5, 7, 11], hint: "Okinawan — bright and sunny.", aka: "" },
    Scale { name: "Pelog (approx.)", group: "East Asia", steps: &[0, 1, 3, 7, 8], hint: "Balinese/Javanese gamelan, squeezed into 12-tone tuning.", aka: "" },
    // ---- India (thaats, 12-tone approximations)
    Scale { name: "Bhairav", group: "India (thaat, approx.)", steps: &[0, 1, 4, 5, 7, 8, 11], hint: "Morning raga family. (Bilawal = Major, Kafi = Dorian, Khamaj = Mixolydian, Asavari = Aeolian, Bhairavi = Phrygian, Kalyan = Lydian.)", aka: "Double harmonic" },
    Scale { name: "Todi", group: "India (thaat, approx.)", steps: &[0, 1, 3, 6, 7, 8, 11], hint: "♭2, ♭3, ♯4, ♭6 — intense.", aka: "" },
    Scale { name: "Purvi", group: "India (thaat, approx.)", steps: &[0, 1, 4, 6, 7, 8, 11], hint: "Evening raga family.", aka: "" },
    Scale { name: "Marwa", group: "India (thaat, approx.)", steps: &[0, 1, 4, 6, 7, 9, 11], hint: "Sunset raga family.", aka: "" },
    // ---- symmetric & oddballs
    Scale { name: "Whole tone", group: "Symmetric & strange", steps: &[0, 2, 4, 6, 8, 10], hint: "No semitones — dream sequences, Debussy.", aka: "" },
    Scale { name: "Augmented", group: "Symmetric & strange", steps: &[0, 3, 4, 7, 8, 11], hint: "Two augmented triads interlocked.", aka: "" },
    Scale { name: "Tritone", group: "Symmetric & strange", steps: &[0, 1, 4, 6, 7, 10], hint: "Two major triads a tritone apart.", aka: "" },
    Scale { name: "Prometheus", group: "Symmetric & strange", steps: &[0, 2, 4, 6, 9, 10], hint: "Scriabin's 'mystic' sound.", aka: "" },
    Scale { name: "Enigmatic", group: "Symmetric & strange", steps: &[0, 1, 4, 6, 8, 10, 11], hint: "Verdi's puzzle scale.", aka: "" },
    Scale { name: "Messiaen mode 3", group: "Symmetric & strange", steps: &[0, 2, 3, 4, 6, 7, 8, 10, 11], hint: "Whole-half-half, repeated.", aka: "" },
    Scale { name: "Chromatic", group: "Symmetric & strange", steps: &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11], hint: "Everything.", aka: "" },
];

pub fn scale_index(name: &str) -> usize {
    SCALES.iter().position(|s| s.name == name).unwrap_or(0)
}

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
        let in_key = scale_mask(root_pc, scale);
        if scale.steps.len() == 7 {
            // out-of-scale notes: sharpen the scale note below, else flatten the one above
            let base = table;
            for pc in 0..12usize {
                if in_key & (1 << pc) != 0 {
                    continue;
                }
                let below = (pc + 11) % 12;
                let above = (pc + 1) % 12;
                if in_key & (1 << below) != 0 && base[below].acc < 2 {
                    table[pc] = Spell { letter: base[below].letter, acc: base[below].acc + 1 };
                } else if in_key & (1 << above) != 0 && base[above].acc > -2 {
                    table[pc] = Spell { letter: base[above].letter, acc: base[above].acc - 1 };
                }
            }
        }
        Speller { table, in_key }
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
        let hm = &SCALES[scale_index("Harmonic minor")];
        let s = Speller::for_scale(2, hm);
        let names: Vec<String> = hm.steps.iter().map(|x| s.name((2 + x) % 12)).collect();
        assert_eq!(names.join(" "), "D E F G A B♭ C♯");
        // E freygish
        let fr = &SCALES[scale_index("Freygish (Ahava Rabboh)")];
        let s = Speller::for_scale(4, fr);
        let names: Vec<String> = fr.steps.iter().map(|x| s.name((4 + x) % 12)).collect();
        assert_eq!(names.join(" "), "E F G♯ A B C D");
        // A Lydian ♯2: A B♯ C♯ D♯ E F♯ G♯ ; C Altered: C D♭ E♭ F♭ G♭ A♭ B♭
        let l2 = &SCALES[scale_index("Lydian ♯2")];
        let s = Speller::for_scale(9, l2);
        let names: Vec<String> = l2.steps.iter().map(|x| s.name((9 + x) % 12)).collect();
        assert_eq!(names.join(" "), "A B♯ C♯ D♯ E F♯ G♯");
        let alt = &SCALES[scale_index("Altered (Super Locrian)")];
        let s = Speller::for_scale(0, alt);
        let names: Vec<String> = alt.steps.iter().map(|x| s.name(x % 12)).collect();
        assert_eq!(names.join(" "), "C D♭ E♭ F♭ G♭ A♭ B♭");
        // out-of-scale F♯ in D Mi Sheberach (D E F G♯ A B C) must read F♯, not G♭
        let ms = &SCALES[scale_index("Mi Sheberach (Ukrainian Dorian)")];
        let s = Speller::for_scale(2, ms);
        assert_eq!(s.name(6), "F♯");
        for sc in SCALES {
            assert!(sc.steps[0] == 0 && sc.steps.windows(2).all(|w| w[0] < w[1] && w[1] < 12), "{}", sc.name);
        }
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
