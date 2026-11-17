// Noodle — open MuseScore scores (.mscz, .mscx) and MusicXML (.musicxml, .xml, .mxl) as songs.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// If MuseScore is installed, Noodle asks it to turn the score into MIDI (most faithful:
// repeats, dynamics, every part). Without MuseScore, MusicXML is read here directly; a
// .mscz needs MuseScore (or File › Export › MusicXML from it).

use crate::song::{self, Song, SongEv, SongNote};
use std::path::{Path, PathBuf};

pub const EXTS: [&str; 5] = ["mscz", "mscx", "musicxml", "xml", "mxl"];

pub fn is_score(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).map(|e| EXTS.iter().any(|x| e.eq_ignore_ascii_case(x))).unwrap_or(false)
}

/// MuseScore's command-line program, if it's installed.
pub fn musescore() -> Option<PathBuf> {
    let names = ["mscore", "musescore", "mscore4portable", "musescore4", "mscore3", "musescore3", "MuseScore4", "MuseScore4.exe", "MuseScore3.exe"];
    if let Some(paths) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&paths) {
            for n in names {
                let p = dir.join(n);
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    let fixed = [
        "/Applications/MuseScore 4.app/Contents/MacOS/mscore",
        "/Applications/MuseScore 3.app/Contents/MacOS/mscore",
        "C:\\Program Files\\MuseScore 4\\bin\\MuseScore4.exe",
        "C:\\Program Files\\MuseScore 3\\bin\\MuseScore3.exe",
        "/var/lib/flatpak/exports/bin/org.musescore.MuseScore",
    ];
    fixed.iter().map(PathBuf::from).find(|p| p.is_file())
}

/// Load a score as a song. Slow when MuseScore does the converting (a few seconds), so run it
/// off the UI thread.
pub fn load(path: &Path) -> Result<Song, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    if let Some(ms) = musescore() {
        match via_musescore(&ms, path) {
            Ok(mut s) => {
                s.name = name;
                return Ok(s);
            }
            Err(e) if ext == "mscz" || ext == "mscx" => return Err(e),
            Err(_) => {} // MusicXML: read it ourselves instead
        }
    }
    match ext.as_str() {
        "mscz" | "mscx" => Err("Opening MuseScore files needs MuseScore installed (musescore.org). Or in MuseScore: File › Export › MusicXML, and open that.".into()),
        "mxl" => {
            let xml = mxl_text(path)?;
            from_musicxml(&xml, name)
        }
        _ => {
            let xml = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
            from_musicxml(&xml, name)
        }
    }
}

fn via_musescore(ms: &Path, path: &Path) -> Result<Song, String> {
    let out = std::env::temp_dir().join(format!("noodle-score-{}.mid", std::process::id()));
    let _ = std::fs::remove_file(&out);
    let mut cmd = if ms.to_string_lossy().ends_with("org.musescore.MuseScore") {
        let mut c = std::process::Command::new("flatpak");
        c.args(["run", "--filesystem=host", "org.musescore.MuseScore"]);
        c
    } else {
        std::process::Command::new(ms)
    };
    cmd.arg("-o").arg(&out).arg(path);
    // no window, no splash
    cmd.env("QT_QPA_PLATFORM", "offscreen");
    let res = cmd.output().map_err(|e| format!("Couldn't run MuseScore: {e}"))?;
    if !out.exists() {
        let err = String::from_utf8_lossy(&res.stderr);
        let last = err.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("no output");
        return Err(format!("MuseScore couldn't convert it ({last})"));
    }
    let s = song::load(&out);
    let _ = std::fs::remove_file(&out);
    s
}

/// The score inside a compressed MusicXML (.mxl) file.
fn mxl_text(path: &Path) -> Result<String, String> {
    use std::io::Read;
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut z = zip::ZipArchive::new(f).map_err(|e| format!("Not a readable .mxl: {e}"))?;
    // META-INF/container.xml names the score; otherwise the first .xml/.musicxml outside META-INF
    let mut target = None;
    if let Ok(mut c) = z.by_name("META-INF/container.xml") {
        let mut s = String::new();
        let _ = c.read_to_string(&mut s);
        if let Ok(doc) = roxmltree::Document::parse(&s) {
            target = doc.descendants().find(|n| n.has_tag_name("rootfile")).and_then(|n| n.attribute("full-path")).map(|s| s.to_string());
        }
    }
    if target.is_none() {
        target = (0..z.len()).filter_map(|i| z.by_index(i).ok().map(|f| f.name().to_string())).find(|n| !n.starts_with("META-INF") && (n.ends_with(".xml") || n.ends_with(".musicxml")));
    }
    let t = target.ok_or("No score inside the .mxl")?;
    let mut f = z.by_name(&t).map_err(|e| e.to_string())?;
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(s)
}

