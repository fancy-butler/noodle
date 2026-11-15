// Noodle — ABC tune collections published as plain web folders (one .abc
// file per tune), e.g. John Chambers' klezmer collection at MIT.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Nothing is bundled: the files are downloaded on request, straight from the
// collection's own site, into the user's Noodle folder, and the collection is
// credited in the Tunes window.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

pub struct Source {
    pub key: &'static str,
    pub tab: &'static str,
    /// folder listing; every *.abc linked from it is fetched
    pub index: &'static str,
    pub credit: &'static str,
    pub home: &'static str,
    pub blurb: &'static str,
}

pub const SOURCES: [Source; 1] = [Source {
    key: "klezmer",
    tab: "Klezmer",
    index: "https://trillian.mit.edu/~jc/music/abc/Klezmer/",
    credit: "John Chambers' klezmer ABC collection (MIT), transcribed and gathered by him and many contributors",
    home: "https://trillian.mit.edu/~jc/music/abc/",
    blurb: "John Chambers' klezmer collection: freylekhs, bulgars, horas, khosidls, terkishers, doinas and waltzes, many from Klez Kamp, KlezCamp and the standard books, most with chords. Several tunes come in more than one key; they're grouped as versions.",
}];

pub struct Version {
    pub file: String,
    pub key: String,
    pub text: String,
}

pub struct Tune {
    pub name: String,
    pub kind: String,
    pub meter: String,
    pub source: String,
    pub versions: Vec<Version>,
    pub search: String,
}

pub struct Book {
    pub tunes: Vec<Tune>,
    pub kinds: Vec<String>,
    pub files: usize,
}

pub fn dir(src: &Source) -> PathBuf {
    crate::sounds::user_dir().join("collections").join(src.key)
}

pub fn downloaded(src: &Source) -> bool {
    dir(src).join(".complete").exists()
}

