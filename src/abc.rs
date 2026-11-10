// Noodle — ABC notation reader (folk tune books, The Session, etc.).
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Covers what trad tune collections actually use: header fields, keys with
// modes (and the pipers' K:HP / K:Hp), explicit key accidentals (K:D phr ^f),
// accidentals that last to the bar line, note lengths, broken rhythm (> <),
// tuplets, ties, chords, rests, repeats with 1st/2nd endings, inline fields.
// Grace notes, decorations, chord symbols and lyrics are skipped.

use crate::song::{Song, SongEv, SongNote};

/// One tune's text (header + body) found in an ABC file.
#[derive(Clone, Debug)]
pub struct TuneText {
    pub title: String,
    pub text: String,
}

/// What we learned about the tune besides its notes.
#[derive(Clone, Debug, Default)]
pub struct TuneInfo {
    pub title: String,
    pub rhythm: String,
    pub meter: String,
    /// tonic pitch class (0 = C)
    pub tonic: u8,
    /// mode word as Noodle's scale name ("Major (Ionian)", "Dorian", …)
    pub scale: &'static str,
    /// written key, e.g. "D major", "E dorian"
    pub key_label: String,
}

/// Split an ABC file into tunes (each starts at an `X:` line). A file with no
/// `X:` is one tune.
pub fn split_tunes(src: &str) -> Vec<TuneText> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for line in src.lines() {
        if line.trim_start().starts_with("X:") && !cur.trim().is_empty() && cur.contains("K:") {
            out.push(cur.clone());
            cur.clear();
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        out.push(cur);
    }
    out.into_iter()
        .filter(|t| t.lines().any(|l| l.trim_start().starts_with("K:")) || !t.contains(':'))
        .map(|t| {
            let title = t.lines().find_map(|l| l.trim_start().strip_prefix("T:").map(|s| s.trim().to_string())).unwrap_or_else(|| "Untitled".into());
            TuneText { title, text: t }
        })
        .collect()
}

/// Build ABC text from The Session's data fields (its `abc` field is the body only).
pub fn from_session(name: &str, kind: &str, meter: &str, mode: &str, body: &str) -> String {
    // Session modes look like "Dmajor", "Edorian", "Aminor", "Gmixolydian"
    let mut k = mode.to_string();
    for (long, short) in [("major", "maj"), ("minor", "min"), ("dorian", "dor"), ("mixolydian", "mix"), ("lydian", "lyd"), ("phrygian", "phr"), ("locrian", "loc"), ("aeolian", "aeo"), ("ionian", "ion")] {
        if let Some(r) = mode.strip_suffix(long) {
            k = format!("{r}{short}");
        }
    }
    // The Session writes every tune in eighths (its own header is always L:1/8)
    let l = "1/8";
    let body: Vec<&str> = body.split(['\r', '\n']).filter(|l| !l.trim().is_empty()).collect();
    format!("X:1\nT:{name}\nR:{kind}\nM:{meter}\nL:{l}\nK:{k}\n{}\n", body.join("\n"))
}

fn meter_value(m: &str) -> f64 {
    let m = m.trim();
    match m {
        "C" => 1.0,
        "C|" => 1.0,
        _ => {
            let mut it = m.split('/');
            let n: f64 = it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(4.0);
            let d: f64 = it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(4.0);
            n / d.max(1.0)
        }
    }
}

/// Default speed (quarter notes per minute) for a tune type when the ABC has no Q:.
fn default_qpm(rhythm: &str, meter: &str) -> f64 {
    let r = rhythm.to_lowercase();
    let pick = |s: &str| r.contains(s);
    if pick("slip") {
        165.0
    } else if pick("jig") {
        165.0
    } else if pick("reel") {
        180.0
    } else if pick("hornpipe") {
        150.0
    } else if pick("polka") {
        200.0
    } else if pick("slide") {
        190.0
    } else if pick("strathspey") {
        120.0
    } else if pick("waltz") || pick("valse") {
        150.0
    } else if pick("mazurka") {
        140.0
    } else if pick("bourr") {
        170.0
    } else if pick("barndance") || pick("schottis") || pick("scottish") {
        160.0
    } else if pick("march") {
        120.0
    } else if pick("freylekh") || pick("bulgar") || pick("hora") || pick("sher") {
        140.0
    } else if meter.contains("/8") {
        165.0
    } else {
        120.0
    }
}