/// MusicXML (score-partwise) → song: every part on its own channel with its MIDI program,
/// chords, ties, voices (backup/forward), tempo marks. Repeats are played once.
pub fn from_musicxml(xml: &str, name: String) -> Result<Song, String> {
    let opts = roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(xml, opts).map_err(|e| format!("Not readable MusicXML: {e}"))?;
    let root = doc.root_element();
    if root.has_tag_name("score-timewise") {
        return Err("Timewise MusicXML isn't supported; export it partwise (the usual kind).".into());
    }

    // part id → MIDI program (1-based in MusicXML) and whether it's drums
    let mut progs: Vec<(String, i16, bool)> = Vec::new();
    if let Some(pl) = child(root, "part-list") {
        for sp in pl.children().filter(|c| c.has_tag_name("score-part")) {
            let id = sp.attribute("id").unwrap_or("").to_string();
            let mi = sp.descendants().find(|c| c.has_tag_name("midi-instrument"));
            let prog = mi.and_then(|m| text_of(child(m, "midi-program"))).and_then(|t| t.parse::<i16>().ok()).map(|p| p - 1).unwrap_or(0);
            let chan = mi.and_then(|m| text_of(child(m, "midi-channel"))).and_then(|t| t.parse::<u8>().ok());
            progs.push((id, prog.clamp(0, 127), chan == Some(10)));
        }
    }

    // pass 1: notes in quarter-note time per part, and the tempo map
    struct Raw {
        start: f64,
        end: f64,
        key: u8,
        ch: u8,
        tie_stop: bool,
    }
    let mut raws: Vec<Raw> = Vec::new();
    let mut tempos: Vec<(f64, f64)> = vec![(0.0, 120.0)];
    let mut next_ch = 0u8;
    let mut ch_for_part: Vec<(String, u8)> = Vec::new();
    for part in root.children().filter(|c| c.has_tag_name("part")) {
        let id = part.attribute("id").unwrap_or("").to_string();
        let drums = progs.iter().find(|p| p.0 == id).map(|p| p.2).unwrap_or(false);
        let ch = if drums {
            9
        } else {
            let c = next_ch;
            next_ch = if next_ch == 8 { 10 } else { (next_ch + 1).min(15) };
            c
        };
        ch_for_part.push((id, ch));
        let mut div = 1.0f64;
        let mut t = 0.0f64; // quarters
        let mut last_start = 0.0f64;
        for m in part.children().filter(|c| c.has_tag_name("measure")) {
            for el in m.children().filter(|c| c.is_element()) {
                match el.tag_name().name() {
                    "attributes" => {
                        if let Some(d) = text_of(child(el, "divisions")).and_then(|t| t.parse::<f64>().ok()) {
                            if d > 0.0 {
                                div = d;
                            }
                        }
                    }
                    "backup" => t -= text_of(child(el, "duration")).and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0) / div,
                    "forward" => t += text_of(child(el, "duration")).and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0) / div,
                    "direction" | "sound" => {
                        let snd = if el.has_tag_name("sound") { Some(el) } else { el.descendants().find(|c| c.has_tag_name("sound")) };
                        if let Some(bpm) = snd.and_then(|s| s.attribute("tempo")).and_then(|t| t.parse::<f64>().ok()) {
                            if bpm > 0.0 {
                                tempos.push((t.max(0.0), bpm));
                            }
                        }
                    }
                    "note" => {
                        if child(el, "grace").is_some() {
                            continue;
                        }
                        let dur = text_of(child(el, "duration")).and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0) / div;
                        let chord = child(el, "chord").is_some();
                        let start = if chord { last_start } else { t };
                        if let Some(p) = child(el, "pitch") {
                            let step = text_of(child(p, "step")).unwrap_or_default();
                            let alter = text_of(child(p, "alter")).and_then(|t| t.parse::<f64>().ok()).unwrap_or(0.0).round() as i32;
                            let oct = text_of(child(p, "octave")).and_then(|t| t.parse::<i32>().ok()).unwrap_or(4);
                            let pc = match step.as_str() {
                                "C" => 0,
                                "D" => 2,
                                "E" => 4,
                                "F" => 5,
                                "G" => 7,
                                "A" => 9,
                                "B" => 11,
                                _ => -100,
                            };
                            let key = (oct + 1) * 12 + pc + alter;
                            if (0..128).contains(&key) && dur > 0.0 {
                                let ties: Vec<&str> = el.children().filter(|c| c.has_tag_name("tie")).filter_map(|c| c.attribute("type")).collect();
                                raws.push(Raw { start, end: start + dur, key: key as u8, ch, tie_stop: ties.contains(&"stop") });
                            }
                        } else if let Some(u) = child(el, "unpitched") {
                            // drums: display-step/octave is a staff position, not a drum; use the
                            // instrument's MIDI unpitched number if there is one
                            let _ = u;
                        }
                        if !chord {
                            last_start = t;
                            t += dur;
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    if raws.is_empty() {
        return Err("No notes found in that score.".into());
    }
    // ties: a "stop" note continues the same key's note that ends where it starts
    raws.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    let mut merged: Vec<Raw> = Vec::with_capacity(raws.len());
    for r in raws {
        if r.tie_stop {
            if let Some(prev) = merged.iter_mut().rev().find(|p| p.ch == r.ch && p.key == r.key && (p.end - r.start).abs() < 1e-6) {
                prev.end = r.end;
                continue;
            }
        }
        merged.push(r);
    }
    // quarters → seconds through the tempo map
    tempos.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    tempos.dedup_by(|b, a| (a.0 - b.0).abs() < 1e-9 && {
        a.1 = b.1;
        true
    });
    let secs = |q: f64| {
        let mut s = 0.0;
        let mut at = 0.0;
        let mut bpm = tempos[0].1;
        for &(tq, b) in &tempos {
            if tq >= q {
                break;
            }
            s += (tq - at) * 60.0 / bpm;
            at = tq;
            bpm = b;
        }
        s + (q - at) * 60.0 / bpm
    };
    let mut evs = Vec::new();
    let mut programs = [-1i16; 16];
    let mut channels = 0u16;
    let mut npc = [0u32; 16];
    for (id, ch) in &ch_for_part {
        if *ch != 9 {
            let prog = progs.iter().find(|p| &p.0 == id).map(|p| p.1).unwrap_or(0);
            programs[*ch as usize] = prog;
            evs.push(SongEv { t: 0.0, ch: *ch, st: 0xC0, d1: prog as u8, d2: 0 });
        }
    }
    let mut notes = Vec::new();
    for r in &merged {
        let (s, e) = (secs(r.start) + 0.05, secs(r.end) + 0.05);
        let e = s + (e - s) * 0.97;
        notes.push(SongNote { start: s, end: e, key: r.key, vel: 90, ch: r.ch });
        evs.push(SongEv { t: s, ch: r.ch, st: 0x90, d1: r.key, d2: 90 });
        evs.push(SongEv { t: e, ch: r.ch, st: 0x80, d1: r.key, d2: 0 });
        channels |= 1 << r.ch;
        npc[r.ch as usize] += 1;
    }
    evs.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap().then((a.st == 0x90).cmp(&(b.st == 0x90))));
    let dur = notes.iter().map(|n| n.end).fold(0.0, f64::max) + 0.5;
    Ok(Song { name, notes, evs, dur, channels, programs, notes_per_ch: npc })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TWINKLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE score-partwise PUBLIC "-//Recordare//DTD MusicXML 4.0 Partwise//EN" "http://www.musicxml.org/dtds/partwise.dtd">
<score-partwise version="4.0">
  <part-list><score-part id="P1"><part-name>Flute</part-name>
    <midi-instrument id="P1-I1"><midi-channel>1</midi-channel><midi-program>74</midi-program></midi-instrument></score-part></part-list>
  <part id="P1">
    <measure number="1">
      <attributes><divisions>2</divisions></attributes>
      <direction><sound tempo="60"/></direction>
      <note><pitch><step>C</step><octave>4</octave></pitch><duration>2</duration></note>
      <note><pitch><step>E</step><octave>4</octave></pitch><duration>2</duration></note>
      <note><chord/><pitch><step>G</step><octave>4</octave></pitch><duration>2</duration></note>
      <note><pitch><step>F</step><alter>1</alter><octave>4</octave></pitch><duration>2</duration><tie type="start"/></note>
      <note><pitch><step>F</step><alter>1</alter><octave>4</octave></pitch><duration>2</duration><tie type="stop"/></note>
    </measure>
  </part>
</score-partwise>"#;

    #[test]
    fn musicxml_notes_chords_ties_tempo() {
        let s = from_musicxml(TWINKLE, "t".into()).unwrap();
        let keys: Vec<u8> = s.notes.iter().map(|n| n.key).collect();
        assert_eq!(keys, vec![60, 64, 67, 66], "C, E+G chord, tied F♯ as one note");
        // 60 bpm: a quarter is a second
        assert!((s.notes[1].start - s.notes[2].start).abs() < 1e-9, "chord notes start together");
        assert!((s.notes[1].start - s.notes[0].start - 1.0).abs() < 1e-6);
        assert!((s.notes[3].end - s.notes[3].start) > 1.9, "tie joins two quarters");
        assert_eq!(s.programs[0], 73, "flute program");
    }

    /// NOODLE_SCORE=file.mscz cargo test -- --ignored score_live: through MuseScore
    #[test]
    #[ignore]
    fn score_live() {
        let Ok(f) = std::env::var("NOODLE_SCORE") else { return };
        println!("musescore: {:?}", musescore());
        let s = load(Path::new(&f)).unwrap();
        println!("{} notes, {:.1}s, keys {:?}", s.notes.len(), s.dur, s.notes.iter().map(|n| n.key).collect::<Vec<_>>());
        assert!(!s.notes.is_empty());
    }
}

fn child<'a, 'i>(n: roxmltree::Node<'a, 'i>, tag: &str) -> Option<roxmltree::Node<'a, 'i>> {
    n.children().find(|c| c.has_tag_name(tag))
}

fn text_of(n: Option<roxmltree::Node<'_, '_>>) -> Option<String> {
    n.and_then(|n| n.text()).map(|t| t.trim().to_string())
}
