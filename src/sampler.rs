// Noodle — sample player for SFZ and Decent Sampler (.dspreset) instruments.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Loads the instrument (all samples into RAM as f32), then plays it with:
// key/velocity/CC/random/round-robin region selection, AHDSR amp envelopes,
// sample loops (from the SFZ or the WAV's own smpl chunk), release triggers,
// choke groups, pitch bend, CC7/CC11 (breath) volume, mod-wheel vibrato and,
// for wind instruments, legato: a new note while one is held crossfades
// instead of re-attacking.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// ---------------------------------------------------------------- samples

pub struct Sample {
    pub l: Vec<f32>,
    pub r: Option<Vec<f32>>,
    pub rate: f32,
    /// loop from the file itself (start, end inclusive)
    pub file_loop: Option<(usize, usize)>,
}

impl Sample {
    fn len(&self) -> usize {
        self.l.len()
    }
}

fn read_wav(bytes: &[u8]) -> Result<Sample, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file".into());
    }
    let u16le = |o: usize| u16::from_le_bytes([bytes[o], bytes[o + 1]]);
    let u32le = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
    let mut pos = 12;
    let (mut fmt, mut chans, mut rate, mut bits) = (1u16, 1usize, 44100u32, 16u16);
    let mut data: Option<(usize, usize)> = None;
    let mut lp = None;
    while pos + 8 <= bytes.len() {
        let id = &bytes[pos..pos + 4];
        let size = u32le(pos + 4) as usize;
        let body = pos + 8;
        let end = (body + size).min(bytes.len());
        match id {
            b"fmt " if size >= 16 => {
                fmt = u16le(body);
                chans = u16le(body + 2).max(1) as usize;
                rate = u32le(body + 4);
                bits = u16le(body + 14);
                if fmt == 0xFFFE && size >= 26 {
                    fmt = u16le(body + 24);
                }
            }
            b"data" => data = Some((body, end)),
            b"smpl" if size >= 36 => {
                let n = u32le(body + 28);
                if n > 0 && body + 36 + 24 <= end {
                    let s = u32le(body + 36 + 8) as usize;
                    let e = u32le(body + 36 + 12) as usize;
                    if e > s {
                        lp = Some((s, e));
                    }
                }
            }
            _ => {}
        }
        pos = body + size + (size & 1);
    }
    let (ds, de) = data.ok_or("WAV has no data")?;
    let raw = &bytes[ds..de];
    let bps = (bits as usize).div_ceil(8);
    let frames = raw.len() / (bps * chans);
    let mut l = Vec::with_capacity(frames);
    let mut r = if chans > 1 { Some(Vec::with_capacity(frames)) } else { None };
    let read = |o: usize| -> f32 {
        match (fmt, bits) {
            (3, 32) => f32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]),
            (3, 64) => f64::from_le_bytes(raw[o..o + 8].try_into().unwrap()) as f32,
            (_, 8) => (raw[o] as f32 - 128.0) / 128.0,
            (_, 16) => i16::from_le_bytes([raw[o], raw[o + 1]]) as f32 / 32768.0,
            (_, 24) => ((((raw[o] as u32) << 8 | (raw[o + 1] as u32) << 16 | (raw[o + 2] as u32) << 24) as i32) >> 8) as f32 / 8_388_608.0,
            (_, 32) => i32::from_le_bytes([raw[o], raw[o + 1], raw[o + 2], raw[o + 3]]) as f32 / 2_147_483_648.0,
            _ => 0.0,
        }
    };
    for f in 0..frames {
        let o = f * bps * chans;
        l.push(read(o));
        if let Some(r) = r.as_mut() {
            r.push(read(o + bps));
        }
    }
    Ok(Sample { l, r, rate: rate as f32, file_loop: lp })
}

fn read_flac(path: &Path) -> Result<Sample, String> {
    let mut rd = claxon::FlacReader::open(path).map_err(|e| e.to_string())?;
    let info = rd.streaminfo();
    let chans = info.channels as usize;
    let scale = 1.0 / (1u64 << (info.bits_per_sample - 1)) as f32;
    let mut l = Vec::new();
    let mut r = if chans > 1 { Some(Vec::new()) } else { None };
    for (i, s) in rd.samples().enumerate() {
        let v = s.map_err(|e| e.to_string())? as f32 * scale;
        match i % chans {
            0 => l.push(v),
            1 => {
                if let Some(r) = r.as_mut() {
                    r.push(v)
                }
            }
            _ => {}
        }
    }
    Ok(Sample { l, r, rate: info.sample_rate as f32, file_loop: None })
}

fn load_sample(path: &Path) -> Result<Sample, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "flac" => read_flac(path),
        _ => read_wav(&std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?),
    }
    .map_err(|e| format!("{}: {e}", path.display()))
}

