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
pub fn spawn(tx: Sender<Cmd>, devices: Arc<Mutex<Vec<String>>>, blocked: Arc<Mutex<Vec<String>>>) {
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
