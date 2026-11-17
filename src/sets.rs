// Noodle — sets: the tunes for a jam, in order, to practise.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Paste the setlist from the e-mail; every line becomes a tune and Noodle looks each one up
// in your files, the klezmer collection and The Session. Saved in sets.json in Noodle's folder.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum Source {
    /// a file of yours: MIDI, MuseScore, MusicXML, PDF, ABC
    File(PathBuf),
    /// an ABC tune from a collection (kept here, so it works offline): where from, the text
    Abc(String, String),
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Item {
    pub title: String,
    /// "First Set:" lines: a heading, not a tune
    #[serde(default)]
    pub heading: bool,
    #[serde(default)]
    pub source: Option<Source>,
    #[serde(default)]
    pub learned: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct Set {
    pub name: String,
    pub items: Vec<Item>,
}

fn path() -> PathBuf {
    crate::sounds::user_dir().join("sets.json")
}

pub fn load() -> Vec<Set> {
    std::fs::read_to_string(path()).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

pub fn save(sets: &[Set]) {
    let p = path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(s) = serde_json::to_string_pretty(sets) {
        let tmp = p.with_extension("json.tmp");
        if std::fs::write(&tmp, s).is_ok() {
            let _ = std::fs::rename(tmp, p);
        }
    }
}

/// A pasted setlist → (set name, items). Lines ending in ':' are headings ("First Set:");
/// a first line like "Setlist for Nov. 18th!" names the set; "Charts:", links and blank
/// lines are skipped.
pub fn parse_setlist(text: &str) -> (String, Vec<Item>) {
    let mut name = String::new();
    let mut items = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let l = raw.trim().trim_start_matches(|c: char| c == '-' || c == '•' || c == '*' || c.is_ascii_digit() && false).trim();
        if l.is_empty() {
            continue;
        }
        let low = l.to_lowercase();
        if low.starts_with("http") || low.contains("drive.google") || low.starts_with("charts") || low == "see less" || low == "see more" || low.ends_with("- google drive") {
            continue;
        }
        if i == 0 && (low.starts_with("setlist") || low.starts_with("set list") || low.starts_with("sets for")) {
            name = l.trim_end_matches(['!', ':']).to_string();
            continue;
        }
        // "1. Tune" / "1) Tune" numbering goes; "Gasn Nign 81" keeps its number
        let l = strip_numbering(l);
        if let Some(h) = l.strip_suffix(':') {
            items.push(Item { title: h.trim().to_string(), heading: true, ..Default::default() });
        } else {
            items.push(Item { title: l.to_string(), ..Default::default() });
        }
    }
    (name, items)
}

fn strip_numbering(l: &str) -> &str {
    let digits = l.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits > 0 && digits < 3 {
        let rest = &l[digits..];
        if let Some(r) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")).or_else(|| rest.strip_prefix(".\t")) {
            return r.trim();
        }
    }
    l
}

/// A word's skeleton for matching transliterated titles: lowercase letters/digits, ch→kh,
/// tz→ts, w→v, vowels and doubled letters dropped. "Freylekhs" ≈ "Freilachs" ≈ "Frailach".
fn skeleton(w: &str) -> String {
    let w: String = w.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect();
    if w.chars().all(|c| c.is_ascii_digit()) {
        return w;
    }
    let w = w.replace("ch", "kh").replace("tz", "ts").replace('w', "v").replace('y', "i").replace('j', "i");
    let mut out = String::new();
    for (i, c) in w.chars().enumerate() {
        if i > 0 && "aeiou".contains(c) {
            continue;
        }
        if out.ends_with(c) {
            continue;
        }
        out.push(c);
    }
    // plural / adjective endings: Freylekhs = Freylekh, Bulgarish ≈ Bulgar
    for end in ["s", "ish"] {
        if out.len() > 4 && out.ends_with(end) {
            out.truncate(out.len() - end.len());
        }
    }
    out
}