// ---------------------------------------------------------------- regions

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LoopMode {
    Off,
    Continuous,
    Sustain,
    OneShot,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Trig {
    Attack,
    Release,
    First,
    Legato,
}

#[derive(Clone)]
pub struct Region {
    pub sample: Arc<Sample>,
    pub lokey: u8,
    pub hikey: u8,
    pub lovel: u8,
    pub hivel: u8,
    pub center: f32,
    pub tune: f32,
    pub keytrack: f32,
    pub gain: f32,
    pub pan: f32,
    pub veltrack: f32,
    pub delay: f32,
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub offset: usize,
    pub end: usize,
    pub loop_mode: LoopMode,
    pub loop_start: usize,
    pub loop_end: usize,
    pub trigger: Trig,
    pub seq_len: u32,
    pub seq_pos: u32,
    pub group: i64,
    pub off_by: i64,
    pub cc_range: Vec<(u8, u8, u8)>,
    pub lorand: f32,
    pub hirand: f32,
    /// crossfade length at the loop seam (0 = hard loop)
    pub xfade: usize,
}

pub struct Instrument {
    pub name: String,
    pub regions: Vec<Region>,
    /// one note at a time with legato crossfades (winds, bagpipes)
    pub mono: bool,
    /// legato only applies from this key up (bagpipes keep their drones on low keys)
    pub mono_from: u8,
    /// loudness match: every instrument lands near the same level
    pub norm: f32,
    pub source: PathBuf,
}

/// Opcode values collected from <global>/<master>/<group>/<region>.
type Ops = HashMap<String, String>;

fn note_num(s: &str) -> Option<i32> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i32>() {
        return Some(n);
    }
    // c4 = 60, c#4, db4, also "C-1"
    let lower = s.to_ascii_lowercase();
    let mut ch = lower.chars().peekable();
    let base = match ch.next()? {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => return None,
    };
    let mut acc = 0;
    while let Some(&c) = ch.peek() {
        match c {
            '#' | '♯' => acc += 1,
            'b' | '♭' => acc -= 1,
            _ => break,
        }
        ch.next();
    }
    let oct: i32 = ch.collect::<String>().parse().ok()?;
    Some((oct + 1) * 12 + base + acc)
}

fn f(o: &Ops, k: &str, d: f32) -> f32 {
    o.get(k).and_then(|v| v.trim().parse().ok()).unwrap_or(d)
}

fn key_op(o: &Ops, k: &str) -> Option<i32> {
    o.get(k).and_then(|v| note_num(v))
}

struct Loader {
    cache: HashMap<PathBuf, Arc<Sample>>,
    err: Option<String>,
    missing: usize,
}

impl Loader {
    fn get(&mut self, p: &Path) -> Option<Arc<Sample>> {
        if let Some(s) = self.cache.get(p) {
            return Some(s.clone());
        }
        match load_sample(p) {
            Ok(s) => {
                let a = Arc::new(s);
                self.cache.insert(p.to_path_buf(), a.clone());
                Some(a)
            }
            Err(e) => {
                self.missing += 1;
                self.err.get_or_insert(e);
                None
            }
        }
    }
}

fn region_from(o: &Ops, sample: Arc<Sample>) -> Region {
    let key = key_op(o, "key");
    let lokey = key_op(o, "lokey").or(key).unwrap_or(0).clamp(0, 127) as u8;
    let hikey = key_op(o, "hikey").or(key).unwrap_or(127).clamp(0, 127) as u8;
    let center = key_op(o, "pitch_keycenter").or(key).unwrap_or(60) as f32;
    let transpose = f(o, "transpose", 0.0);
    let vol_db = f(o, "volume", 0.0) + f(o, "group_volume", 0.0) + f(o, "master_volume", 0.0) + f(o, "global_volume", 0.0);
    let amp = f(o, "amplitude", 100.0) / 100.0;
    let len = sample.len();
    let (fl_s, fl_e) = sample.file_loop.unwrap_or((0, len.saturating_sub(1)));
    let has_file_loop = sample.file_loop.is_some();
    let loop_mode = match o.get("loop_mode").or(o.get("loopmode")).map(|s| s.trim()) {
        Some("loop_continuous") => LoopMode::Continuous,
        Some("loop_sustain") => LoopMode::Sustain,
        Some("one_shot") => LoopMode::OneShot,
        Some("no_loop") => LoopMode::Off,
        _ => {
            if has_file_loop || o.contains_key("loop_start") || o.contains_key("loopstart") {
                LoopMode::Continuous
            } else {
                LoopMode::Off
            }
        }
    };
    let loop_start = o.get("loop_start").or(o.get("loopstart")).and_then(|v| v.trim().parse().ok()).unwrap_or(fl_s).min(len.saturating_sub(1));
    let loop_end = o.get("loop_end").or(o.get("loopend")).and_then(|v| v.trim().parse().ok()).unwrap_or(fl_e).min(len.saturating_sub(1));
    let trigger = match o.get("trigger").map(|s| s.trim()) {
        Some("release") | Some("release_key") => Trig::Release,
        Some("first") => Trig::First,
        Some("legato") => Trig::Legato,
        _ => Trig::Attack,
    };
    let mut cc_range = Vec::new();
    for (k, v) in o {
        if let Some(n) = k.strip_prefix("locc").and_then(|n| n.parse::<u8>().ok()) {
            let lo = v.trim().parse().unwrap_or(0);
            let hi = o.get(&format!("hicc{n}")).and_then(|v| v.trim().parse().ok()).unwrap_or(127);
            cc_range.push((n, lo, hi));
        } else if let Some(n) = k.strip_prefix("hicc").and_then(|n| n.parse::<u8>().ok()) {
            if !o.contains_key(&format!("locc{n}")) {
                cc_range.push((n, 0, v.trim().parse().unwrap_or(127)));
            }
        }
    }
    Region {
        lokey,
        hikey,
        lovel: f(o, "lovel", 1.0).clamp(0.0, 127.0) as u8,
        hivel: f(o, "hivel", 127.0).clamp(0.0, 127.0) as u8,
        center: center - transpose,
        tune: f(o, "tune", 0.0) + f(o, "pitch", 0.0),
        keytrack: f(o, "pitch_keytrack", 100.0),
        gain: 10f32.powf(vol_db / 20.0) * amp,
        pan: (f(o, "pan", 0.0) / 100.0).clamp(-1.0, 1.0),
        veltrack: (f(o, "amp_veltrack", 100.0) / 100.0).clamp(-1.0, 1.0),
        delay: f(o, "ampeg_delay", 0.0),
        attack: f(o, "ampeg_attack", 0.0),
        hold: f(o, "ampeg_hold", 0.0),
        decay: f(o, "ampeg_decay", 0.0),
        sustain: (f(o, "ampeg_sustain", 100.0) / 100.0).clamp(0.0, 1.0),
        release: f(o, "ampeg_release", 0.0).max(0.008),
        offset: f(o, "offset", 0.0).max(0.0) as usize,
        end: o.get("end").and_then(|v| v.trim().parse::<usize>().ok()).map(|e| e.min(len)).unwrap_or(len),
        loop_mode,
        loop_start,
        loop_end: loop_end.max(loop_start + 1).min(len.saturating_sub(1).max(1)),
        trigger,
        seq_len: f(o, "seq_length", 1.0).max(1.0) as u32,
        seq_pos: f(o, "seq_position", 1.0).max(1.0) as u32,
        group: f(o, "group", 0.0) as i64,
        off_by: f(o, "off_by", 0.0) as i64,
        cc_range,
        lorand: f(o, "lorand", 0.0),
        hirand: f(o, "hirand", 1.0),
        xfade: 0,
        sample,
    }
}

