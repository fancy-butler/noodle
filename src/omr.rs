// Noodle — fetches Audiveris, the free sheet-music reader, into Noodle's own folder.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Audiveris (AGPL-3.0, github.com/Audiveris/audiveris) is a separate Java program.
// Noodle downloads the official, unmodified release for this computer and unpacks it
// into <data>/noodle/audiveris — no system install, no admin rights — then runs it
// as its own program to turn PDFs into MusicXML.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const VERSION: &str = "5.8.1";

#[derive(Clone, Debug, PartialEq)]
pub enum State {
    Idle,
    /// bytes so far, total (0 = unknown)
    Downloading(u64, u64),
    Unpacking,
    Done,
    Failed(String),
}

#[derive(Clone)]
pub struct Fetcher {
    pub state: Arc<Mutex<State>>,
}

impl Default for Fetcher {
    fn default() -> Self {
        Fetcher { state: Arc::new(Mutex::new(State::Idle)) }
    }
}

pub fn dir() -> PathBuf {
    crate::sounds::user_dir().join("audiveris")
}

fn asset() -> &'static str {
    if cfg!(windows) {
        "windows-x86_64.msi"
    } else if cfg!(target_os = "macos") {
        if cfg!(target_arch = "aarch64") {
            "macosx-arm64.dmg"
        } else {
            "macosx-x86_64.dmg"
        }
    } else {
        // built on the older Ubuntu: runs on more distros (it carries its own Java)
        "ubuntu22.04-x86_64.deb"
    }
}

pub fn url() -> String {
    format!("https://github.com/Audiveris/audiveris/releases/download/{VERSION}/Audiveris-{VERSION}-{}", asset())
}

/// The copy Noodle fetched, if there is one.
pub fn installed() -> Option<PathBuf> {
    installed_in(&dir())
}

fn installed_in(d: &Path) -> Option<PathBuf> {
    let p = if cfg!(target_os = "macos") {
        d.join("Audiveris.app/Contents/MacOS/Audiveris")
    } else if cfg!(windows) {
        return find_file(d, "Audiveris.exe", 4);
    } else {
        d.join("bin/Audiveris")
    };
    p.is_file().then_some(p)
}

fn find_file(d: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    let rd = std::fs::read_dir(d).ok()?;
    let mut dirs = vec![];
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            dirs.push(p);
        } else if p.file_name().map(|n| n.eq_ignore_ascii_case(name)).unwrap_or(false) {
            return Some(p);
        }
    }
    if depth == 0 {
        return None;
    }
    dirs.into_iter().find_map(|p| find_file(&p, name, depth - 1))
}

impl Fetcher {
    pub fn state(&self) -> State {
        self.state.lock().unwrap().clone()
    }
    pub fn busy(&self) -> bool {
        matches!(self.state(), State::Downloading(..) | State::Unpacking)
    }

    pub fn start(&self, ctx: Option<eframe::egui::Context>) {
        if self.busy() {
            return;
        }
        *self.state.lock().unwrap() = State::Downloading(0, 0);
        let st = self.state.clone();
        std::thread::spawn(move || {
            let r = fetch_into(&dir(), &st, ctx.as_ref());
            *st.lock().unwrap() = match r {
                Ok(()) => State::Done,
                Err(e) => State::Failed(e),
            };
            if let Some(c) = ctx {
                c.request_repaint();
            }
        });
    }
}