// ---------------- key signatures ----------------

/// letter index (C=0..B=6) → pitch class
const LETTER_PC: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
/// letter index → position on the circle of fifths for the natural note
const LETTER_FIFTHS: [i32; 7] = [0, 2, 4, -1, 1, 3, 5];

fn letter_idx(c: char) -> Option<usize> {
    match c.to_ascii_uppercase() {
        'C' => Some(0),
        'D' => Some(1),
        'E' => Some(2),
        'F' => Some(3),
        'G' => Some(4),
        'A' => Some(5),
        'B' => Some(6),
        _ => None,
    }
}

/// Parse a K: value. Returns (per-letter accidentals, info about the key).
fn parse_key(k: &str) -> ([i32; 7], u8, &'static str, String) {
    let k = k.trim();
    let mut acc = [0i32; 7];
    // strip clef etc.
    let main: String = k.split_whitespace().take_while(|w| !w.contains('=') || w.starts_with('=')).collect::<Vec<_>>().join(" ");
    let lower = main.to_lowercase();
    if lower.starts_with("hp") {
        // Highland pipes. "HP": written with no signature. "Hp": F♯ C♯ (G natural).
        if main.starts_with("Hp") {
            acc[3] = 1;
            acc[0] = 1;
        }
        return (acc, 9, "Mixolydian", "A mixolydian (pipes)".into());
    }
    if lower.is_empty() || lower.starts_with("none") {
        return (acc, 0, "Major (Ionian)", "C major".into());
    }
    let mut chars = main.chars().peekable();
    let Some(lc) = chars.next() else { return (acc, 0, "Major (Ionian)", "C major".into()) };
    let Some(li) = letter_idx(lc) else { return (acc, 0, "Major (Ionian)", "C major".into()) };
    let mut root_acc = 0i32;
    match chars.peek() {
        Some('#') | Some('♯') => {
            root_acc = 1;
            chars.next();
        }
        Some('b') | Some('♭') => {
            // "Bb" vs "B" followed by nothing — only treat b as flat when it directly follows the letter
            root_acc = -1;
            chars.next();
        }
        _ => {}
    }
    let rest: String = chars.collect();
    let mut words = rest.split_whitespace();
    let first = words.next().unwrap_or("");
    // mode may be glued to the root ("Dmix") or separate ("D mix"); explicit accidentals follow
    let (mode_word, extra_words): (String, Vec<&str>) = if first.starts_with(['^', '_', '=']) {
        (String::new(), std::iter::once(first).chain(words).collect())
    } else {
        (first.to_lowercase(), words.collect())
    };
    let m3: String = mode_word.chars().take(3).collect();
    let (shift, scale, mname): (i32, &'static str, &str) = match m3.as_str() {
        "" | "maj" | "ion" => (0, "Major (Ionian)", "major"),
        "m" | "min" | "aeo" => (-3, "Natural minor (Aeolian)", "minor"),
        "dor" => (-2, "Dorian", "dorian"),
        "phr" => (-4, "Phrygian", "phrygian"),
        "lyd" => (1, "Lydian", "lydian"),
        "mix" => (-1, "Mixolydian", "mixolydian"),
        "loc" => (-5, "Locrian", "locrian"),
        "exp" => (0, "Major (Ionian)", "major"),
        _ if m3.starts_with('m') => (-3, "Natural minor (Aeolian)", "minor"),
        _ => (0, "Major (Ionian)", "major"),
    };
    let fifths = if m3 == "exp" { 0 } else { LETTER_FIFTHS[li] + 7 * root_acc + shift };
    // sharps: F C G D A E B ; flats: B E A D G C F
    const SHARP_ORDER: [usize; 7] = [3, 0, 4, 1, 5, 2, 6];
    const FLAT_ORDER: [usize; 7] = [6, 2, 5, 1, 4, 0, 3];
    if fifths > 0 {
        for i in 0..(fifths.min(14) as usize) {
            acc[SHARP_ORDER[i % 7]] += 1;
        }
    } else if fifths < 0 {
        for i in 0..((-fifths).min(14) as usize) {
            acc[FLAT_ORDER[i % 7]] -= 1;
        }
    }
    // explicit accidentals: ^f _b =c
    let mut changed_mode = false;
    for w in extra_words.iter().chain(std::iter::once(&mode_word.as_str()).filter(|w| w.starts_with(['^', '_', '=']))) {
        let mut a = 0i32;
        let mut nat = false;
        for c in w.chars() {
            match c {
                '^' => a += 1,
                '_' => a -= 1,
                '=' => nat = true,
                c => {
                    if let Some(l) = letter_idx(c) {
                        acc[l] = if nat { 0 } else { a };
                        changed_mode = true;
                    }
                    a = 0;
                    nat = false;
                }
            }
        }
    }
    let tonic = (LETTER_PC[li] + root_acc).rem_euclid(12) as u8;
    let mut scale = scale;
    // D phrygian with a raised 3rd (^f) is freygish
    if changed_mode && scale == "Phrygian" {
        let third = (li + 2) % 7;
        let pc3 = (LETTER_PC[third] + acc[third]).rem_euclid(12);
        if (pc3 - tonic as i32).rem_euclid(12) == 4 {
            scale = "Freygish (Ahava Rabboh)";
        }
    }
    let name = format!("{}{}", lc.to_ascii_uppercase(), match root_acc { 1 => "♯", -1 => "♭", _ => "" });
    let label = if scale.starts_with("Freygish") { format!("{name} freygish") } else { format!("{name} {mname}") };
    (acc, tonic, scale, label)
}