/// Strip comments, apply #define, inline #include.
fn preprocess(path: &Path, defines: &mut HashMap<String, String>, depth: u32) -> String {
    let Ok(src) = std::fs::read(path) else { return String::new() };
    let src = String::from_utf8_lossy(&src).to_string();
    // block comments
    let mut out = String::new();
    let mut rest = src.as_str();
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        rest = match rest[i..].find("*/") {
            Some(j) => &rest[i + j + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    let mut res = String::new();
    for line in out.lines() {
        let line = match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        };
        let t = line.trim();
        if let Some(d) = t.strip_prefix("#define") {
            let mut it = d.split_whitespace();
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                defines.insert(k.to_string(), v.to_string());
            }
            continue;
        }
        if let Some(inc) = t.strip_prefix("#include") {
            let name = inc.trim().trim_matches('"');
            if depth < 8 {
                let p = path.parent().unwrap_or(Path::new(".")).join(name.replace('\\', "/"));
                res.push_str(&preprocess(&p, defines, depth + 1));
                res.push('\n');
            }
            continue;
        }
        let mut l = line.to_string();
        if l.contains('$') {
            let mut keys: Vec<&String> = defines.keys().collect();
            keys.sort_by_key(|k| std::cmp::Reverse(k.len()));
            for k in keys {
                l = l.replace(k.as_str(), &defines[k]);
            }
        }
        res.push_str(&l);
        res.push('\n');
    }
    res
}

/// Split a line into (opcode, value) pairs. Values may contain spaces (sample paths).
fn opcodes(seg: &str) -> Vec<(String, String)> {
    let b = seg.as_bytes();
    let mut starts = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            // opcode name runs back to whitespace
            let mut j = i;
            while j > 0 && (b[j - 1].is_ascii_alphanumeric() || b[j - 1] == b'_') {
                j -= 1;
            }
            if j < i && (j == 0 || b[j - 1].is_ascii_whitespace()) {
                starts.push((j, i));
            }
        }
        i += 1;
    }
    let mut v = Vec::new();
    for (n, &(ks, eq)) in starts.iter().enumerate() {
        let vend = starts.get(n + 1).map(|s| s.0).unwrap_or(b.len());
        v.push((seg[ks..eq].to_string(), seg[eq + 1..vend].trim().to_string()));
    }
    v
}

pub fn load_sfz(path: &Path) -> Result<Instrument, String> {
    let mut defines = HashMap::new();
    let text = preprocess(path, &mut defines, 0);
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut default_path = String::new();
    let mut level: [Ops; 4] = Default::default(); // global, master, group, region
    let mut header = String::new();
    let mut ld = Loader { cache: HashMap::new(), err: None, missing: 0 };
    let mut regions = Vec::new();
    let flush = |header: &str, level: &[Ops; 4], default_path: &str, regions: &mut Vec<Region>, ld: &mut Loader| {
        if header != "region" {
            return;
        }
        let mut o: Ops = HashMap::new();
        for l in level.iter() {
            for (k, v) in l {
                o.insert(k.clone(), v.clone());
            }
        }
        let Some(s) = o.get("sample") else { return };
        if s.starts_with('*') {
            return; // *sine etc: generators not supported
        }
        let p = dir.join(format!("{default_path}{s}").replace('\\', "/"));
        if let Some(sample) = ld.get(&p) {
            regions.push(region_from(&o, sample));
        }
    };
    // walk headers and opcodes in order
    let mut cursor = text.as_str();
    loop {
        let next_h = cursor.find('<');
        let seg = match next_h {
            Some(i) => &cursor[..i],
            None => cursor,
        };
        for line in seg.lines() {
            for (k, v) in opcodes(line) {
                if header == "control" {
                    if k == "default_path" {
                        default_path = v.replace('\\', "/");
                    }
                    continue;
                }
                let li = match header.as_str() {
                    "global" => 0,
                    "master" => 1,
                    "group" => 2,
                    "region" => 3,
                    _ => continue,
                };
                level[li].insert(k, v);
            }
        }
        let Some(i) = next_h else { break };
        let Some(j) = cursor[i..].find('>') else { break };
        let h = cursor[i + 1..i + j].trim().to_string();
        flush(&header, &level, &default_path, &mut regions, &mut ld);
        match h.as_str() {
            "global" => {
                level = Default::default();
            }
            "master" => {
                level[1].clear();
                level[2].clear();
                level[3].clear();
            }
            "group" => {
                level[2].clear();
                level[3].clear();
            }
            "region" => level[3].clear(),
            _ => {}
        }
        header = h;
        cursor = &cursor[i + j + 1..];
    }
    flush(&header, &level, &default_path, &mut regions, &mut ld);
    if regions.is_empty() {
        return Err(ld.err.unwrap_or_else(|| "no playable regions in that SFZ".into()));
    }
    Ok(Instrument { name: nice_name(path), regions, mono: false, mono_from: 0, norm: 1.0, source: path.to_path_buf() })
}

