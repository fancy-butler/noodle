// Noodle — The Session (thesession.org) tune book: download, cache, search.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Data: the weekly JSON export at github.com/adactio/TheSession-data, made by
// The Session's creator and its community. Downloaded on request into the
// user's Noodle folder, never bundled with the app.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const BASE: &str = "https://raw.githubusercontent.com/adactio/TheSession-data/main/json/";
const FILES: [&str; 3] = ["tunes.json", "tune_popularity.json", "aliases.json"];

pub struct Setting {
    pub id: u32,
    pub mode: String,
    pub abc: String,
    pub by: String,
}

pub struct Tune {
    pub id: u32,
    pub name: String,
    pub kind: String,
    pub meter: String,
    pub settings: Vec<Setting>,
    /// how many people have it in their tunebook
    pub pop: u32,
    /// lowercase name + aliases, for searching
    pub search: String,
    pub aliases: Vec<String>,
}

pub struct Book {
    pub tunes: Vec<Tune>,
    pub updated: String,
}

impl Tune {
    pub fn abc(&self, setting: usize) -> String {
        let s = &self.settings[setting.min(self.settings.len() - 1)];
        crate::abc::from_session(&display_name(&self.name), &self.kind, &self.meter, &s.mode, &s.abc)
    }
}

/// Pretty key from The Session's mode field ("Edorian" → "E dorian").
pub fn pretty_mode(m: &str) -> String {
    let split = m.char_indices().find(|(i, c)| *i > 0 && c.is_ascii_lowercase() && !(*i == 1 && *c == 'b')).map(|(i, _)| i).unwrap_or(m.len());
    let (root, mode) = m.split_at(split);
    format!("{} {}", root.replace('b', "♭").replace('#', "♯"), mode)
}

/// The Session files names as "Kesh, The"; show "The Kesh".
pub fn display_name(n: &str) -> String {
    for art in ["The", "An", "A", "Na", "Le", "La", "Les", "Il", "El", "Der", "Die", "Das"] {
        if let Some(rest) = n.strip_suffix(&format!(", {art}")) {
            return format!("{art} {rest}");
        }
    }
    n.to_string()
}

pub fn dir() -> PathBuf {
    crate::sounds::user_dir().join("thesession")
}

pub fn downloaded() -> bool {
    dir().join("tunes.json").exists()
}

#[derive(Clone, Default)]
pub enum Status {
    #[default]
    Idle,
    Working(String),
    Ready,
    Failed(String),
}

/// Shared between the UI and the background loader.
#[derive(Clone, Default)]
pub struct Loader {
    pub status: Arc<Mutex<Status>>,
    pub book: Arc<Mutex<Option<Arc<Book>>>>,
}

impl Loader {
    pub fn status(&self) -> Status {
        self.status.lock().map(|s| s.clone()).unwrap_or_default()
    }
    pub fn book(&self) -> Option<Arc<Book>> {
        self.book.lock().ok().and_then(|b| b.clone())
    }
    fn set(&self, s: Status) {
        if let Ok(mut g) = self.status.lock() {
            *g = s;
        }
    }

    /// Download (if `fetch` or nothing cached yet) and then index, in the background.
    pub fn start(&self, fetch: bool, ctx: eframe::egui::Context) {
        if matches!(self.status(), Status::Working(_)) {
            return;
        }
        let me = self.clone();
        self.set(Status::Working("Starting…".into()));
        std::thread::spawn(move || {
            let r = (|| -> Result<Book, String> {
                if fetch || !downloaded() {
                    download(&me, &ctx)?;
                }
                me.set(Status::Working("Reading the tune book…".into()));
                ctx.request_repaint();
                load()
            })();
            match r {
                Ok(b) => {
                    if let Ok(mut g) = me.book.lock() {
                        *g = Some(Arc::new(b));
                    }
                    me.set(Status::Ready);
                }
                Err(e) => me.set(Status::Failed(e)),
            }
            ctx.request_repaint();
        });
    }
}

fn download(me: &Loader, ctx: &eframe::egui::Context) -> Result<(), String> {
    let d = dir();
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    for (n, f) in FILES.iter().enumerate() {
        me.set(Status::Working(format!("Downloading {} ({}/{})…", f, n + 1, FILES.len())));
        ctx.request_repaint();
        let url = format!("{BASE}{f}");
        let mut resp = ureq::get(&url).call().map_err(|e| format!("Couldn't download {f}: {e}"))?;
        let bytes = resp.body_mut().with_config().limit(200 * 1024 * 1024).read_to_vec().map_err(|e| format!("Download of {f} failed: {e}"))?;
        // write then rename so a half download never replaces a good file
        let tmp = d.join(format!("{f}.part"));
        std::fs::write(&tmp, &bytes).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, d.join(f)).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(serde::Deserialize)]