// ---------------- body parsing ----------------

#[derive(Clone, Debug)]
struct Ev {
    keys: Vec<u8>,
    /// length in whole notes
    len: f64,
    tie: bool,
    /// a bar line came right before this event
    downbeat: bool,
}

struct Parser {
    acc: [i32; 7],
    bar_acc: std::collections::HashMap<(usize, i32), i32>,
    unit: f64,
    evs: Vec<Ev>,
    // tuplets
    tup_left: u32,
    tup_mul: f64,
    // broken rhythm waiting for the next note
    pending_mul: f64,
    // repeats
    rep_start: usize,
    ending1: Option<usize>,
    in_ending2: bool,
    downbeat: bool,
    meter: f64,
}

impl Parser {
    fn bar(&mut self) {
        self.bar_acc.clear();
        self.downbeat = true;
    }

    fn push(&mut self, keys: Vec<u8>, len: f64, tie: bool) {
        let mut len = len * self.pending_mul;
        self.pending_mul = 1.0;
        if self.tup_left > 0 {
            len *= self.tup_mul;
            self.tup_left -= 1;
        }
        let downbeat = std::mem::take(&mut self.downbeat);
        self.evs.push(Ev { keys, len, tie, downbeat });
    }

    /// end of a repeated section: play it again (minus the 1st ending)
    fn repeat_end(&mut self) {
        let end = self.ending1.unwrap_or(self.evs.len());
        if end > self.rep_start {
            let copy: Vec<Ev> = self.evs[self.rep_start..end].to_vec();
            self.evs.extend(copy);
        }
        self.in_ending2 = self.ending1.is_some();
        self.ending1 = None;
        self.rep_start = self.evs.len();
    }

    fn repeat_start(&mut self) {
        self.rep_start = self.evs.len();
        self.ending1 = None;
        self.in_ending2 = false;
    }
}

fn read_len(chars: &[char], i: &mut usize) -> f64 {
    let mut num = String::new();
    while *i < chars.len() && chars[*i].is_ascii_digit() {
        num.push(chars[*i]);
        *i += 1;
    }
    let n: f64 = if num.is_empty() { 1.0 } else { num.parse().unwrap_or(1.0) };
    let mut d = 1.0;
    while *i < chars.len() && chars[*i] == '/' {
        *i += 1;
        let mut den = String::new();
        while *i < chars.len() && chars[*i].is_ascii_digit() {
            den.push(chars[*i]);
            *i += 1;
        }
        d *= if den.is_empty() { 2.0 } else { den.parse().unwrap_or(2.0) };
    }
    n / d
}