// ---------------------------------------------------------------- Decent Sampler

fn xml_attrs(tag: &str) -> HashMap<String, String> {
    let mut m = HashMap::new();
    let b = tag.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'=' {
            let mut j = i;
            while j > 0 && b[j - 1].is_ascii_whitespace() {
                j -= 1;
            }
            let ke = j;
            while j > 0 && !b[j - 1].is_ascii_whitespace() {
                j -= 1;
            }
            let key = tag[j..ke].to_string();
            let mut k = i + 1;
            while k < b.len() && b[k].is_ascii_whitespace() {
                k += 1;
            }
            if k < b.len() && (b[k] == b'"' || b[k] == b'\'') {
                let q = b[k];
                let vs = k + 1;
                let mut ve = vs;
                while ve < b.len() && b[ve] != q {
                    ve += 1;
                }
                let val = tag[vs..ve].replace("&amp;", "&").replace("&quot;", "\"").replace("&apos;", "'").replace("&lt;", "<").replace("&gt;", ">");
                m.insert(key, val);
                i = ve;
            }
        }
        i += 1;
    }
    m
}

fn ds_gain(v: &str) -> f32 {
    let v = v.trim();
    if let Some(db) = v.strip_suffix("dB").or(v.strip_suffix("db")) {
        10f32.powf(db.trim().parse::<f32>().unwrap_or(0.0) / 20.0)
    } else {
        v.parse().unwrap_or(1.0)
    }
}

pub fn load_dspreset(path: &Path) -> Result<Instrument, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
    let mut ld = Loader { cache: HashMap::new(), err: None, missing: 0 };
    let mut regions = Vec::new();
    let mut groups_attr: HashMap<String, String> = HashMap::new();
    let mut group_attr: HashMap<String, String> = HashMap::new();
    let mut group_on = true;
    let mut rest = text.as_str();
    while let Some(i) = rest.find('<') {
        let Some(j) = rest[i..].find('>') else { break };
        let tag = &rest[i + 1..i + j];
        rest = &rest[i + j + 1..];
        let name: String = tag.trim_start_matches('/').split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_string();
        let closing = tag.starts_with('/');
        match (name.as_str(), closing) {
            ("groups", false) => groups_attr = xml_attrs(tag),
            ("group", false) => {
                group_attr = xml_attrs(tag);
                group_on = group_attr.get("enabled").map(|v| v != "false").unwrap_or(true);
            }
            ("group", true) => {
                group_attr.clear();
                group_on = true;
            }
            ("sample", false) if group_on => {
                let s = xml_attrs(tag);
                let get = |k: &str| s.get(k).or(group_attr.get(k)).or(groups_attr.get(k)).cloned();
                let Some(p) = s.get("path") else { continue };
                let sp = dir.join(p.replace('\\', "/"));
                let Some(sample) = ld.get(&sp) else { continue };
                let mut o: Ops = HashMap::new();
                let root = get("rootNote").unwrap_or_else(|| "60".into());
                o.insert("pitch_keycenter".into(), root.clone());
                o.insert("lokey".into(), get("loNote").unwrap_or_else(|| root.clone()));
                o.insert("hikey".into(), get("hiNote").unwrap_or_else(|| root.clone()));
                for (a, b) in [("loVel", "lovel"), ("hiVel", "hivel"), ("start", "offset"), ("end", "end"), ("seqPosition", "seq_position"), ("seqLength", "seq_length"), ("loopStart", "loop_start"), ("loopEnd", "loop_end")] {
                    if let Some(v) = get(a) {
                        o.insert(b.into(), v);
                    }
                }
                if let Some(t) = get("tuning") {
                    o.insert("tune".into(), format!("{}", t.parse::<f32>().unwrap_or(0.0) * 100.0));
                }
                for (a, b, scale) in [("attack", "ampeg_attack", 1.0), ("decay", "ampeg_decay", 1.0), ("sustain", "ampeg_sustain", 100.0), ("release", "ampeg_release", 1.0)] {
                    if let Some(v) = get(a).and_then(|v| v.parse::<f32>().ok()) {
                        o.insert(b.into(), format!("{}", v * scale));
                    }
                }
                if let Some(t) = get("trigger") {
                    o.insert("trigger".into(), t.to_lowercase());
                }
                match get("loopEnabled").as_deref() {
                    Some("true") => {
                        o.insert("loop_mode".into(), "loop_continuous".into());
                    }
                    Some("false") => {
                        o.insert("loop_mode".into(), "no_loop".into());
                    }
                    _ => {}
                }
                if let Some(v) = get("pan") {
                    o.insert("pan".into(), v);
                }
                let mut r = region_from(&o, sample);
                // volume: sample × group × groups
                r.gain *= [s.get("volume"), group_attr.get("volume"), groups_attr.get("volume")].iter().flatten().map(|v| ds_gain(v)).product::<f32>();
                if get("ampVelTrack").is_some() {
                    r.veltrack = get("ampVelTrack").and_then(|v| v.parse().ok()).unwrap_or(1.0);
                }
                regions.push(r);
            }
            _ => {}
        }
    }
    if regions.is_empty() {
        return Err(ld.err.unwrap_or_else(|| "no samples in that Decent Sampler preset".into()));
    }
    Ok(Instrument { name: nice_name(path), regions, mono: false, mono_from: 0, norm: 1.0, source: path.to_path_buf() })
}

