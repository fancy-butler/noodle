// Noodle — MIDI input with hot-plug: every keyboard that shows up gets connected.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use crate::engine::Cmd;
use crossbeam_channel::Sender;
use midir::{Ignore, MidiInput, MidiInputConnection};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Duration;

fn skip(name: &str) -> bool {
    let n = name.to_lowercase();
    n.contains("midi through") || n.contains("noodle") || n.contains("rtmidi")
}

/// Strip ALSA's "client:port" noise for display.
pub fn pretty(name: &str) -> String {
    let base = name.split(':').next().unwrap_or(name).trim();
    if base.is_empty() {
        name.to_string()
    } else {
        base.to_string()
    }
}

/// `blocked`: pretty names of inputs the user switched off.
/// The same message arriving from two ports within a few ms is one key press
/// seen twice (a device with several ports, or PipeWire's MIDI bridge mirroring it).
struct Dedupe {
    last: [([u8; 3], usize, std::time::Instant); 16],
    i: usize,
}

impl Dedupe {
    fn is_duplicate(&mut self, msg: [u8; 3], port: usize) -> bool {
        let now = std::time::Instant::now();
        let dup = self.last.iter().any(|(m, p, t)| *m == msg && *p != port && now.duration_since(*t) < Duration::from_millis(12));
        self.last[self.i] = (msg, port, now);
        self.i = (self.i + 1) % self.last.len();
        dup
    }
}

/// PipeWire's ALSA MIDI bridge re-publishes hardware ports; skip the copy when the real port is there.
fn bridge_copy(name: &str, all: &[String]) -> bool {
    let n = name.to_lowercase();
    if !n.starts_with("midi-bridge") {
        return false;
    }
    // "Midi-Bridge:Minilab3 Minilab3 MIDI (capture)" → is there a "Minilab3…" port too?
    let rest = n.split(':').nth(1).unwrap_or("");
    all.iter().any(|o| {
        let o = o.to_lowercase();
        !o.starts_with("midi-bridge") && !o.is_empty() && rest.contains(o.split(':').next().unwrap_or("~").trim())
    })
}

pub fn spawn(tx: Sender<Cmd>, devices: Arc<Mutex<Vec<String>>>, blocked: Arc<Mutex<Vec<String>>>) {
    let dedupe = Arc::new(Mutex::new(Dedupe { last: [([0; 3], usize::MAX, std::time::Instant::now()); 16], i: 0 }));
    let mut next_port = 0usize;
    std::thread::Builder::new()
        .name("noodle-midi".into())
        .spawn(move || {
            let mut conns: HashMap<String, MidiInputConnection<()>> = HashMap::new();
            loop {
                if let Ok(scan) = MidiInput::new("Noodle scan") {
                    let mut found = Vec::new();
                    for p in scan.ports() {
                        if let Ok(n) = scan.port_name(&p) {
                            if !skip(&n) {
                                found.push((n, p));
                            }
                        }
                    }
                    drop(scan);
                    let names: Vec<String> = found.iter().map(|(n, _)| n.clone()).collect();
                    found.retain(|(n, _)| !bridge_copy(n, &names));
                    let off = blocked.lock().map(|b| b.clone()).unwrap_or_default();
                    let mut all: Vec<String> = found.iter().map(|(n, _)| pretty(n)).collect();
                    all.sort();
                    all.dedup();
                    found.retain(|(n, _)| !off.contains(&pretty(n)));
                    conns.retain(|n, _| found.iter().any(|(m, _)| m == n));
                    for (name, port) in found {
                        if conns.contains_key(&name) {
                            continue;
                        }
                        let Ok(mut mi) = MidiInput::new("Noodle") else { continue };
                        mi.ignore(Ignore::All);
                        let tx2 = tx.clone();
                        let dd = dedupe.clone();
                        let port_id = next_port;
                        next_port += 1;
                        let res = mi.connect(
                            &port,
                            "noodle-in",
                            move |_, msg, _| {
                                if msg.is_empty() || msg[0] < 0x80 || msg[0] >= 0xF0 {
                                    return;
                                }
                                let mut b = [0u8; 3];
                                for (i, v) in msg.iter().take(3).enumerate() {
                                    b[i] = *v;
                                }
                                if dd.lock().map(|mut d| d.is_duplicate(b, port_id)).unwrap_or(false) {
                                    return;
                                }
                                let _ = tx2.try_send(Cmd::Midi(b));
                            },
                            (),
                        );
                        match res {
                            Ok(c) => {
                                eprintln!("noodle: MIDI in ← {}", pretty(&name));
                                conns.insert(name, c);
                            }
                            Err(e) => eprintln!("noodle: couldn't open MIDI input {}: {e}", pretty(&name)),
                        }
                    }
                    // report every input; switched-off ones carry a leading '-'
                    let names: Vec<String> = all.into_iter().map(|n| if off.contains(&n) { format!("-{n}") } else { n }).collect();
                    if let Ok(mut d) = devices.lock() {
                        *d = names;
                    }
                }
                std::thread::sleep(Duration::from_millis(1500));
            }
        })
        .expect("spawn midi thread");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn duplicates_and_bridge() {
        let mut d = Dedupe { last: [([0; 3], usize::MAX, std::time::Instant::now()); 16], i: 0 };
        assert!(!d.is_duplicate([0x90, 60, 100], 0));
        assert!(d.is_duplicate([0x90, 60, 100], 1), "same note from a second port");
        // a fast real repeat on the same port is kept
        let mut d2 = Dedupe { last: [([0; 3], usize::MAX, std::time::Instant::now()); 16], i: 0 };
        assert!(!d2.is_duplicate([0x90, 62, 90], 0));
        assert!(!d2.is_duplicate([0x90, 62, 90], 0), "same port: a real repeat");
        std::thread::sleep(Duration::from_millis(15));
        assert!(!d.is_duplicate([0x90, 60, 100], 1), "a later press from the other port is real");
        let all = vec!["Minilab3:Minilab3 Minilab3 MIDI 24:0".to_string(), "Midi-Bridge:Minilab3 Minilab3 MIDI (capture) 14:1".to_string()];
        assert!(bridge_copy(&all[1], &all));
        assert!(!bridge_copy(&all[0], &all));
    }
}