/// Read one pitch (accidentals, letter, octave marks). Returns MIDI key.
fn read_pitch(p: &mut Parser, chars: &[char], i: &mut usize) -> Option<u8> {
    let mut a: Option<i32> = None;
    loop {
        match chars.get(*i) {
            Some('^') => {
                a = Some(a.unwrap_or(0) + 1);
                *i += 1;
            }
            Some('_') => {
                a = Some(a.unwrap_or(0) - 1);
                *i += 1;
            }
            Some('=') => {
                a = Some(0);
                *i += 1;
            }
            _ => break,
        }
    }
    let c = *chars.get(*i)?;
    let li = letter_idx(c)?;
    *i += 1;
    let mut oct: i32 = if c.is_ascii_lowercase() { 5 } else { 4 };
    while let Some(&m) = chars.get(*i) {
        match m {
            '\'' => oct += 1,
            ',' => oct -= 1,
            _ => break,
        }
        *i += 1;
    }
    let accv = match a {
        Some(v) => {
            p.bar_acc.insert((li, oct), v);
            v
        }
        None => p.bar_acc.get(&(li, oct)).copied().unwrap_or(p.acc[li]),
    };
    let key = (oct + 1) * 12 + LETTER_PC[li] + accv;
    (0..128).contains(&key).then_some(key as u8)
}

fn field(p: &mut Parser, name: char, val: &str, info: &mut TuneInfo, qpm: &mut Option<f64>) {
    let val = val.trim();
    match name {
        'K' => {
            let (acc, tonic, scale, label) = parse_key(val);
            p.acc = acc;
            if info.key_label.is_empty() {
                info.tonic = tonic;
                info.scale = scale;
                info.key_label = label;
            }
        }
        'L' => {
            let mut it = val.split('/');
            let n: f64 = it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(1.0);
            let d: f64 = it.next().and_then(|s| s.trim().parse().ok()).unwrap_or(8.0);
            p.unit = n / d.max(1.0);
        }
        'M' => {
            p.meter = meter_value(val);
            if info.meter.is_empty() {
                info.meter = val.to_string();
            }
        }
        'Q' => {
            // "1/4=120", "3/8=110", "120", "\"Allegro\" 1/4=120"
            let v = val.rsplit(' ').next().unwrap_or(val);
            if let Some((beat, bpm)) = v.split_once('=') {
                let mut bl = 0.0;
                for part in beat.split_whitespace() {
                    let mut it = part.split('/');
                    let n: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(1.0);
                    let d: f64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(4.0);
                    bl += n / d;
                }
                if let Ok(b) = bpm.trim().parse::<f64>() {
                    *qpm = Some(b * bl / 0.25);
                }
            } else if let Ok(b) = v.trim().parse::<f64>() {
                *qpm = Some(b * p.unit / 0.25);
            }
        }
        'T' if info.title.is_empty() => info.title = val.to_string(),
        'R' if info.rhythm.is_empty() => info.rhythm = val.to_string(),
        _ => {}
    }
}