fn nice_name(p: &Path) -> String {
    p.file_stem().map(|s| s.to_string_lossy().replace('_', " ")).unwrap_or_default()
}

/// Load any supported instrument file.
pub fn load(path: &Path) -> Result<Instrument, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mut inst = match ext.as_str() {
        "sfz" => load_sfz(path)?,
        "dspreset" => load_dspreset(path)?,
        _ => return Err("unknown instrument format".into()),
    };
    // wind instruments play legato, one note at a time
    let n = inst.name.to_lowercase() + &path.to_string_lossy().to_lowercase();
    inst.mono = ["flute", "whistle", "recorder", "oboe", "clarinet", "bassoon", "sax", "horn", "trumpet", "trombone", "tuba", "pipe", "chanter", "ocarina", "piccolo", "duduk", "quena", "suling", "shakuhachi", "fife", "cornet", "euphonium", "sopilka", "kaval", "ney", "bansuri"]
        .iter()
        .any(|w| n.contains(w))
        && !n.contains("organ")
        && !n.contains("ens")
        && !n.contains("section");
    // loudness: median RMS of the regions' loudest second, times their gain
    let mut levels: Vec<f32> = inst
        .regions
        .iter()
        .filter(|r| r.trigger == Trig::Attack)
        .map(|r| {
            let d = &r.sample.l;
            let win = (r.sample.rate as usize).min(d.len()).max(1);
            let mut best = 0f32;
            let mut i = 0;
            while i + win <= d.len() {
                let e = (d[i..i + win].iter().map(|x| x * x).sum::<f32>() / win as f32).sqrt();
                best = best.max(e);
                i += win / 2;
            }
            if d.len() < win * 2 {
                best = best.max((d.iter().map(|x| x * x).sum::<f32>() / d.len().max(1) as f32).sqrt());
            }
            best * r.gain
        })
        .filter(|x| *x > 0.0)
        .collect();
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap());
    if let Some(&m) = levels.get(levels.len() / 2) {
        inst.norm = (0.55 / m).clamp(0.25, 12.0);
    }
    if inst.mono && (n.contains("bagpipe") || n.contains("drone")) {
        inst.mono_from = 60;
    }
    if inst.mono {
        // held wind notes outlast their recordings: loop the steady middle with a crossfade
        for r in inst.regions.iter_mut() {
            let len = r.sample.len();
            let secs = len as f32 / r.sample.rate;
            if r.loop_mode == LoopMode::Off && r.trigger == Trig::Attack && secs > 1.2 {
                let ls = (len as f32 * 0.35) as usize;
                let le = ((len as f32 * 0.85) as usize).min(r.end.saturating_sub(2));
                if le > ls + 1000 {
                    r.loop_mode = LoopMode::Sustain;
                    r.loop_start = ls;
                    r.loop_end = le;
                    r.xfade = ((r.sample.rate * 0.2) as usize).min((le - ls) / 3);
                }
            }
        }
    }
    Ok(inst)
}

// ---------------------------------------------------------------- player

#[derive(Clone, Copy, PartialEq)]
enum Stage {
    Delay,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
    Dead,
}

#[derive(Clone, Copy)]
struct Voice {
    r: usize,
    key: u8,
    pos: f64,
    pitch: f32, // semitones offset from region center at note-on
    vel_gain: f32,
    stage: Stage,
    t: f32,
    env: f32,
    rel_from: f32,
    fade: f32,
    fade_step: f32,
    sustained: bool,
    age: f32,
}

pub struct Player {
    inst: Arc<Instrument>,
    sr: f32,
    voices: Vec<Voice>,
    held: Vec<(u8, u8)>, // stack of (key, vel) for legato
    cc: [u8; 128],
    sustain: bool,
    bend: f32, // semitones
    seq: u32,
    rng: u32,
    vib_phase: f32,
    vib_amt: f32,
    expr: f32,
}

const MAX_VOICES: usize = 96;

impl Player {
    pub fn new(inst: Arc<Instrument>, sr: f32) -> Self {
        let mut cc = [0u8; 128];
        cc[7] = 100;
        cc[11] = 127;
        cc[10] = 64;
        Player { inst, sr, voices: Vec::with_capacity(MAX_VOICES), held: Vec::with_capacity(16), cc, sustain: false, bend: 0.0, seq: 0, rng: 0x2545_f491, vib_phase: 0.0, vib_amt: 0.0, expr: 1.0 }
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / (1u32 << 24) as f32
    }