/// Download and unpack into `dest` (replacing what's there). Blocking.
pub fn fetch_into(dest: &Path, st: &Arc<Mutex<State>>, ctx: Option<&eframe::egui::Context>) -> Result<(), String> {
    let parent = dest.parent().ok_or("bad folder")?;
    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let file = parent.join(format!("Audiveris-{VERSION}-{}", asset()));
    let u = url();
    let mut resp = ureq::get(&u).call().map_err(|e| format!("Couldn't download the reader: {e}"))?;
    let total: u64 = resp.headers().get("content-length").and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).unwrap_or(0);
    {
        let mut out = std::fs::File::create(&file).map_err(|e| e.to_string())?;
        let mut rd = resp.body_mut().with_config().limit(500 << 20).reader();
        let mut buf = vec![0u8; 1 << 16];
        let mut got = 0u64;
        let mut last = std::time::Instant::now();
        loop {
            let n = rd.read(&mut buf).map_err(|e| format!("Download cut off: {e}"))?;
            if n == 0 {
                break;
            }
            std::io::Write::write_all(&mut out, &buf[..n]).map_err(|e| e.to_string())?;
            got += n as u64;
            if last.elapsed().as_millis() > 150 {
                *st.lock().unwrap() = State::Downloading(got, total);
                if let Some(c) = ctx {
                    c.request_repaint();
                }
                last = std::time::Instant::now();
            }
        }
        if total > 0 && got != total {
            return Err(format!("Download cut off ({got} of {total} bytes). Try again."));
        }
    }
    *st.lock().unwrap() = State::Unpacking;
    if let Some(c) = ctx {
        c.request_repaint();
    }
    let r = install_from(&file, dest);
    let _ = std::fs::remove_file(&file);
    r
}