pub use crate::session::Status;

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
    pub fn start(&self, src: &'static Source, fetch: bool, ctx: eframe::egui::Context) {
        if matches!(self.status(), Status::Working(_)) {
            return;
        }
        let me = self.clone();
        self.set(Status::Working("Starting…".into()));
        std::thread::spawn(move || {
            let r = (|| -> Result<Book, String> {
                if fetch || !downloaded(src) {
                    download(src, &me, &ctx)?;
                }
                me.set(Status::Working("Reading the tunes…".into()));
                ctx.request_repaint();
                load(src)
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

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut r = ureq::get(url).call().or_else(|e| {
        // the MIT server also answers plain http; try it if https is refused
        match url.strip_prefix("https://") {
            Some(rest) => ureq::get(&format!("http://{rest}")).call(),
            None => Err(e),
        }
    });
    let r = r.as_mut().map_err(|e| format!("{url}: {e}"))?;
    r.body_mut().with_config().limit(limit).read_to_vec().map_err(|e| format!("{url}: {e}"))
}

/// `.abc` links in a web server's folder listing: (href as written, file name).
pub fn abc_links(html: &str) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    let lower = html.to_ascii_lowercase();
    let mut at = 0;
    while let Some(p) = lower[at..].find("href=") {
        let s = at + p + 5;
        let Some(q) = html[s..].chars().next() else { break };
        let (start, end) = if q == '"' || q == '\'' {
            match html[s + 1..].find(q) {
                Some(e) => (s + 1, s + 1 + e),
                None => break,
            }
        } else {
            (s, s + html[s..].find(|c: char| c.is_whitespace() || c == '>').unwrap_or(html.len() - s))
        };
        let href = &html[start..end];
        at = end;
        if href.contains('/') || href.contains('?') || href.starts_with('#') || !href.to_ascii_lowercase().ends_with(".abc") {
            continue;
        }
        let name = percent_decode(href);
        if name.contains("..") || name.contains('\\') || out.iter().any(|(_, n)| *n == name) {
            continue;
        }
        out.push((href.to_string(), name));
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).replace("&amp;", "&")
}

fn download(src: &Source, me: &Loader, ctx: &eframe::egui::Context) -> Result<(), String> {
    let d = dir(src);
    std::fs::create_dir_all(&d).map_err(|e| format!("{}: {e}", d.display()))?;
    me.set(Status::Working("Reading the collection's file list…".into()));
    ctx.request_repaint();
    let html = String::from_utf8_lossy(&get(src.index, 8 << 20)?).to_string();
    let links = abc_links(&html);
    if links.is_empty() {
        return Err("The collection's page had no tune files on it — the site may have moved.".into());
    }
    let total = links.len();
    let jobs = Arc::new(Mutex::new(links));
    let done = Arc::new(AtomicUsize::new(0));
    let failed = Arc::new(AtomicUsize::new(0));
    let mut workers = Vec::new();
    for _ in 0..6 {
        let (jobs, done, failed, d, me, ctx) = (jobs.clone(), done.clone(), failed.clone(), d.clone(), me.clone(), ctx.clone());
        let base = src.index.to_string();
        workers.push(std::thread::spawn(move || loop {
            let Some((href, name)) = jobs.lock().ok().and_then(|mut j| j.pop()) else { break };
            let path = d.join(&name);
            if !path.exists() {
                match get(&format!("{base}{href}"), 4 << 20) {
                    Ok(b) => {
                        let tmp = d.join(format!("{name}.part"));
                        if std::fs::write(&tmp, &b).and_then(|_| std::fs::rename(&tmp, &path)).is_err() {
                            failed.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(_) => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
            let n = done.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 10 == 0 || n == total {
                me.set(Status::Working(format!("Downloading tunes… {n} of {total}")));
                ctx.request_repaint();
            }
        }));
    }
    for w in workers {
        let _ = w.join();
    }
    let f = failed.load(Ordering::Relaxed);
    if f * 2 > total {
        return Err(format!("{f} of {total} tune files wouldn't download. Try again later."));
    }
    let _ = std::fs::write(d.join(".complete"), format!("{total} files, {f} failed\n"));
    Ok(())
}

fn field<'a>(text: &'a str, f: &str) -> Option<&'a str> {
    text.lines().map(|l| l.trim_start()).find_map(|l| l.strip_prefix(f).map(|v| v.trim()))
}

/// Tidy a title for grouping and display: "Epstein Bulgar v.3   [Dm]" → "Epstein Bulgar v.3".
fn clean_title(t: &str) -> String {
    let t = match t.find('[') {
        Some(i) if i > 0 => &t[..i],
        _ => t,
    };
    t.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn kind_of(r: &str, title: &str) -> String {
    let first = r.split(',').next().unwrap_or("").trim().to_lowercase();
    classify(&first).or_else(|| classify(&title.to_lowercase())).or_else(|| classify(&r.to_lowercase())).unwrap_or(first)
}

fn classify(s: &str) -> Option<String> {
    if s.is_empty() {
        return None;
    }
    for (k, words) in [
        ("freylekhs", &["frey", "frei", "frej"][..]),
        ("bulgar", &["bulg"][..]),
        ("hora", &["hora", "zhok", "zhuk"][..]),
        ("khosidl", &["khos", "chos", "chus", "hasid", "chas"][..]),
        ("terkisher", &["terk", "turk"][..]),
        ("sher", &["sher"][..]),
        ("doina", &["doin", "doyn"][..]),
        ("nign", &["nign", "nigun", "niggun"][..]),
        ("waltz", &["waltz", "valse", "vals", "walzer"][..]),
        ("polka", &["polka"][..]),
        ("mazurka", &["mazur"][..]),
        ("march", &["march", "marsh"][..]),
        ("sirba", &["sirb", "sîrb"][..]),
        ("kolomeyke", &["kolom"][..]),
        ("reel", &["reel"][..]),
        ("hornpipe", &["hornpipe"][..]),
        ("jig", &["jig"][..]),
        ("tango", &["tango"][..]),
    ] {
        if words.iter().any(|w| s.contains(w)) {
            return Some(k.to_string());
        }
    }
    None
}

pub fn load(src: &Source) -> Result<Book, String> {
    load_dir(&dir(src))
}

pub fn load_dir(d: &std::path::Path) -> Result<Book, String> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(d)
        .map_err(|e| format!("{}: {e}", d.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("abc")).unwrap_or(false))
        .collect();
    names.sort();
    let mut tunes: Vec<Tune> = Vec::new();
    let mut by_name: HashMap<String, usize> = HashMap::new();
    for p in &names {
        let Ok(bytes) = std::fs::read(p) else { continue };
        let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| bytes.iter().map(|&b| b as char).collect());
        let file = p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();
        for t in crate::abc::split_tunes(&text) {
            if !t.text.lines().any(|l| l.trim_start().starts_with("K:")) {
                continue;
            }
            let raw_title = field(&t.text, "T:").or_else(|| field(&t.text, "P:")).unwrap_or(&t.title);
            let name = clean_title(raw_title);
            if name.is_empty() {
                continue;
            }
            let key = field(&t.text, "K:").unwrap_or("").to_string();
            let meter = field(&t.text, "M:").unwrap_or("").to_string();
            let r = field(&t.text, "R:").unwrap_or("");
            let kind = kind_of(r, &name);
            let source = field(&t.text, "B:").or_else(|| field(&t.text, "S:")).or_else(|| field(&t.text, "N:")).unwrap_or("").to_string();
            let v = Version { file: file.clone(), key, text: t.text.clone() };
            let g = name.to_lowercase();
            match by_name.get(&g) {
                Some(&i) => {
                    // the same file in another key is a version; identical text is a duplicate
                    if !tunes[i].versions.iter().any(|x| x.text == v.text) {
                        tunes[i].versions.push(v);
                    }
                }
                None => {
                    by_name.insert(g.clone(), tunes.len());
                    let extra: Vec<&str> = t.text.lines().filter_map(|l| l.trim_start().strip_prefix("T:")).collect();
                    let search = format!("{}\n{}\n{}", g, extra.join("\n").to_lowercase(), kind);
                    tunes.push(Tune { name, kind, meter, source, versions: vec![v], search });
                }
            }
        }
    }
    if tunes.is_empty() {
        return Err("No tunes found in the downloaded files.".into());
    }
    tunes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    let mut count: HashMap<String, usize> = HashMap::new();
    for t in &tunes {
        if !t.kind.is_empty() {
            *count.entry(t.kind.clone()).or_default() += 1;
        }
    }
    let mut kinds: Vec<(String, usize)> = count.into_iter().filter(|(_, n)| *n >= 3).collect();
    kinds.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(Book { tunes, kinds: kinds.into_iter().map(|k| k.0).collect(), files: names.len() })
}

pub fn search(book: &Book, q: &str, kind: &str, limit: usize) -> Vec<usize> {
    let words: Vec<String> = q.to_lowercase().split_whitespace().map(|w| w.to_string()).collect();
    book.tunes
        .iter()
        .enumerate()
        .filter(|(_, t)| kind.is_empty() || t.kind == kind)
        .filter(|(_, t)| words.iter().all(|w| t.search.contains(w.as_str())))
        .map(|(i, _)| i)
        .take(limit)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_an_apache_listing() {
        let html = r#"<html><body><h1>Index of /~jc/music/abc/Klezmer</h1><pre>
<a href="?C=N;O=D">Name</a> <a href="/~jc/music/abc/">Parent Directory</a>
<a href="Bb/">Bb/</a>
<a href="Bulg_Odessa1.abc">Bulg_Odessa1.abc</a>   2009-01-01 12:00  1.2K
<a href="Boiereasca_Bb.abc">Boiereasca_Bb.abc</a>
<a href="Bulgar_No.1_C.PDF">Bulgar_No.1_C.PDF</a>
<a href="Oi%20Shloimele.abc">Oi Shloimele.abc</a>
<A HREF='Freylekhs.ABC'>x</A>
<a href="Bulg_Odessa1.abc">dupe</a>
</pre></body></html>"#;
        let l = abc_links(html);
        let names: Vec<&str> = l.iter().map(|x| x.1.as_str()).collect();
        assert_eq!(names, vec!["Bulg_Odessa1.abc", "Boiereasca_Bb.abc", "Oi Shloimele.abc", "Freylekhs.ABC"]);
        assert_eq!(l[2].0, "Oi%20Shloimele.abc");
    }

    #[test]
    fn groups_keys_as_versions() {
        let d = std::env::temp_dir().join(format!("noodle-coll-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("A.abc"), "X: 1\nT: Epstein Bulgar v.3   [Dm]\nR: bulgar, freylach\nM: 4/4\nL: 1/8\nK: Dm\n\"Dm\"A2 G2 F2 E2|\n").unwrap();
        std::fs::write(d.join("A_Em.abc"), "X: 1\nT: Epstein Bulgar v.3 [Em]\nR: bulgar\nM: 4/4\nL: 1/8\nK: Em\nB2 A2 G2 F2|\n").unwrap();
        std::fs::write(d.join("W.abc"), "X: 2\nT: Oi Shloimele\nR: Waltz\nM: 3/4\nL: 1/4\nK: Em\nE G B|\n\nX: 3\nT: Odessa Bulgar #1\nB: Henry Sapoznik \"The Compleat Klezmer\" 1987\nM: 2/4\nK: Ddor^G\nA2 G2|\n").unwrap();
        let b = load_dir(&d).unwrap();
        let _ = std::fs::remove_dir_all(&d);
        assert_eq!(b.tunes.len(), 3);
        let e = b.tunes.iter().find(|t| t.name == "Epstein Bulgar v.3").unwrap();
        assert_eq!(e.versions.len(), 2);
        assert_eq!(e.kind, "bulgar");
        let o = b.tunes.iter().find(|t| t.name == "Odessa Bulgar #1").unwrap();
        assert!(o.source.contains("Compleat Klezmer"));
        assert_eq!(b.tunes.iter().find(|t| t.name == "Oi Shloimele").unwrap().kind, "waltz");
        assert_eq!(search(&b, "odessa", "", 10).len(), 1);
    }
}

#[cfg(test)]
mod net {
    /// cargo test --release collections::net -- --ignored  (needs internet)
    #[test]
    #[ignore]
    fn klezmer_listing() {
        let html = String::from_utf8_lossy(&super::get(super::SOURCES[0].index, 8 << 20).unwrap()).to_string();
        assert!(super::abc_links(&html).len() > 100);
    }
}