    fn start(&mut self, key: u8, vel: u8, trig: Trig, legato: bool) {
        let rnd = self.rand();
        self.seq = self.seq.wrapping_add(1);
        let first = self.held.len() <= 1;
        let inst = self.inst.clone();
        for (i, r) in inst.regions.iter().enumerate() {
            let tr_ok = match r.trigger {
                Trig::Attack => trig == Trig::Attack,
                Trig::Release => trig == Trig::Release,
                Trig::First => trig == Trig::Attack && first,
                Trig::Legato => trig == Trig::Attack && !first,
            };
            if !tr_ok || key < r.lokey || key > r.hikey || vel < r.lovel || vel > r.hivel {
                continue;
            }
            if rnd < r.lorand || rnd >= r.hirand {
                continue;
            }
            if r.seq_len > 1 && (self.seq % r.seq_len) + 1 != r.seq_pos {
                continue;
            }
            if r.cc_range.iter().any(|&(n, lo, hi)| {
                let v = self.cc[n as usize];
                v < lo || v > hi
            }) {
                continue;
            }
            if r.group != 0 {
                for v in self.voices.iter_mut() {
                    if inst.regions[v.r].off_by == r.group && v.stage != Stage::Release {
                        v.stage = Stage::Release;
                        v.rel_from = v.env;
                        v.t = 0.0;
                    }
                }
            }
            let vn = vel as f32 / 127.0;
            let vel_gain = if r.veltrack >= 0.0 { 1.0 - r.veltrack + r.veltrack * vn * vn } else { 1.0 + r.veltrack * vn * vn };
            if self.voices.len() >= MAX_VOICES {
                // steal the oldest released (or oldest) voice
                let idx = self.voices.iter().enumerate().max_by(|a, b| (a.1.stage == Stage::Release, a.1.age).partial_cmp(&(b.1.stage == Stage::Release, b.1.age)).unwrap()).map(|x| x.0).unwrap_or(0);
                self.voices.swap_remove(idx);
            }
            let skip = if legato { ((r.sample.rate * 0.06) as usize).min(r.end.saturating_sub(1)) } else { 0 };
            self.voices.push(Voice {
                r: i,
                key,
                pos: (r.offset + skip).min(r.end.saturating_sub(1)) as f64,
                pitch: (key as f32 - r.center) * r.keytrack / 100.0 + r.tune / 100.0,
                vel_gain,
                stage: if legato { Stage::Sustain } else if r.delay > 0.0 { Stage::Delay } else { Stage::Attack },
                t: 0.0,
                env: if legato { r.sustain.max(0.001) } else { 0.0 },
                rel_from: 0.0,
                fade: if legato { 0.0 } else { 1.0 },
                fade_step: if legato { 1.0 / (0.035 * self.sr) } else { 0.0 },
                sustained: false,
                age: 0.0,
            });
        }
    }

    fn release_key(&mut self, key: u8, fast: bool) {
        let sustain = self.sustain;
        let inst = self.inst.clone();
        for v in self.voices.iter_mut().filter(|v| v.key == key && v.stage != Stage::Release && v.stage != Stage::Dead) {
            let r = &inst.regions[v.r];
            if r.loop_mode == LoopMode::OneShot && !fast {
                continue;
            }
            if sustain && !fast {
                v.sustained = true;
                continue;
            }
            if fast {
                v.fade_step = -1.0 / (0.045 * self.sr);
                v.sustained = false;
            } else {
                v.stage = Stage::Release;
                v.rel_from = v.env;
                v.t = 0.0;
            }
        }
    }

    pub fn note_on(&mut self, key: u8, vel: u8) {
        if vel == 0 {
            return self.note_off(key);
        }
        if self.inst.mono && key < self.inst.mono_from {
            // below the melodic range (drones): plain polyphonic note
            self.start(key, vel, Trig::Attack, false);
            return;
        }
        self.held.retain(|h| h.0 != key);
        let legato = self.inst.mono && !self.held.is_empty();
        if self.inst.mono {
            if let Some(&(prev, _)) = self.held.last() {
                self.release_key(prev, legato);
            }
        }
        self.held.push((key, vel));
        self.start(key, vel, Trig::Attack, legato);
    }

    pub fn note_off(&mut self, key: u8) {
        if self.inst.mono && key < self.inst.mono_from {
            self.release_key(key, false);
            return;
        }
        let was_top = self.held.last().map(|h| h.0) == Some(key);
        let vel = self.held.iter().find(|h| h.0 == key).map(|h| h.1).unwrap_or(64);
        self.held.retain(|h| h.0 != key);
        if self.inst.mono && was_top {
            if let Some(&(back, bvel)) = self.held.last() {
                // legato back to the note still held
                self.release_key(key, true);
                self.start(back, bvel, Trig::Attack, true);
                return;
            }
        }
        self.release_key(key, false);
        if self.inst.regions.iter().any(|r| r.trigger == Trig::Release) {
            self.start(key, vel, Trig::Release, false);
        }
    }

    pub fn cc(&mut self, n: u8, v: u8) {
        self.cc[n as usize] = v;
        match n {
            64 => {
                self.sustain = v >= 64;
                if !self.sustain {
                    for v in self.voices.iter_mut().filter(|v| v.sustained) {
                        v.sustained = false;
                        v.stage = Stage::Release;
                        v.rel_from = v.env;
                        v.t = 0.0;
                    }
                }
            }
            120 => self.all_off(true),
            123 => self.all_off(false),
            _ => {}
        }
    }

    pub fn bend(&mut self, v14: i32) {
        self.bend = (v14 - 8192) as f32 / 8192.0 * 2.0;
    }

    pub fn all_off(&mut self, hard: bool) {
        self.held.clear();
        self.sustain = false;
        if hard {
            self.voices.clear();
        } else {
            for v in self.voices.iter_mut() {
                v.stage = Stage::Release;
                v.rel_from = v.env;
                v.t = 0.0;
                v.sustained = false;
            }
        }
    }