/// Parse one tune into a playable song. `bank`/`program` choose the sound.
pub fn to_song(text: &str, bank: i32, program: i32) -> Result<(Song, TuneInfo), String> {
    let mut info = TuneInfo::default();
    let mut qpm: Option<f64> = None;
    let mut p = Parser {
        acc: [0; 7],
        bar_acc: Default::default(),
        unit: 0.0,
        evs: Vec::new(),
        tup_left: 0,
        tup_mul: 1.0,
        pending_mul: 1.0,
        rep_start: 0,
        ending1: None,
        in_ending2: false,
        downbeat: true,
        meter: 1.0,
    };
    let mut in_body = false;
    let mut first_voice: Option<String> = None;
    let mut voice_ok = true;
    for raw in text.lines() {
        let line = raw.split('%').next().unwrap_or("");
        let t = line.trim();
        if t.is_empty() {
            if in_body {
                // a blank line ends the tune
                break;
            }
            continue;
        }
        // header / field lines
        let b = t.as_bytes();
        let looks_like_music = in_body && b.len() > 2 && (b[2] == b':' || b[2] == b'|');
        if b.len() >= 2 && b[1] == b':' && (b[0] as char).is_ascii_alphabetic() && !looks_like_music {
            let name = b[0] as char;
            let val = &t[2..];
            match name {
                'w' | 'W' => continue,
                'V' => {
                    let id = val.split_whitespace().next().unwrap_or("").to_string();
                    if first_voice.is_none() {
                        first_voice = Some(id.clone());
                    }
                    voice_ok = first_voice.as_deref() == Some(id.as_str());
                    continue;
                }
                _ => {}
            }
            if p.unit == 0.0 && name == 'K' {
                // default unit length from the meter
                p.unit = if p.meter < 0.75 { 1.0 / 16.0 } else { 1.0 / 8.0 };
            }
            field(&mut p, name, val, &mut info, &mut qpm);
            if name == 'K' {
                in_body = true;
            }
            continue;
        }
        if !in_body {
            continue;
        }
        if !voice_ok {
            continue;
        }
        if p.unit == 0.0 {
            p.unit = 1.0 / 8.0;
        }
        let chars: Vec<char> = t.chars().collect();
        let mut i = 0usize;
        while i < chars.len() {
            let c = chars[i];
            match c {
                ' ' | '\t' | '\\' | '`' => i += 1,
                '"' => {
                    // chord symbol / annotation
                    i += 1;
                    while i < chars.len() && chars[i] != '"' {
                        i += 1;
                    }
                    i += 1;
                }
                '!' | '+' => {
                    // !decoration!
                    let close = chars[i + 1..].iter().position(|&x| x == c);
                    i = match close {
                        Some(n) => i + n + 2,
                        None => i + 1,
                    };
                }
                '{' => {
                    // grace notes: skipped
                    while i < chars.len() && chars[i] != '}' {
                        i += 1;
                    }
                    i += 1;
                }
                '~' | '.' | 'H' | 'L' | 'M' | 'O' | 'P' | 'S' | 'T' | 'u' | 'v' | 'R' | 'J' => i += 1,
                '(' => {
                    i += 1;
                    let mut num = String::new();
                    while i < chars.len() && chars[i].is_ascii_digit() {
                        num.push(chars[i]);
                        i += 1;
                    }
                    if let Ok(n) = num.parse::<u32>() {
                        // (p:q:r
                        let mut q: Option<u32> = None;
                        let mut r: Option<u32> = None;
                        if chars.get(i) == Some(&':') {
                            i += 1;
                            let mut s = String::new();
                            while i < chars.len() && chars[i].is_ascii_digit() {
                                s.push(chars[i]);
                                i += 1;
                            }
                            q = s.parse().ok();
                            if chars.get(i) == Some(&':') {
                                i += 1;
                                let mut s = String::new();
                                while i < chars.len() && chars[i].is_ascii_digit() {
                                    s.push(chars[i]);
                                    i += 1;
                                }
                                r = s.parse().ok();
                            }
                        }
                        let compound = (p.meter * 8.0).round() as i32 % 3 == 0 && p.meter > 0.5;
                        let qd = q.unwrap_or(match n {
                            2 | 4 | 8 => 3,
                            3 | 6 => 2,
                            _ => {
                                if compound {
                                    3
                                } else {
                                    2
                                }
                            }
                        });
                        p.tup_mul = qd as f64 / n as f64;
                        p.tup_left = r.unwrap_or(n);
                    }
                    // otherwise a slur: ignore
                }
                ')' => i += 1,
                '-' => {
                    if let Some(last) = p.evs.last_mut() {
                        last.tie = true;
                    }
                    i += 1;
                }
                '>' | '<' => {
                    let mut n = 0;
                    while i < chars.len() && chars[i] == c {
                        n += 1;
                        i += 1;
                    }
                    let f = 1.0 - 0.5f64.powi(n);
                    // dotted first note, shortened second (or the reverse)
                    let (a, b) = if c == '>' { (1.0 + f, 1.0 - f) } else { (1.0 - f, 1.0 + f) };
                    if let Some(last) = p.evs.last_mut() {
                        last.len *= a;
                    }
                    p.pending_mul = b;
                }
                '|' | ':' | '[' | ']' => {
                    // inline field [K:..] [M:..] [L:..]
                    if c == '[' && chars.get(i + 2) == Some(&':') && chars.get(i + 1).map(|x| x.is_ascii_alphabetic()).unwrap_or(false) {
                        let end = chars[i..].iter().position(|&x| x == ']').map(|n| i + n).unwrap_or(chars.len());
                        let name = chars[i + 1];
                        let val: String = chars[i + 3..end].iter().collect();
                        field(&mut p, name, &val, &mut info, &mut qpm);
                        i = end + 1;
                        continue;
                    }
                    // chord [CEG]
                    if c == '[' && chars.get(i + 1).map(|x| letter_idx(*x).is_some() || "^_=".contains(*x)).unwrap_or(false) {
                        i += 1;
                        let mut keys = Vec::new();
                        let mut first_len: Option<f64> = None;
                        while i < chars.len() && chars[i] != ']' {
                            if let Some(k) = read_pitch(&mut p, &chars, &mut i) {
                                let l = read_len(&chars, &mut i);
                                first_len.get_or_insert(l);
                                keys.push(k);
                            } else {
                                i += 1;
                            }
                        }
                        i += 1; // ]
                        let outer = read_len(&chars, &mut i);
                        let len = first_len.unwrap_or(1.0) * outer * p.unit;
                        let tie = chars.get(i) == Some(&'-');
                        if tie {
                            i += 1;
                        }
                        p.push(keys, len, tie);
                        continue;
                    }
                    // a bar line / repeat sign run: collect it
                    let start = i;
                    while i < chars.len() && "|:[]".contains(chars[i]) {
                        // stop before an inline field or chord
                        if chars[i] == '[' && i + 1 < chars.len() && !chars[i + 1].is_ascii_digit() && chars[i + 1] != '|' {
                            break;
                        }
                        i += 1;
                    }
                    let sym: String = chars[start..i].iter().collect();
                    let mut num = String::new();
                    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == ',' || chars[i] == '-') && (num.is_empty() || !chars[i].is_ascii_digit() || true) {
                        if !chars[i].is_ascii_digit() && num.is_empty() {
                            break;
                        }
                        num.push(chars[i]);
                        i += 1;
                    }
                    if sym.is_empty() {
                        i += 1;
                        continue;
                    }
                    p.bar();
                    let ends_rep = sym.starts_with(':') || sym.contains(":|") || sym == "::";
                    let starts_rep = sym.ends_with(':') || sym.contains("|:") || sym == "::";
                    if ends_rep {
                        p.repeat_end();
                    }
                    if starts_rep {
                        p.repeat_start();
                    }
                    if !num.is_empty() {
                        if num.starts_with('1') && !p.in_ending2 {
                            p.ending1 = Some(p.evs.len());
                        }
                    } else if (sym == "||" || sym == "|]" || sym == "[|") && !ends_rep {
                        p.repeat_start();
                    }
                }
                'z' | 'x' | 'Z' | 'X' => {
                    i += 1;
                    let l = read_len(&chars, &mut i);
                    let len = if c == 'Z' || c == 'X' { l * p.meter } else { l * p.unit };
                    p.push(Vec::new(), len, false);
                }
                _ => {
                    if letter_idx(c).is_some() || "^_=".contains(c) {
                        let save = i;
                        match read_pitch(&mut p, &chars, &mut i) {
                            Some(k) => {
                                let l = read_len(&chars, &mut i);
                                let tie = chars.get(i) == Some(&'-');
                                if tie {
                                    i += 1;
                                }
                                p.push(vec![k], l * p.unit, tie);
                            }
                            None => i = save + 1,
                        }
                    } else {
                        i += 1;
                    }
                }
            }
        }
    }
    if p.evs.iter().all(|e| e.keys.is_empty()) {
        return Err("That ABC tune has no notes Noodle could read.".into());
    }
    if info.title.is_empty() {
        info.title = "Untitled".into();
    }
    if info.scale.is_empty() {
        info.scale = "Major (Ionian)";
        info.key_label = "C major".into();
    }
    let qpm = qpm.unwrap_or_else(|| default_qpm(&info.rhythm, &info.meter));
    let sec_per_whole = 60.0 / qpm * 4.0;

    // flatten to timed notes, merging ties
    let mut notes: Vec<SongNote> = Vec::new();
    let mut t = 0.0f64;
    let mut tied_from: Vec<usize> = Vec::new();
    for e in &p.evs {
        let d = e.len * sec_per_whole;
        let mut next_tied = Vec::new();
        for &k in &e.keys {
            if let Some(&ni) = tied_from.iter().find(|&&ni| notes[ni].key == k) {
                notes[ni].end = t + d;
                if e.tie {
                    next_tied.push(ni);
                }
                continue;
            }
            let vel = if e.downbeat { 104 } else { 88 };
            notes.push(SongNote { start: t, end: t + d * 0.92, key: k, vel, ch: 0 });
            if e.tie {
                next_tied.push(notes.len() - 1);
            }
        }
        tied_from = next_tied;
        t += d;
    }
    for n in notes.iter_mut() {
        n.end = n.end.max(n.start + 0.03);
    }
    let mut evs = vec![
        SongEv { t: 0.0, ch: 0, st: 0xB0, d1: 0, d2: bank.clamp(0, 127) as u8 },
        SongEv { t: 0.0, ch: 0, st: 0xC0, d1: program.clamp(0, 127) as u8, d2: 0 },
    ];
    for n in &notes {
        evs.push(SongEv { t: n.start, ch: 0, st: 0x90, d1: n.key, d2: n.vel });
        evs.push(SongEv { t: n.end, ch: 0, st: 0x80, d1: n.key, d2: 0 });
    }
    // note-offs first at equal times
    evs.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap().then((a.st == 0x90).cmp(&(b.st == 0x90))));
    let dur = t + 0.5;
    let mut programs = [-1i16; 16];
    programs[0] = program as i16;
    let mut npc = [0u32; 16];
    npc[0] = notes.len() as u32;
    let song = Song { name: format!("{} ({})", info.title, info.key_label), notes, evs, dur, channels: 1, programs, notes_per_ch: npc };
    Ok((song, info))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(abc: &str) -> Vec<u8> {
        let (s, _) = to_song(abc, 0, 0).unwrap();
        s.notes.iter().map(|n| n.key).collect()
    }

    #[test]
    fn keys_and_modes() {
        let (_, i) = to_song("X:1\nK:Edor\nE", 0, 0).unwrap();
        assert_eq!((i.tonic, i.scale), (4, "Dorian"));
        // E dorian = D major signature: F♯ C♯
        assert_eq!(keys("X:1\nK:Edor\nFcf"), vec![66, 73, 78]);
        // G major F♯; bar-scoped natural
        assert_eq!(keys("X:1\nK:G\n=F F | F"), vec![65, 65, 66]);
        // B♭ major
        assert_eq!(keys("X:1\nK:Bb\nBE"), vec![70, 63]);
        // klezmer: D phrygian with ^f = freygish
        let (s, i) = to_song("X:1\nK:D phr ^f\nDEFG", 0, 0).unwrap();
        assert_eq!(s.notes.iter().map(|n| n.key).collect::<Vec<_>>(), vec![62, 63, 66, 67]);
        assert_eq!(i.scale, "Freygish (Ahava Rabboh)");
        // pipes
        assert_eq!(keys("X:1\nK:Hp\nFcg"), vec![66, 73, 79]);
        // octaves
        assert_eq!(keys("X:1\nK:C\nC, C c c'"), vec![48, 60, 72, 84]);
    }

    #[test]
    fn lengths_and_rhythm() {
        let (s, _) = to_song("X:1\nM:4/4\nL:1/8\nQ:1/4=120\nK:C\nC2 D E/ F/ G>A (3Bcd", 0, 0).unwrap();
        let st: Vec<f64> = s.notes.iter().map(|n| (n.start * 1000.0).round() / 1000.0).collect();
        // eighth = 0.25 s at 120 qpm
        assert_eq!(st, vec![0.0, 0.5, 0.75, 0.875, 1.0, 1.375, 1.5, 1.667, 1.833]);
    }

    #[test]
    fn repeats_and_endings() {
        // |: A B |1 C :|2 D |]  →  A B C A B D
        assert_eq!(keys("X:1\nK:C\n|:AB|1c:|2d|]"), vec![69, 71, 72, 69, 71, 74]);
        // plain repeat
        assert_eq!(keys("X:1\nK:C\n|:AB:|cd|"), vec![69, 71, 69, 71, 72, 74]);
        // :: between two repeated parts
        assert_eq!(keys("X:1\nK:C\nA::B:|"), vec![69, 69, 71, 71]);
        // chord and tie
        let (s, _) = to_song("X:1\nL:1/4\nQ:1/4=60\nK:C\n[CEG]2 A-A", 0, 0).unwrap();
        assert_eq!(s.notes.len(), 4);
        assert!((s.notes[3].end - 4.0).abs() < 0.01);
    }

    /// cargo test --release abc::tests::whole_session -- --ignored --nocapture
    /// (needs The Session's tunes.json at /tmp/sess/tunes.json)
    #[test]
    #[ignore]
    fn whole_session() {
        let v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string("/tmp/sess/tunes.json").unwrap()).unwrap();
        let mut bad = 0;
        let mut odd = Vec::new();
        let all = v.as_array().unwrap();
        for t in all {
            let g = |k: &str| t[k].as_str().unwrap_or("").to_string();
            let text = from_session(&g("name"), &g("type"), &g("meter"), &g("mode"), &g("abc"));
            match to_song(&text, 0, 0) {
                Ok((s, _)) => {
                    let lo = s.notes.iter().map(|n| n.key).min().unwrap();
                    let hi = s.notes.iter().map(|n| n.key).max().unwrap();
                    if lo < 36 || hi > 100 || s.dur < 3.0 || s.dur > 900.0 {
                        odd.push(format!("{} [{}] lo {} hi {} dur {:.0}", g("name"), g("setting_id"), lo, hi, s.dur));
                    }
                }
                Err(_) => bad += 1,
            }
        }
        for t in all.iter().filter(|t| ["27", "55", "1"].contains(&t["tune_id"].as_str().unwrap_or("")) ).take(3) {
            let g = |k: &str| t[k].as_str().unwrap_or("").to_string();
            let (s, i) = to_song(&from_session(&g("name"), &g("type"), &g("meter"), &g("mode"), &g("abc")), 0, 0).unwrap();
            let bar = if g("meter") == "6/8" { 0.75 } else { 1.0 } * 4.0 * 60.0 / if g("type") == "jig" { 165.0 } else { 180.0 };
            println!("{} | {} | {} notes | {:.1}s = {:.1} bars\n{}", i.title, i.key_label, s.notes.len(), s.dur - 0.5, (s.dur - 0.5) / bar, g("abc"));
        }
        println!("{} settings, {} unreadable, {} odd", all.len(), bad, odd.len());
        for o in odd.iter().take(25) {
            println!("  {o}");
        }
        assert!(bad * 200 < all.len());
    }

    #[test]
    fn session_header() {
        let t = from_session("The Kesh", "jig", "6/8", "Gmajor", "|:G3 GAB|A3 ABd:|");
        let (s, i) = to_song(&t, 0, 0).unwrap();
        assert_eq!(i.key_label, "G major");
        assert_eq!(s.notes.len(), 16);
        assert_eq!(split_tunes("X:1\nT:a\nK:C\nC\n\nX:2\nT:b\nK:D\nD\n").len(), 2);
    }
}
