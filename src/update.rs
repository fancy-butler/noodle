// Noodle — keeps itself up to date: checks GitHub's latest release (at most once a day) and,
// when asked, installs it.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Linux: downloads the .tar.gz and runs its install.sh (same as installing by hand).
// Windows: swaps noodle.exe for the new one (the old one is renamed and cleaned up next start).
// macOS: opens the download page (an app bundle is best replaced by dragging the new one in).

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const API: &str = "https://api.github.com/repos/fancy-butler/noodle/releases/latest";
pub const PAGE: &str = "https://github.com/fancy-butler/noodle/releases/latest";

#[derive(Clone, Debug)]
pub struct Release {
    pub version: String,
    pub notes: String,
    pub asset: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub enum State {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Installing(String),
    /// installed: restart to use it
    Done(String),
    Failed(String),
}

#[derive(Clone, Default)]
pub struct Updater {
    pub state: Arc<Mutex<State>>,
}

/// "v0.17.1" / "0.17.1" → (0, 17, 1)
pub fn parse_version(s: &str) -> Option<(u32, u32, u32)> {
    let s = s.trim().trim_start_matches(['v', 'V']);
    let mut it = s.split(['.', '-', '+']).map(|p| p.parse::<u32>().ok());
    Some((it.next()??, it.next().flatten().unwrap_or(0), it.next().flatten().unwrap_or(0)))
}

pub fn newer(remote: &str, local: &str) -> bool {
    match (parse_version(remote), parse_version(local)) {
        (Some(r), Some(l)) => r > l,
        _ => false,
    }
}

fn asset_name() -> &'static str {
    if cfg!(windows) {
        "noodle-windows-x86_64.zip"
    } else if cfg!(target_os = "macos") {
        "noodle-macos-universal.dmg"
    } else {
        "noodle-linux-x86_64.tar.gz"
    }
}

impl Updater {
    pub fn get(&self) -> State {
        self.state.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn set(&self, s: State) {
        if let Ok(mut st) = self.state.lock() {
            *st = s;
        }
    }

    pub fn check(&self, ctx: eframe::egui::Context) {
        if matches!(self.get(), State::Checking | State::Installing(_)) {
            return;
        }
        self.set(State::Checking);
        let me = self.clone();
        std::thread::spawn(move || {
            let r = (|| -> Result<Option<Release>, String> {
                let mut resp = ureq::get(API).header("User-Agent", "Noodle").header("Accept", "application/vnd.github+json").call().map_err(|e| e.to_string())?;
                let body = resp.body_mut().read_to_string().map_err(|e| e.to_string())?;
                let v: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
                let tag = v["tag_name"].as_str().unwrap_or("").to_string();
                if !newer(&tag, crate::VERSION) {
                    return Ok(None);
                }
                let asset = v["assets"].as_array().and_then(|a| a.iter().find(|x| x["name"].as_str() == Some(asset_name()))).and_then(|x| x["browser_download_url"].as_str()).map(|s| s.to_string());
                Ok(Some(Release { version: tag.trim_start_matches('v').to_string(), notes: v["body"].as_str().unwrap_or("").to_string(), asset }))
            })();
            me.set(match r {
                Ok(Some(rel)) => State::Available(rel),
                Ok(None) => State::UpToDate,
                Err(e) => State::Failed(format!("Couldn't check for updates: {e}")),
            });
            ctx.request_repaint();
        });
    }

    pub fn install(&self, rel: Release, ctx: eframe::egui::Context) {
        let Some(url) = rel.asset.clone() else {
            open_page();
            return;
        };
        if cfg!(target_os = "macos") {
            open_page();
            return;
        }
        self.set(State::Installing(format!("Downloading Noodle {}…", rel.version)));
        let me = self.clone();
        std::thread::spawn(move || {
            let r = (|| -> Result<(), String> {
                let mut resp = ureq::get(&url).header("User-Agent", "Noodle").call().map_err(|e| e.to_string())?;
                let bytes = resp.body_mut().with_config().limit(200 * 1024 * 1024).read_to_vec().map_err(|e| e.to_string())?;
                me.set(State::Installing(format!("Installing Noodle {}…", rel.version)));
                let work = crate::sounds::user_dir().join("cache").join("update");
                let _ = std::fs::remove_dir_all(&work);
                std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
                if cfg!(windows) {
                    install_windows(&bytes, &work)
                } else {
                    install_linux(&bytes, &work)
                }
            })();
            me.set(match r {
                Ok(()) => State::Done(rel.version.clone()),
                Err(e) => State::Failed(format!("Update didn't install: {e}")),
            });
            ctx.request_repaint();
        });
    }
}

pub fn open_page() {
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener).arg(PAGE).spawn();
}