    /// Mix into l/r.
    pub fn render(&mut self, out_l: &mut [f32], out_r: &mut [f32]) {
        let n = out_l.len();
        if self.voices.is_empty() {
            return;
        }
        let dt = 1.0 / self.sr;
        let block_t = n as f32 * dt;
        // vibrato from the mod wheel, 5.6 Hz, up to ±35 cents
        self.vib_amt += (self.cc[1] as f32 / 127.0 * 0.35 - self.vib_amt) * 0.2;
        self.vib_phase = (self.vib_phase + 5.6 * block_t).fract();
        let vib = (self.vib_phase * std::f32::consts::TAU).sin() * self.vib_amt;
        let vol = (self.cc[7] as f32 / 127.0).powi(2);
        let target_expr = (self.cc[11] as f32 / 127.0).powf(1.6);
        let e0 = self.expr;
        let e1 = e0 + (target_expr - e0) * 0.5;
        self.expr = e1;
        let inst = self.inst.clone();
        let sr = self.sr;
        let bend = self.bend;
        for v in self.voices.iter_mut() {
            let r = &inst.regions[v.r];
            let s = &r.sample;
            let step = (s.rate / sr) as f64 * 2f64.powf(((v.pitch + bend + vib) / 12.0) as f64);
            let (gl, gr) = {
                let p = r.pan;
                let a = (p + 1.0) * std::f32::consts::FRAC_PI_4;
                (a.cos() * std::f32::consts::SQRT_2, a.sin() * std::f32::consts::SQRT_2)
            };
            let base = r.gain * v.vel_gain * vol * 0.5 * inst.norm;
            let looping = matches!(r.loop_mode, LoopMode::Continuous) || (r.loop_mode == LoopMode::Sustain && v.stage != Stage::Release);
            let (ls, le) = (r.loop_start as f64, (r.loop_end + 1) as f64);
            let end = r.end as f64;
            let len = s.len();
            for i in 0..n {
                // envelope
                match v.stage {
                    Stage::Delay => {
                        v.t += dt;
                        if v.t >= r.delay {
                            v.stage = Stage::Attack;
                            v.t = 0.0;
                        }
                        continue;
                    }
                    Stage::Attack => {
                        v.t += dt;
                        v.env = if r.attack > 0.0 { (v.t / r.attack).min(1.0) } else { 1.0 };
                        if v.env >= 1.0 {
                            v.stage = Stage::Hold;
                            v.t = 0.0;
                        }
                    }
                    Stage::Hold => {
                        v.t += dt;
                        if v.t >= r.hold {
                            v.stage = Stage::Decay;
                            v.t = 0.0;
                        }
                    }
                    Stage::Decay => {
                        v.t += dt;
                        if r.decay > 0.0 && v.t < r.decay {
                            let k = v.t / r.decay;
                            v.env = 1.0 + (r.sustain - 1.0) * (1.0 - (-5.0 * k).exp()) / (1.0 - (-5.0f32).exp());
                        } else {
                            v.env = r.sustain;
                            v.stage = Stage::Sustain;
                        }
                    }
                    Stage::Sustain => v.env = r.sustain,
                    Stage::Release => {
                        v.t += dt;
                        let k = v.t / r.release;
                        v.env = v.rel_from * (-6.9 * k).exp();
                        if k >= 1.0 {
                            v.stage = Stage::Dead;
                            break;
                        }
                    }
                    Stage::Dead => break,
                }
                if v.fade_step != 0.0 {
                    v.fade = (v.fade + v.fade_step).clamp(0.0, 1.0);
                    if v.fade <= 0.0 && v.fade_step < 0.0 {
                        v.stage = Stage::Dead;
                        break;
                    }
                    if v.fade >= 1.0 && v.fade_step > 0.0 {
                        v.fade_step = 0.0;
                    }
                }
                if looping && v.pos >= le && le > ls {
                    v.pos -= le - ls;
                }
                if v.pos >= end - 1.0 || v.pos as usize + 2 >= len {
                    v.stage = Stage::Dead;
                    break;
                }
                // 4-point cubic (Catmull-Rom)
                let ip = v.pos as usize;
                let fr = (v.pos - ip as f64) as f32;
                let cub = |d: &[f32]| {
                    let xm1 = if ip > 0 { d[ip - 1] } else { d[ip] };
                    let (x0, x1, x2) = (d[ip], d[ip + 1], d[(ip + 2).min(len - 1)]);
                    x0 + 0.5 * fr * (x1 - xm1 + fr * (2.0 * xm1 - 5.0 * x0 + 4.0 * x1 - x2 + fr * (3.0 * (x0 - x1) + x2 - xm1)))
                };
                let a = v.env * v.fade * base * (e0 + (e1 - e0) * i as f32 / n as f32);
                let mut sl = cub(&s.l);
                let mut sr_ = s.r.as_ref().map(|d| cub(d)).unwrap_or(sl);
                if looping && r.xfade > 0 && v.pos >= le - r.xfade as f64 {
                    // approaching the seam: blend in the matching point before the loop start
                    let t = ((v.pos - (le - r.xfade as f64)) / r.xfade as f64) as f32;
                    let back = v.pos - (le - ls);
                    if back >= 1.0 {
                        let bi = back as usize;
                        let bf = (back - bi as f64) as f32;
                        let lin = |d: &[f32]| d[bi] + (d[(bi + 1).min(len - 1)] - d[bi]) * bf;
                        let (gl_, gb) = ((1.0 - t).sqrt(), t.sqrt());
                        sl = sl * gl_ + lin(&s.l) * gb;
                        sr_ = sr_ * gl_ + s.r.as_ref().map(|d| lin(d)).unwrap_or_else(|| lin(&s.l)) * gb;
                    }
                }
                out_l[i] += sl * a * gl;
                out_r[i] += sr_ * a * gr;
                v.pos += step;
            }
            v.age += block_t;
        }
        self.voices.retain(|v| v.stage != Stage::Dead);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav(path: &Path, freq: f32, secs: f32) {
        let sr = 44100u32;
        let n = (sr as f32 * secs) as usize;
        let mut data = Vec::new();
        for i in 0..n {
            let v = ((i as f32 / sr as f32 * freq * std::f32::consts::TAU).sin() * 20000.0) as i16;
            data.extend_from_slice(&v.to_le_bytes());
        }
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&((36 + data.len()) as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        b.extend_from_slice(&16u32.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&sr.to_le_bytes());
        b.extend_from_slice(&(sr * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        std::fs::write(path, b).unwrap();
    }

    #[test]
    fn sfz_parse_and_play() {
        let d = std::env::temp_dir().join("noodle_sfz_test");
        std::fs::create_dir_all(d.join("samples")).unwrap();
        wav(&d.join("samples/a4 soft.wav"), 440.0, 0.5);
        wav(&d.join("samples/a4 loud.wav"), 440.0, 0.5);
        std::fs::write(
            d.join("Test Flute.sfz"),
            "// test\n<control> default_path=samples/\n<global> ampeg_release=0.2\n<group> lokey=c4 hikey=b5 pitch_keycenter=a4\n<region> sample=a4 soft.wav hivel=64\n<region> sample=a4 loud.wav lovel=65 volume=-6\n",
        )
        .unwrap();
        let inst = load(&d.join("Test Flute.sfz")).unwrap();
        assert_eq!(inst.regions.len(), 2);
        assert!(inst.mono, "flutes play legato");
        assert_eq!((inst.regions[0].lokey, inst.regions[0].hikey, inst.regions[0].center), (60, 83, 69.0));
        assert_eq!(inst.regions[1].lovel, 65);
        let mut p = Player::new(Arc::new(inst), 48000.0);
        p.note_on(69, 100);
        let (mut l, mut r) = (vec![0.0; 4800], vec![0.0; 4800]);
        p.render(&mut l, &mut r);
        let peak = l.iter().fold(0f32, |a, b| a.max(b.abs()));
        assert!(peak > 0.05, "sound comes out ({peak})");
        // legato: second note replaces the first without a second attack voice lingering
        p.note_on(71, 100);
        let (mut l2, mut r2) = (vec![0.0; 4800], vec![0.0; 4800]);
        p.render(&mut l2, &mut r2);
        assert_eq!(p.voices.iter().filter(|v| v.key == 69).count(), 0);
        assert_eq!(p.voices.iter().filter(|v| v.key == 71).count(), 1);
        assert_eq!(note_num("c#4"), Some(61));
        assert_eq!(note_num("Bb3"), Some(58));
    }

    #[test]
    fn dspreset_parse() {
        let d = std::env::temp_dir().join("noodle_ds_test");
        std::fs::create_dir_all(d.join("Samples")).unwrap();
        wav(&d.join("Samples/D5.wav"), 587.3, 0.3);
        std::fs::write(
            d.join("Tin Whistle.dspreset"),
            r#"<?xml version="1.0"?><DecentSampler><groups attack="0.01" release="0.3" volume="-3dB"><group><sample path="Samples/D5.wav" rootNote="74" loNote="72" hiNote="76" loopEnabled="false"/></group></groups></DecentSampler>"#,
        )
        .unwrap();
        let inst = load(&d.join("Tin Whistle.dspreset")).unwrap();
        assert_eq!(inst.regions.len(), 1);
        assert!(inst.mono);
        assert_eq!((inst.regions[0].lokey, inst.regions[0].hikey), (72, 76));
        assert!((inst.regions[0].gain - 0.708).abs() < 0.01);
    }
}

/// For the downloader: the (opcode, value) pairs on one line.
pub fn opcodes_pub(seg: &str) -> Vec<(String, String)> {
    opcodes(seg)
}

#[cfg(test)]
mod live_tests {
    use super::*;
    /// cargo test --release sampler::live_tests -- --ignored --nocapture (after packs::tests::live)
    #[test]
    #[ignore]
    fn long_flute_note_has_no_seam_clicks() {
        let p = crate::packs::PACKS.iter().find(|p| p.name == "Flute (vibrato)").unwrap();
        let inst = Arc::new(load(&crate::packs::installed_path(p)).unwrap());
        let mut pl = Player::new(inst.clone(), 48000.0);
        pl.note_on(69, 100);
        let mut out = Vec::new();
        for _ in 0..(8 * 48000 / 512) {
            let (mut l, mut r) = (vec![0.0; 512], vec![0.0; 512]);
            pl.render(&mut l, &mut r);
            out.extend(l);
        }
        let diffs: Vec<f32> = out.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let mut sorted = diffs.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p999 = sorted[sorted.len() * 999 / 1000];
        let max = *sorted.last().unwrap();
        let rms_end = (out[out.len() - 48000..].iter().map(|x| x * x).sum::<f32>() / 48000.0).sqrt();
        println!("8 s held A4: max step {max:.4}, 99.9th pct {p999:.4}, rms in last second {rms_end:.4}");
        assert!(rms_end > 0.01, "still sounding after 8 s");
        assert!(max < p999 * 4.0, "no clicks");
    }
}
