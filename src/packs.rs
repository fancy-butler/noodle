// Noodle — free sample instruments you can download one at a time.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Every entry is freely licensed (CC0 unless noted) and fetched straight
// from its authors' public GitHub repositories: only the instrument's .sfz
// and the samples it uses, into ~/.local/share/noodle/instruments.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering::Relaxed};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy)]
pub enum Src {
    /// one .sfz in a GitHub repo (owner, repo, branch, path)
    Sfz(&'static str, &'static str, &'static str, &'static str),
    /// a whole SoundFont file (url, file name, licence url)
    Sf2(&'static str, &'static str, &'static str),
}

#[derive(Clone, Copy)]
pub struct Pack {
    pub name: &'static str,
    pub group: &'static str,
    pub src: Src,
    pub credit: &'static str,
}

const VSCO: (&str, &str, &str) = ("sgossner", "VSCO-2-CE", "SFZ");
const VCSL: (&str, &str, &str) = ("sgossner", "VCSL", "sfz");
const fn vsco(name: &'static str, group: &'static str, path: &'static str) -> Pack {
    Pack { name, group, src: Src::Sfz(VSCO.0, VSCO.1, VSCO.2, path), credit: "VSCO 2 CE · Versilian Studios · CC0" }
}
const fn vcsl(name: &'static str, group: &'static str, path: &'static str) -> Pack {
    Pack { name, group, src: Src::Sfz(VCSL.0, VCSL.1, VCSL.2, path), credit: "VCSL · Versilian Studios · CC0" }
}

pub const PACKS: &[Pack] = &[
    Pack {
        name: "GeneralUser GS (better default bank)",
        group: "Sound bank",
        src: Src::Sf2("https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/GeneralUser-GS.sf2", "GeneralUser-GS.sf2", "https://raw.githubusercontent.com/mrbumpy409/GeneralUser-GS/main/documentation/LICENSE.txt"),
        credit: "S. Christian Collins · free licence (use without restriction)",
    },
    Pack { name: "Bagpipe in G (with drones)", group: "Pipes & whistles", src: Src::Sfz("freepats", "bagpipe", "main", "Bagpipe 20260806.sfz"), credit: "FreePats · Gilles Sadowski · CC0" },
    vcsl("Soprano recorder", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Baroque Soprano Recorder - Sustain.sfz"),
    vcsl("Alto recorder (vibrato)", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Baroque Alto Recorder - SusVib.sfz"),
    vcsl("Tenor recorder (vibrato)", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Baroque Tenor Recorder - SusVib.sfz"),
    vcsl("Bass recorder", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Baroque Bass Recorder - Sustain.sfz"),
    vcsl("Ocarina (vibrato)", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Ocarina, Typical - SusVib.sfz"),
    vcsl("Ocarina, small", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Ocarina, Small - Sustain.sfz"),
    vcsl("Ocarina (no vibrato)", "Pipes & whistles", "Aerophones/Edge-blown Aerophones/Ocarina, Typical - Sus.sfz"),
    vsco("Flute (vibrato)", "Woodwinds", "FluteSusVib.sfz"),
    vsco("Flute (no vibrato)", "Woodwinds", "FluteSusNV.sfz"),
    vsco("Flute (expressive)", "Woodwinds", "FluteExpVib.sfz"),
    vsco("Piccolo", "Woodwinds", "PiccoloSus.sfz"),
    vsco("Oboe (vibrato)", "Woodwinds", "OboeSusVib.sfz"),
    vsco("Clarinet", "Woodwinds", "ClarinetSus.sfz"),
    vsco("Bassoon", "Woodwinds", "BassoonSus.sfz"),
    vcsl("Tenor sax (vibrato)", "Woodwinds", "Aerophones/Reed Aerophones/Tenor Saxophone - Vibrato.sfz"),
    vcsl("Tenor sax (straight)", "Woodwinds", "Aerophones/Reed Aerophones/Tenor Saxophone - Non-Vibrato.sfz"),
    vcsl("Saxello (vibrato)", "Woodwinds", "Aerophones/Reed Aerophones/Saxello - Vibrato.sfz"),
    vcsl("Saxello (straight)", "Woodwinds", "Aerophones/Reed Aerophones/Saxello - Non-Vibrato.sfz"),
    vsco("Oboe (no vibrato)", "Woodwinds", "OboeSusNV.sfz"),
    vsco("Bassoon (vibrato)", "Woodwinds", "BassoonVib.sfz"),
    vcsl("Harmonica, Special 20 in C", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Special20-C - Normal.sfz"),
    vcsl("Harmonica, Special 20 in C (vibrato)", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Special20-C - Vib.sfz"),
    vcsl("Chromatic harmonica, Super 64", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Super64 - Normal.sfz"),
    vcsl("Harmonica, Special 20 in C (soft)", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Special20-C - Soft.sfz"),
    vcsl("Harmonica, Special 20 in F", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Special20-F - Normal.sfz"),
    vcsl("Harmonica, Special 20 in F (hand vibrato)", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Special20-F - HandVib.sfz"),
    vcsl("Harmonica, Special 20 in F (accented)", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Special20-F - Accented.sfz"),
    vcsl("Chromatic harmonica, Super 64 (vibrato)", "Free reeds", "Aerophones/Free Aerophones/Harmonica-Hohner-Super64 - Vib.sfz"),
    vsco("French horn", "Brass", "FHornSus.sfz"),
    vsco("Trumpet (vibrato)", "Brass", "TrumpetSusVib.sfz"),
    vsco("Trombone", "Brass", "TromboneSus.sfz"),
    vsco("Tuba", "Brass", "TubaSus.sfz"),
    vcsl("Didgeridoo", "Brass", "Aerophones/Lip Aerophones/Didgeridoo.sfz"),
    vsco("Trombone (vibrato)", "Brass", "TromboneVib.sfz"),
    vsco("Trumpet, straight mute", "Brass", "TrumpetStraightMuteSus.sfz"),
    vsco("Trumpet, harmon mute", "Brass", "TrumpetHarmonMuteSus.sfz"),
    vsco("French horn, muted", "Brass", "FHornMute.sfz"),
    vsco("Solo violin", "Strings", "SViolinVib.sfz"),
    vsco("Violin section", "Strings", "ViolinEnsSusVib.sfz"),
    vsco("Viola section", "Strings", "ViolaEnsSusVib.sfz"),
    vsco("Cello section", "Strings", "CelloEnsSusVib.sfz"),
    vsco("Double bass", "Strings", "ContrabassSusVB.sfz"),
    vsco("Double bass, pizzicato", "Strings", "ContrabassPizz.sfz"),
    vsco("Solo violin, pizzicato", "Strings", "SViolinPizz.sfz"),
    vsco("Violin section, pizzicato", "Strings", "ViolinEnsPizz.sfz"),
    vsco("Violin section, tremolo", "Strings", "ViolinEnsTrem.sfz"),
    vsco("Cello section, pizzicato", "Strings", "CelloEnsPizz.sfz"),
    Pack { name: "Double bass, arco (Smolken)", group: "Strings", src: Src::Sfz("sfzinstruments", "dsmolken.double-bass", "master", "d_smolken_rubner_bass_arco.sfz"), credit: "Double bass · D. Smolken · CC0" },
    Pack { name: "Double bass, pizzicato (Smolken)", group: "Strings", src: Src::Sfz("sfzinstruments", "dsmolken.double-bass", "master", "d_smolken_rubner_bass_pizz.sfz"), credit: "Double bass · D. Smolken · CC0" },
    Pack { name: "Meatbass (bowed double bass)", group: "Strings", src: Src::Sfz("sfzinstruments", "karoryfer.meatbass", "master", "Programs/arco_basic.sfz"), credit: "Meatbass · Karoryfer Samples · CC0" },
    vcsl("Folk harp", "Plucked", "Chordophones/Composite Chordophones/Folk Harp.sfz"),
    vcsl("Concert harp", "Plucked", "Chordophones/Composite Chordophones/Concert Harp.sfz"),
    vcsl("Strumstick", "Plucked", "Chordophones/Composite Chordophones/Strumstick.sfz"),
    vcsl("Đàn tranh (zither)", "Plucked", "Chordophones/Zithers/Dan Tranh - Normal.sfz"),
    vcsl("Bowed psaltery", "Plucked", "Chordophones/Zithers/Psaltery, Bowed and Plucked - LongBow.sfz"),
    vcsl("Kalimba (Kenya)", "Plucked", "Idiophones/Plucked Idiophones/Kalimba, Kenya.sfz"),
    vcsl("Mbira (Zimbabwe)", "Plucked", "Idiophones/Plucked Idiophones/Mbira dzaVadzimu Nyamaropa, Zimbabwe, Low B.sfz"),
    vcsl("Kalimba (Tanzania)", "Plucked", "Idiophones/Plucked Idiophones/Kalimba, Tanzania.sfz"),
    vcsl("Nyunga nyunga (Mozambique)", "Plucked", "Idiophones/Plucked Idiophones/Nyunga Nyunga, Mozambique, Low F.sfz"),
    vcsl("Đàn tranh (vibrato)", "Plucked", "Chordophones/Zithers/Dan Tranh - Vibrato.sfz"),
    vcsl("Psaltery, plucked", "Plucked", "Chordophones/Zithers/Psaltery, Bowed and Plucked - Pluck.sfz"),
    vcsl("Grand piano, Steinway B", "Keys", "Chordophones/Zithers/Grand Piano, Steinway B.sfz"),
    vcsl("Upright piano, Yamaha", "Keys", "Chordophones/Zithers/Upright Piano, Yamaha.sfz"),
    vcsl("Harpsichord, Flemish 8'", "Keys", "Chordophones/Zithers/Harpsichord, Flemish - 8'.sfz"),
    vcsl("Pipe organ", "Keys", "Aerophones/Edge-blown Aerophones/Pipe Organ - Loud.sfz"),
    vcsl("Renaissance organ 8'", "Keys", "Aerophones/Edge-blown Aerophones/Renaissance Organ - 8'.sfz"),
    vcsl("TX81Z FM piano", "Keys", "Electrophones/TX81Z - FM Piano.sfz"),
    vcsl("Harpsichord, French", "Keys", "Chordophones/Zithers/Harpsichord, French.sfz"),
    vcsl("Harpsichord, Italian", "Keys", "Chordophones/Zithers/Harpsichord, Italian.sfz"),
    vcsl("Harpsichord, English (lute stop)", "Keys", "Chordophones/Zithers/Harpsichord, English - Lute.sfz"),
    vcsl("Grand piano, Kawai", "Keys", "Chordophones/Zithers/Grand Piano, Kawai.sfz"),
    vcsl("Upright piano, Knight", "Keys", "Chordophones/Zithers/Upright Piano, Knight.sfz"),
    vcsl("TX81Z piano", "Keys", "Electrophones/TX81Z - Piano 1.sfz"),
    vcsl("TX81Z clavisynth", "Keys", "Electrophones/TX81Z - Clavisynth.sfz"),
    vcsl("Pipe organ, quiet", "Keys", "Aerophones/Edge-blown Aerophones/Pipe Organ - Quiet.sfz"),
    vcsl("Renaissance organ, full", "Keys", "Aerophones/Edge-blown Aerophones/Renaissance Organ - Full.sfz"),
    vsco("Upright piano (VSCO)", "Keys", "UprightPiano.sfz"),
    vsco("Church organ (VSCO)", "Keys", "OrganLoud.sfz"),
    Pack { name: "Rhodes electric piano (jRhodes3d)", group: "Keys", src: Src::Sfz("sfzinstruments", "jlearman.jRhodes3d", "master", "jRhodes3d-st-no-xfade.sfz"), credit: "jRhodes3d · Jeff Learman · samples CC BY-NC (free for your music)" },
    vcsl("Vibraphone", "Mallets & bells", "Idiophones/Struck Idiophones/Vibraphone - Soft Mallets.sfz"),
    vcsl("Marimba", "Mallets & bells", "Idiophones/Struck Idiophones/Marimba.sfz"),
    vcsl("Glockenspiel", "Mallets & bells", "Idiophones/Struck Idiophones/Glockenspiel.sfz"),
    vcsl("Tubular bells", "Mallets & bells", "Idiophones/Struck Idiophones/Tubular Bells 1.sfz"),
    vcsl("Hand bells, Nepalese", "Mallets & bells", "Idiophones/Struck Idiophones/Hand Bells, Nepalese.sfz"),
    vcsl("Wine glasses", "Mallets & bells", "Idiophones/Friction Idiophones/Wine Glasses - Slow.sfz"),
    vcsl("Xylophone", "Mallets & bells", "Idiophones/Struck Idiophones/Xylophone - Medium Mallets.sfz"),
    vcsl("Vibraphone, hard mallets", "Mallets & bells", "Idiophones/Struck Idiophones/Vibraphone - Hard Mallets.sfz"),
    vcsl("Vibraphone, bowed", "Mallets & bells", "Idiophones/Struck Idiophones/Vibraphone - Bowed.sfz"),
    vcsl("Balafon", "Mallets & bells", "Idiophones/Struck Idiophones/Balafon - Traditional Mallet.sfz"),
    vcsl("Hand chimes", "Mallets & bells", "Idiophones/Struck Idiophones/Hand Chimes.sfz"),
    vcsl("Tubular glockenspiel", "Mallets & bells", "Idiophones/Struck Idiophones/Tubular Glockenspiel.sfz"),
    vcsl("Slit drum", "Mallets & bells", "Idiophones/Struck Idiophones/Slit Drum.sfz"),
    vcsl("Frame drum", "Drums", "Membranophones/Struck Membranophones/Frame Drum.sfz"),
    vcsl("Darbuka", "Drums", "Membranophones/Struck Membranophones/Darbuka.sfz"),
    vcsl("Cajón", "Drums", "Idiophones/Struck Idiophones/Cajon.sfz"),
    vcsl("Bongos", "Drums", "Membranophones/Struck Membranophones/Bongos.sfz"),
    vcsl("Congas", "Drums", "Membranophones/Struck Membranophones/Conga.sfz"),
    vcsl("Tambourine", "Drums", "Idiophones/Struck Idiophones/Tambourine 1.sfz"),
    vcsl("Shaker", "Drums", "Idiophones/Struck Idiophones/Shaker, Large.sfz"),
    vcsl("Claves", "Drums", "Idiophones/Struck Idiophones/Claves.sfz"),
    vcsl("Cowbells", "Drums", "Idiophones/Struck Idiophones/Cowbells.sfz"),
    vcsl("Agogô bells", "Drums", "Idiophones/Struck Idiophones/Agogo Bells.sfz"),
    vcsl("Snare drum", "Drums", "Membranophones/Struck Membranophones/Snare Drum, Modern 1.sfz"),
    vcsl("Bass drum (concert)", "Drums", "Membranophones/Struck Membranophones/Bass Drum 1.sfz"),
    vcsl("Timpani", "Drums", "Membranophones/Struck Membranophones/Timpani 1 - Hit.sfz"),
    vsco("Orchestral percussion kit (GM keys)", "Drums", "GM-StylePerc.sfz"),
];

pub fn root() -> PathBuf {
    crate::sounds::user_dir().join("instruments")
}

/// Where an installed pack's playable file lives.
pub fn installed_path(p: &Pack) -> PathBuf {
    match p.src {
        Src::Sfz(owner, repo, _, path) => root().join(format!("{owner}-{repo}")).join(path),
        Src::Sf2(_, file, _) => crate::sounds::user_dir().join("soundfonts").join(file),
    }
}

pub fn installed(p: &Pack) -> bool {
    installed_path(p).exists()
}

#[derive(Clone)]
pub enum Status {
    Working(usize, usize),
    Done(PathBuf),
    Failed(String),
}

#[derive(Clone, Default)]
pub struct Downloads {
    pub state: Arc<Mutex<HashMap<&'static str, Status>>>,
}

fn encode(path: &str) -> String {
    path.split('/')
        .map(|seg| {
            seg.bytes()
                .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn raw_url(owner: &str, repo: &str, branch: &str, path: &str) -> String {
    format!("https://raw.githubusercontent.com/{owner}/{repo}/{branch}/{}", encode(path))
}

fn fetch(url: &str) -> Result<Vec<u8>, String> {
    let mut r = ureq::get(url).call().map_err(|e| format!("{url}: {e}"))?;
    r.body_mut().with_config().limit(512 * 1024 * 1024).read_to_vec().map_err(|e| format!("{url}: {e}"))
}

/// "a/b/../c" → "a/c"
fn normalize(p: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in p.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                out.pop();
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

fn parent(p: &str) -> &str {
    p.rsplit_once('/').map(|x| x.0).unwrap_or("")
}

/// Sample files and #include files an SFZ refers to (relative to the SFZ's folder).
pub fn sfz_refs(text: &str) -> (Vec<String>, Vec<String>) {
    let mut defines: Vec<(String, String)> = Vec::new();
    let mut includes = Vec::new();
    let mut samples = Vec::new();
    let mut default_path = String::new();
    let mut in_control = false;
    // drop block comments
    let mut t = String::new();
    let mut rest = text;
    while let Some(i) = rest.find("/*") {
        t.push_str(&rest[..i]);
        rest = rest[i..].find("*/").map(|j| &rest[i + j + 2..]).unwrap_or("");
    }
    t.push_str(rest);
    for raw in t.lines() {
        let mut line = raw.split("//").next().unwrap_or("").to_string();
        let tl = line.trim();
        if let Some(d) = tl.strip_prefix("#define") {
            let mut it = d.split_whitespace();
            if let (Some(k), Some(v)) = (it.next(), it.next()) {
                defines.push((k.to_string(), v.to_string()));
                defines.sort_by_key(|d| std::cmp::Reverse(d.0.len()));
            }
            continue;
        }
        if let Some(inc) = tl.strip_prefix("#include") {
            includes.push(inc.trim().trim_matches('"').replace('\\', "/"));
            continue;
        }
        for (k, v) in &defines {
            line = line.replace(k.as_str(), v);
        }
        // headers on this line change the context
        let mut seg = line.as_str();
        loop {
            match seg.find('<') {
                Some(i) => {
                    scan_ops(&seg[..i], in_control, &mut default_path, &mut samples);
                    let Some(j) = seg[i..].find('>') else { break };
                    in_control = seg[i + 1..i + j].trim() == "control";
                    seg = &seg[i + j + 1..];
                }
                None => {
                    scan_ops(seg, in_control, &mut default_path, &mut samples);
                    break;
                }
            }
        }
    }
    samples.sort();
    samples.dedup();
    (includes, samples)
}

fn scan_ops(seg: &str, in_control: bool, default_path: &mut String, samples: &mut Vec<String>) {
    for (k, v) in crate::sampler::opcodes_pub(seg) {
        if in_control && k == "default_path" {
            *default_path = v.replace('\\', "/");
        } else if k == "sample" && !v.starts_with('*') {
            samples.push(format!("{default_path}{}", v.replace('\\', "/")));
        }
    }
}

impl Downloads {
    pub fn status(&self, name: &str) -> Option<Status> {
        self.state.lock().ok().and_then(|m| m.get(name).cloned())
    }

    fn set(&self, name: &'static str, s: Status) {
        if let Ok(mut m) = self.state.lock() {
            m.insert(name, s);
        }
    }

    pub fn start(&self, p: Pack, ctx: eframe::egui::Context) {
        if matches!(self.status(p.name), Some(Status::Working(..))) {
            return;
        }
        self.set(p.name, Status::Working(0, 1));
        let me = self.clone();
        std::thread::spawn(move || {
            let r = me.run(p, &ctx);
            me.set(p.name, match r {
                Ok(path) => Status::Done(path),
                Err(e) => Status::Failed(e),
            });
            ctx.request_repaint();
        });
    }

    fn run(&self, p: Pack, ctx: &eframe::egui::Context) -> Result<PathBuf, String> {
        match p.src {
            Src::Sf2(url, file, lic) => {
                let dir = crate::sounds::user_dir().join("soundfonts");
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let b = fetch(url)?;
                let tmp = dir.join(format!("{file}.part"));
                std::fs::write(&tmp, &b).map_err(|e| e.to_string())?;
                if let Ok(l) = fetch(lic) {
                    let _ = std::fs::write(dir.join(format!("{file}-LICENSE.txt")), l);
                }
                let dest = dir.join(file);
                std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())?;
                Ok(dest)
            }
            Src::Sfz(owner, repo, branch, sfz) => {
                let base = root().join(format!("{owner}-{repo}"));
                // the .sfz and anything it #includes
                let mut texts: Vec<(String, Vec<u8>)> = Vec::new();
                let mut todo = vec![sfz.to_string()];
                let mut samples = Vec::new();
                while let Some(f) = todo.pop() {
                    if texts.iter().any(|t| t.0 == f) || texts.len() > 64 {
                        continue;
                    }
                    let b = fetch(&raw_url(owner, repo, branch, &f))?;
                    let (inc, smp) = sfz_refs(&String::from_utf8_lossy(&b));
                    let dir = parent(&f).to_string();
                    for i in inc {
                        todo.push(normalize(&format!("{dir}/{i}")));
                    }
                    // samples are relative to the top-level .sfz's folder
                    let top = parent(sfz);
                    for s in smp {
                        samples.push(normalize(&format!("{top}/{s}")));
                    }
                    texts.push((f, b));
                }
                samples.sort();
                samples.dedup();
                let total = samples.len();
                let done = Arc::new(AtomicUsize::new(0));
                let queue = Arc::new(Mutex::new(samples));
                let err: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
                self.set(p.name, Status::Working(0, total));
                let mut workers = Vec::new();
                for _ in 0..6 {
                    let (q, d, e, base, me, ctx) = (queue.clone(), done.clone(), err.clone(), base.clone(), self.clone(), ctx.clone());
                    workers.push(std::thread::spawn(move || loop {
                        let Some(f) = q.lock().ok().and_then(|mut q| q.pop()) else { break };
                        if e.lock().map(|e| e.is_some()).unwrap_or(true) {
                            break;
                        }
                        let dest = base.join(&f);
                        if !dest.exists() {
                            let r = fetch(&raw_url(owner, repo, branch, &f)).and_then(|b| {
                                if let Some(par) = dest.parent() {
                                    std::fs::create_dir_all(par).map_err(|e| e.to_string())?;
                                }
                                let tmp = dest.with_extension("part");
                                std::fs::write(&tmp, b).map_err(|e| e.to_string())?;
                                std::fs::rename(&tmp, &dest).map_err(|e| e.to_string())
                            });
                            if let Err(x) = r {
                                if let Ok(mut e) = e.lock() {
                                    e.get_or_insert(x);
                                }
                                break;
                            }
                        }
                        let n = d.fetch_add(1, Relaxed) + 1;
                        me.set(p.name, Status::Working(n, total));
                        ctx.request_repaint();
                    }));
                }
                for w in workers {
                    let _ = w.join();
                }
                if let Some(e) = err.lock().ok().and_then(|e| e.clone()) {
                    return Err(e);
                }
                // the .sfz files last: an instrument only appears once all its samples are here
                for (f, b) in texts.iter().rev() {
                    let dest = base.join(f);
                    if let Some(par) = dest.parent() {
                        std::fs::create_dir_all(par).map_err(|e| e.to_string())?;
                    }
                    std::fs::write(&dest, b).map_err(|e| e.to_string())?;
                }
                let lic = base.join("LICENSE-CC0.txt");
                if !lic.exists() {
                    let _ = std::fs::write(&lic, format!("{}\nhttps://github.com/{owner}/{repo}\nCreative Commons CC0 1.0: https://creativecommons.org/publicdomain/zero/1.0/\n", p.credit));
                }
                Ok(base.join(sfz))
            }
        }
    }
}

/// Delete an installed instrument (only its .sfz; shared samples stay for other instruments).
pub fn remove(p: &Pack) {
    let _ = std::fs::remove_file(installed_path(p));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refs_and_paths() {
        let (inc, smp) = sfz_refs("<control>\ndefault_path=Woodwinds\\Flute\\susvib\\\n<global> ampeg_release=0.7\n<region> sample=LDFlute A3.wav lokey=67 // c\n<region>\nsample=B 4#.wav\n#include \"x/common.sfzh\"\n");
        assert_eq!(smp, vec!["Woodwinds/Flute/susvib/B 4#.wav", "Woodwinds/Flute/susvib/LDFlute A3.wav"]);
        assert_eq!(inc, vec!["x/common.sfzh"]);
        assert_eq!(normalize("a/b/../c/./d.wav"), "a/c/d.wav");
        assert_eq!(encode("Aerophones/Ocarina, Small - Sustain.sfz"), "Aerophones/Ocarina%2C%20Small%20-%20Sustain.sfz");
        assert_eq!(encode("C#5_31.flac"), "C%235_31.flac");
    }

    /// cargo test --release packs::tests::live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn live() {
        // a real instrument end to end: refs from the live repo, then load + play
        let names: Vec<String> = std::env::var("NOODLE_PACKS").map(|v| v.split('|').map(String::from).collect()).unwrap_or_else(|_| vec!["Bagpipe in G (with drones)".into(), "Flute (vibrato)".into(), "Soprano recorder".into()]);
        for name in names.iter().map(|s| s.as_str()) {
            let p = PACKS.iter().find(|p| p.name == name).unwrap();
            let d = Downloads::default();
            let path = d.run(*p, &eframe::egui::Context::default()).unwrap();
            let inst = crate::sampler::load(&path).unwrap();
            let looped = inst.regions.iter().filter(|r| r.loop_mode != crate::sampler::LoopMode::Off).count();
            println!("{}: {} regions, mono {} from {}, {} looped", inst.name, inst.regions.len(), inst.mono, inst.mono_from, looped);
            assert!(inst.regions.len() >= 8);
        }
    }
}