fn install_linux(bytes: &[u8], work: &std::path::Path) -> Result<(), String> {
    let gz = flate2::read::GzDecoder::new(bytes);
    tar::Archive::new(gz).unpack(work).map_err(|e| format!("couldn't unpack: {e}"))?;
    let script = work.join("noodle").join("install.sh");
    if !script.exists() {
        return Err("the download has no install.sh".into());
    }
    let out = std::process::Command::new("bash").arg(&script).current_dir(work.join("noodle")).output().map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).lines().last().unwrap_or("install.sh failed").to_string());
    }
    Ok(())
}

fn install_windows(bytes: &[u8], work: &std::path::Path) -> Result<(), String> {
    use std::io::Read;
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let mut exe = Vec::new();
    {
        let idx = (0..z.len()).find(|&i| z.by_index(i).map(|f| f.name().to_lowercase().ends_with("noodle.exe")).unwrap_or(false)).ok_or("no noodle.exe in the download")?;
        z.by_index(idx).map_err(|e| e.to_string())?.read_to_end(&mut exe).map_err(|e| e.to_string())?;
    }
    let cur = std::env::current_exe().map_err(|e| e.to_string())?;
    let new = work.join("noodle.exe");
    std::fs::write(&new, &exe).map_err(|e| e.to_string())?;
    // a running .exe can be renamed but not overwritten
    let old = cur.with_extension("old.exe");
    let _ = std::fs::remove_file(&old);
    std::fs::rename(&cur, &old).map_err(|e| e.to_string())?;
    if let Err(e) = std::fs::copy(&new, &cur) {
        let _ = std::fs::rename(&old, &cur);
        return Err(e.to_string());
    }
    Ok(())
}

/// Start the freshly installed Noodle and quit this one.
pub fn restart() {
    let exe: PathBuf = if cfg!(windows) {
        std::env::current_exe().unwrap_or_else(|_| PathBuf::from("noodle.exe"))
    } else {
        let home = std::env::var("HOME").unwrap_or_default();
        let p = PathBuf::from(home).join(".local/bin/noodle");
        if p.exists() {
            p
        } else {
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("noodle"))
        }
    };
    if std::process::Command::new(exe).spawn().is_ok() {
        std::process::exit(0);
    }
}

/// Windows: remove the previous noodle.old.exe left by an update.
pub fn cleanup() {
    if let Ok(cur) = std::env::current_exe() {
        let _ = std::fs::remove_file(cur.with_extension("old.exe"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(newer("v0.17.2", "0.17.1"));
        assert!(newer("v1.0.0", "0.99.9"));
        assert!(!newer("v0.17.1", "0.17.1"));
        assert!(!newer("v0.9.0", "0.17.1"), "numbers, not text");
        assert!(!newer("nonsense", "0.17.1"));
    }
}

#[cfg(test)]
mod live {
    /// HOME=/tmp/x cargo test --release update::live -- --ignored --nocapture
    #[test]
    #[ignore]
    fn fetch_and_install_into_home() {
        if let Ok(f) = std::env::var("NOODLE_UPDATE_TGZ") {
            // offline: install a downloaded release tarball
            let bytes = std::fs::read(f).unwrap();
            let work = std::env::temp_dir().join("noodle-update-test");
            let _ = std::fs::remove_dir_all(&work);
            std::fs::create_dir_all(&work).unwrap();
            super::install_linux(&bytes, &work).unwrap();
            let home = std::env::var("HOME").unwrap();
            assert!(std::path::Path::new(&home).join(".local/bin/noodle").exists());
            return;
        }
        let mut resp = ureq::get(super::API).header("User-Agent", "Noodle").call().unwrap();
        let body = resp.body_mut().read_to_string().unwrap();
        let v: serde_json::Value = serde_json::from_str(&body).unwrap();
        let tag = v["tag_name"].as_str().unwrap();
        let url = v["assets"].as_array().unwrap().iter().find(|x| x["name"].as_str() == Some(super::asset_name())).unwrap()["browser_download_url"].as_str().unwrap().to_string();
        println!("latest {tag}: {url}");
        let mut r = ureq::get(&url).header("User-Agent", "Noodle").call().unwrap();
        let bytes = r.body_mut().with_config().limit(200 * 1024 * 1024).read_to_vec().unwrap();
        println!("{} bytes", bytes.len());
        let work = std::env::temp_dir().join("noodle-update-test");
        let _ = std::fs::remove_dir_all(&work);
        std::fs::create_dir_all(&work).unwrap();
        super::install_linux(&bytes, &work).unwrap();
        let home = std::env::var("HOME").unwrap();
        assert!(std::path::Path::new(&home).join(".local/bin/noodle").exists());
    }
}