struct RawSetting {
    tune_id: String,
    setting_id: String,
    name: String,
    #[serde(rename = "type")]
    kind: String,
    meter: String,
    mode: String,
    abc: String,
    #[serde(default)]
    username: String,
    #[serde(default)]
    date: String,
}
#[derive(serde::Deserialize)]
struct RawPop {
    tune_id: String,
    tunebooks: String,
}
#[derive(serde::Deserialize)]
struct RawAlias {
    tune_id: String,
    alias: String,
}

pub fn load() -> Result<Book, String> {
    let d = dir();
    let read = |f: &str| std::fs::read_to_string(d.join(f)).map_err(|e| format!("{f}: {e}"));
    let raw: Vec<RawSetting> = serde_json::from_str(&read("tunes.json")?).map_err(|e| format!("tunes.json: {e}"))?;
    let pop: Vec<RawPop> = read("tune_popularity.json").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let aliases: Vec<RawAlias> = read("aliases.json").ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
    let pop: HashMap<u32, u32> = pop.into_iter().filter_map(|p| Some((p.tune_id.parse().ok()?, p.tunebooks.parse().unwrap_or(0)))).collect();
    let mut al: HashMap<u32, Vec<String>> = HashMap::new();
    for a in aliases {
        if let Ok(id) = a.tune_id.parse() {
            al.entry(id).or_default().push(a.alias);
        }
    }
    let mut updated = String::new();
    let mut by_id: HashMap<u32, usize> = HashMap::new();
    let mut tunes: Vec<Tune> = Vec::new();
    for r in raw {
        let Ok(id) = r.tune_id.parse::<u32>() else { continue };
        if r.date > updated {
            updated = r.date.clone();
        }
        let s = Setting { id: r.setting_id.parse().unwrap_or(0), mode: r.mode, abc: r.abc, by: r.username };
        match by_id.get(&id) {
            Some(&i) => tunes[i].settings.push(s),
            None => {
                by_id.insert(id, tunes.len());
                tunes.push(Tune { id, name: r.name, kind: r.kind, meter: r.meter, settings: vec![s], pop: 0, search: String::new(), aliases: Vec::new() });
            }
        }
    }
    for t in tunes.iter_mut() {
        t.settings.sort_by_key(|s| s.id);
        t.pop = pop.get(&t.id).copied().unwrap_or(0);
        let mut a = al.remove(&t.id).unwrap_or_default();
        a.retain(|x| !x.eq_ignore_ascii_case(&t.name));
        a.dedup();
        t.search = std::iter::once(t.name.as_str()).chain(a.iter().map(|s| s.as_str())).collect::<Vec<_>>().join("\n").to_lowercase();
        t.aliases = a;
    }
    tunes.sort_by(|a, b| b.pop.cmp(&a.pop).then(a.name.cmp(&b.name)));
    if tunes.is_empty() {
        return Err("The downloaded tune book is empty.".into());
    }
    Ok(Book { tunes, updated: updated.chars().take(10).collect() })
}

/// Indices of tunes matching `q` (name or alias, every word must match) and `kind`.
pub fn search(book: &Book, q: &str, kind: &str, limit: usize) -> Vec<usize> {
    let words: Vec<String> = q.to_lowercase().split_whitespace().map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string()).filter(|w| !w.is_empty()).collect();
    book.tunes
        .iter()
        .enumerate()
        .filter(|(_, t)| kind.is_empty() || t.kind == kind)
        .filter(|(_, t)| words.iter().all(|w| t.search.contains(w.as_str())))
        .map(|(i, _)| i)
        .take(limit)
        .collect()
}

pub const KINDS: [&str; 12] = ["reel", "jig", "polka", "waltz", "hornpipe", "slip jig", "march", "barndance", "strathspey", "slide", "mazurka", "three-two"];

#[cfg(test)]
mod tests {
    #[test]
    fn modes() {
        assert_eq!(super::pretty_mode("Edorian"), "E dorian");
        assert_eq!(super::pretty_mode("Bbmajor"), "B♭ major");
        assert_eq!(super::pretty_mode("Bminor"), "B minor");
        assert_eq!(super::display_name("Kesh, The"), "The Kesh");
    }
}

#[cfg(test)]
mod net {
    /// cargo test --release session::net -- --ignored
    #[test]
    #[ignore]
    fn fetch_small_file() {
        let mut r = ureq::get(&format!("{}tune_popularity.json", super::BASE)).call().unwrap();
        let b = r.body_mut().with_config().limit(50 * 1024 * 1024).read_to_vec().unwrap();
        assert!(b.len() > 100_000);
    }
}