/// Unpack a downloaded installer into `dest` (replacing what's there).
pub fn install_from(file: &Path, dest: &Path) -> Result<(), String> {
    let parent = dest.parent().ok_or("bad folder")?;
    // unpack next to the destination, then swap in, so a failure leaves any old copy working
    let tmp = parent.join("audiveris.unpacking");
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    if let Err(e) = unpack(file, &tmp) {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err(e);
    }
    if installed_in(&tmp).is_none() {
        let _ = std::fs::remove_dir_all(&tmp);
        return Err("The download unpacked, but the Audiveris program wasn't in it.".into());
    }
    let _ = std::fs::remove_dir_all(dest);
    std::fs::rename(&tmp, dest).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn unpack(file: &Path, to: &Path) -> Result<(), String> {
    // a .deb is an ar archive; the files are in data.tar.(zst|xz|gz) under ./opt/audiveris/
    let bytes = std::fs::read(file).map_err(|e| e.to_string())?;
    let (name, data) = ar_member(&bytes, "data.tar").ok_or("That .deb has no data in it")?;
    let rd: Box<dyn Read> = if name.ends_with(".zst") {
        Box::new(ruzstd::StreamingDecoder::new(data).map_err(|e| e.to_string())?)
    } else if name.ends_with(".gz") {
        Box::new(flate2::read::GzDecoder::new(data))
    } else {
        return Err(format!("Can't unpack {name}"));
    };
    let mut ar = tar::Archive::new(rd);
    ar.set_preserve_permissions(true);
    for e in ar.entries().map_err(|e| e.to_string())? {
        let mut e = e.map_err(|e| e.to_string())?;
        let path = e.path().map_err(|e| e.to_string())?.into_owned();
        let s = path.to_string_lossy().trim_start_matches("./").to_string();
        let Some(rest) = s.strip_prefix("opt/audiveris/") else { continue };
        if rest.is_empty() || rest.contains("..") {
            continue;
        }
        let out = to.join(rest);
        if let Some(p) = out.parent() {
            std::fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        e.unpack(&out).map_err(|e| format!("{rest}: {e}"))?;
    }
    Ok(())
}

/// Find the first member of an ar archive whose name starts with `prefix`.
#[cfg_attr(not(all(unix, not(target_os = "macos"))), allow(dead_code))]
fn ar_member<'a>(b: &'a [u8], prefix: &str) -> Option<(String, &'a [u8])> {
    if !b.starts_with(b"!<arch>\n") {
        return None;
    }
    let mut i = 8;
    while i + 60 <= b.len() {
        let h = &b[i..i + 60];
        let name = String::from_utf8_lossy(&h[0..16]).trim().trim_end_matches('/').to_string();
        let size: usize = String::from_utf8_lossy(&h[48..58]).trim().parse().ok()?;
        let start = i + 60;
        let end = start.checked_add(size)?;
        if end > b.len() {
            return None;
        }
        if name.starts_with(prefix) {
            return Some((name, &b[start..end]));
        }
        i = end + (size & 1);
    }
    None
}

#[cfg(windows)]
fn unpack(file: &Path, to: &Path) -> Result<(), String> {
    // an "administrative install" just extracts the .msi's files: no admin rights, no registry
    use std::os::windows::process::CommandExt;
    // msiexec wants PROPERTY="value with spaces" exactly like that
    let st = std::process::Command::new("msiexec")
        .raw_arg(format!("/a \"{}\" /qn TARGETDIR=\"{}\"", file.display(), to.display()))
        .status()
        .map_err(|e| format!("msiexec: {e}"))?;
    if !st.success() {
        return Err(format!("msiexec couldn't unpack the reader ({st})"));
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn unpack(file: &Path, to: &Path) -> Result<(), String> {
    let mnt = to.with_extension("mnt");
    let _ = std::fs::create_dir_all(&mnt);
    // some disk images show a licence first: answer it ("Y") on stdin
    let mut child = std::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-readonly", "-noverify", "-noautoopen", "-mountpoint"])
        .arg(&mnt)
        .arg(file)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("hdiutil: {e}"))?;
    if let Some(mut i) = child.stdin.take() {
        let _ = std::io::Write::write_all(&mut i, b"Y\n");
    }
    let out = child.wait_with_output().map_err(|e| format!("hdiutil: {e}"))?;
    if !out.status.success() {
        let why = String::from_utf8_lossy(&out.stderr).trim().to_string();
        return Err(format!("Couldn't open the Audiveris disk image: {why}"));
    }
    let app = std::fs::read_dir(&mnt).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.extension().map(|x| x == "app").unwrap_or(false)));
    let r = match app {
        Some(app) => {
            let st = std::process::Command::new("ditto").arg(&app).arg(to.join("Audiveris.app")).status().map_err(|e| e.to_string());
            match st {
                Ok(s) if s.success() => Ok(()),
                _ => Err("Couldn't copy Audiveris out of its disk image".to_string()),
            }
        }
        None => Err("No app in the Audiveris disk image".to_string()),
    };
    let _ = std::process::Command::new("hdiutil").arg("detach").arg(&mnt).arg("-quiet").status();
    let _ = std::fs::remove_dir(&mnt);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ar_reads_members() {
        let mut a = b"!<arch>\n".to_vec();
        for (n, d) in [("debian-binary", &b"2.0\n"[..]), ("data.tar.zst", &b"abc"[..])] {
            a.extend(format!("{:<16}{:<12}{:<6}{:<6}{:<8}{:<10}`\n", n, 0, 0, 0, 100644, d.len()).as_bytes());
            a.extend(d);
            if d.len() % 2 == 1 {
                a.push(b'\n');
            }
        }
        let (n, d) = ar_member(&a, "data.tar").unwrap();
        assert_eq!(n, "data.tar.zst");
        assert_eq!(d, b"abc");
    }

    /// Downloads ~70 MB: `cargo test --release -- --ignored omr_fetch_live`
    #[test]
    #[ignore]
    fn omr_fetch_live() {
        let d = std::env::temp_dir().join("noodle-omr-test/audiveris");
        let _ = std::fs::remove_dir_all(&d);
        if let Ok(f) = std::env::var("NOODLE_OMR_FILE") {
            // an installer already on disk (sandboxes without a clean TLS path)
            std::fs::create_dir_all(d.parent().unwrap()).unwrap();
            install_from(Path::new(&f), &d).expect("unpack");
        } else {
            let st = Arc::new(Mutex::new(State::Idle));
            fetch_into(&d, &st, None).expect("fetch");
        }
        let exe = installed_in(&d).expect("exe");
        println!("installed: {}", exe.display());
        // it starts and answers
        let out = std::process::Command::new(&exe).args(["-batch", "-help"]).output().expect("run");
        let txt = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
        println!("{}", &txt[..txt.len().min(400)]);
        // the Windows launcher has no console, so it may print nothing; it must still run
        assert!(cfg!(windows) || txt.to_lowercase().contains("audiveris") || txt.contains("-export"), "no answer from Audiveris");
    }
}
