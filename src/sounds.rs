// Noodle — finding and loading SoundFonts (.sf2) and MIDI files.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use crate::engine::SynthPair;
use rustysynth::{SoundFont, Synthesizer, SynthesizerSettings};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone)]
pub struct PresetInfo {
    pub bank: i32,
    pub prog: i32,
    pub name: String,
}

pub const SAMPLE_BANK: i32 = 1000;
pub const CHIP_BANK: i32 = 2000;

pub const GM_FAMILIES: [&str; 18] = [
    "Piano",
    "Chromatic perc.",
    "Organ",
    "Guitar",
    "Bass",
    "Strings",
    "Ensemble",
    "Brass",
    "Reed",
    "Pipe / flute",
    "Synth lead",
    "Synth pad",
    "Synth FX",
    "World",
    "Percussive",
    "Sound FX",
    "Samples",
    "Chiptune",
];

pub fn family(p: &PresetInfo) -> usize {
    match p.bank {
        SAMPLE_BANK => 16,
        CHIP_BANK => 17,
        _ => (p.prog / 8).clamp(0, 15) as usize,
    }
}

/// Folders searched for sample instruments (.sfz / .dspreset).
pub fn instrument_dirs() -> Vec<PathBuf> {
    let mut v = vec![user_dir().join("instruments")];
    if let Some(e) = exe_dir() {
        v.push(e.join("instruments"));
    }
    if let Some(d) = dirs::document_dir() {
        v.push(d.join("Decent Sampler"));
        v.push(d.join("Pianobook"));
    }
    if let Some(d) = dirs::audio_dir() {
        v.push(d.join("Decent Sampler"));
    }
    v
}

/// A stable id for an instrument file, so a saved choice survives new files appearing.
pub fn inst_id(p: &Path) -> i32 {
    let mut h: u32 = 2166136261;
    for b in p.to_string_lossy().bytes() {
        h = (h ^ b as u32).wrapping_mul(16777619);
    }
    (h & 0x7fff_ffff) as i32
}

/// Sample instruments and chip voices, as presets in the sound picker.
pub fn native_presets(extra: &[PathBuf]) -> (Vec<PresetInfo>, Vec<(i32, PathBuf)>) {
    let mut files = scan(&instrument_dirs(), &["sfz", "dspreset"], 4);
    for e in extra {
        if !files.contains(e) {
            files.push(e.clone());
        }
    }
    let mut v = Vec::new();
    let mut paths = Vec::new();
    let pack_names: std::collections::HashMap<PathBuf, &str> = crate::packs::PACKS.iter().map(|p| (crate::packs::installed_path(p), p.name)).collect();
    for f in files {
        let id = inst_id(&f);
        let name = pack_names
            .get(&f)
            .map(|n| n.to_string())
            .unwrap_or_else(|| f.file_stem().map(|s| s.to_string_lossy().replace(['_'], " ")).unwrap_or_default());
        v.push(PresetInfo { bank: SAMPLE_BANK, prog: id, name });
        paths.push((id, f));
    }
    v.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    for (i, p) in crate::chip::PATCHES.iter().enumerate() {
        v.push(PresetInfo { bank: CHIP_BANK, prog: i as i32, name: p.name.to_string() });
    }
    (v, paths)
}

pub fn user_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(|| PathBuf::from(".")).join("noodle")
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

pub fn soundfont_dirs() -> Vec<PathBuf> {
    let mut v = vec![user_dir().join("soundfonts")];
    if let Some(e) = exe_dir() {
        v.push(e.join("soundfonts"));
        v.push(e.clone());
    }
    if let Some(h) = dirs::home_dir() {
        v.push(h.join(".local/share/soundfonts"));
        v.push(h.join(".sounds/sf2"));
    }
    v.push("/usr/share/sounds/sf2".into());
    v.push("/usr/share/soundfonts".into());
    v.push("/usr/local/share/soundfonts".into());
    v
}

pub fn song_dirs() -> Vec<PathBuf> {
    let mut v = vec![user_dir().join("songs")];
    if let Some(e) = exe_dir() {
        v.push(e.join("songs"));
    }
    if let Some(d) = dirs::audio_dir() {
        v.push(d);
    }
    if let Some(d) = dirs::download_dir() {
        v.push(d);
    }
    if let Some(d) = dirs::document_dir() {
        v.push(d.join("MuseScore4").join("Scores"));
        v.push(d.join("MuseScore3").join("Scores"));
    }
    v
}

fn scan(dirs: &[PathBuf], exts: &[&str], depth: usize) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    fn walk(d: &Path, exts: &[&str], depth: usize, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(d) else { return };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if depth > 0 {
                    walk(&p, exts, depth - 1, out);
                }
            } else if let Some(x) = p.extension().and_then(|x| x.to_str()) {
                if exts.iter().any(|e| x.eq_ignore_ascii_case(e)) {
                    out.push(p);
                }
            }
        }
    }
    for d in dirs {
        walk(d, exts, depth, &mut out);
    }
    // de-dupe symlinks (default-GM.sf2 → FluidR3_GM.sf2)
    let mut seen = std::collections::HashSet::new();
    out.into_iter()
        .map(|p| std::fs::canonicalize(&p).unwrap_or(p))
        .filter(|p| seen.insert(p.clone()))
        .collect()
}

pub fn find_soundfonts() -> Vec<PathBuf> {
    let mut v = scan(&soundfont_dirs(), &["sf2"], 1);
    // biggest first — usually the best-sounding one
    v.sort_by_key(|p| std::cmp::Reverse(std::fs::metadata(p).map(|m| m.len()).unwrap_or(0)));
    v
}

pub fn find_songs() -> Vec<PathBuf> {
    let mut v = scan(&song_dirs(), &["mid", "midi", "kar", "abc"], 1);
    v.sort_by_key(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    v
}

pub struct Loaded {
    pub path: PathBuf,
    pub presets: Vec<PresetInfo>,
    pub synths: Box<SynthPair>,
}

pub fn load_soundfont(path: &Path, sample_rate: u32) -> Result<Loaded, String> {
    let f = std::fs::File::open(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut r = std::io::BufReader::new(f);
    let sf = Arc::new(SoundFont::new(&mut r).map_err(|e| format!("Couldn't read {}: {e:?}", path.display()))?);
    let mut presets: Vec<PresetInfo> = sf
        .get_presets()
        .iter()
        .map(|p| PresetInfo { bank: p.get_bank_number(), prog: p.get_patch_number(), name: p.get_name().trim().to_string() })
        .collect();
    presets.sort_by_key(|p| (p.bank, p.prog));
    let mut settings = SynthesizerSettings::new(sample_rate as i32);
    settings.maximum_polyphony = 160;
    let main = Synthesizer::new(&sf, &settings).map_err(|e| format!("{e:?}"))?;
    let song = Synthesizer::new(&sf, &settings).map_err(|e| format!("{e:?}"))?;
    Ok(Loaded { path: path.to_path_buf(), presets, synths: Box::new(SynthPair { main, song }) })
}