/// Same skeleton but for one letter added, dropped or changed ("Gas" / "Gasn").
fn one_edit(a: &str, b: &str) -> bool {
    let (a, b): (Vec<char>, Vec<char>) = (a.chars().collect(), b.chars().collect());
    let (s, l) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    if l.len() - s.len() > 1 {
        return false;
    }
    let mut i = 0;
    while i < s.len() && s[i] == l[i] {
        i += 1;
    }
    if s.len() == l.len() {
        s[i + 1..] == l[i + 1..]
    } else {
        s[i..] == l[i + 1..]
    }
}

const SKIP: [&str; 9] = ["the", "a", "an", "of", "no", "nr", "mit", "fun", "di"];

fn words(t: &str) -> Vec<String> {
    t.split(|c: char| c.is_whitespace() || c == '_' || c == '-' || c == ',' || c == '.' || c == '(' || c == ')' || c == '\'' || c == '’')
        .filter(|w| !w.is_empty() && !SKIP.contains(&w.to_lowercase().as_str()))
        .map(skeleton)
        .filter(|w| !w.is_empty())
        .collect()
}

/// How well a candidate title matches the wanted one, 0..1: the share of the wanted words
/// found in the candidate (numbers must match exactly), lightly penalising long candidates.
pub fn score(want: &str, cand: &str) -> f32 {
    let w = words(want);
    let c = words(cand);
    if w.is_empty() || c.is_empty() {
        return 0.0;
    }
    let hit = w
        .iter()
        .filter(|x| {
            c.iter().any(|y| {
                if x.chars().all(|ch| ch.is_ascii_digit()) || y.chars().all(|ch| ch.is_ascii_digit()) {
                    x.as_str() == y.as_str()
                } else {
                    x.as_str() == y.as_str() || (x.len() >= 4 && y.len() >= 4 && (x.starts_with(y.as_str()) || y.starts_with(x.as_str()))) || (x.len() >= 2 && y.len() >= 2 && one_edit(x, y))
                }
            })
        })
        .count();
    let base = hit as f32 / w.len() as f32;
    let extra = c.len().saturating_sub(w.len()) as f32;
    base * (1.0 - 0.05 * extra).max(0.6)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pasted_setlist() {
        let t = "Setlist for Nov. 18th!\nFirst Set:\nGoldenshteyn Freylekhs No. 17\nAi Raci Ku Ne Draci\n\nSecond Set:\nFetter Max’s Bulgar\nCharts: See less\nDRIVE.GOOGLE.COM\nTKS Resources - Google Drive";
        let (name, items) = parse_setlist(t);
        assert_eq!(name, "Setlist for Nov. 18th");
        let titles: Vec<(&str, bool)> = items.iter().map(|i| (i.title.as_str(), i.heading)).collect();
        assert_eq!(titles, vec![("First Set", true), ("Goldenshteyn Freylekhs No. 17", false), ("Ai Raci Ku Ne Draci", false), ("Second Set", true), ("Fetter Max’s Bulgar", false)]);
    }

    #[test]
    fn transliterations_match() {
        assert!(score("Boiberiker Khasene", "Boyberiker Chasene") > 0.9);
        assert!(score("Zilberne Khasene", "Zilberne Chasene (Silver Wedding)") > 0.6);
        assert!(score("Gasn Nign 81", "Gasn Nign 82") < 0.7, "the number has to match");
        assert!(score("Goldenshteyn Freylekhs No. 17", "Goldenshteyn_Freilachs_17") > 0.9);
        assert!(score("Old Sher", "Sher") < score("Old Sher", "Old Sher"));
        assert!(score("Flatbush", "Fletcher's Waltz") < 0.3);
        assert!(score("Gasn Nign 81", "Gas Nign 81") > 0.9);
        assert!(score("Flatbush", "Flatbush Waltz") > 0.75);
    }
}
