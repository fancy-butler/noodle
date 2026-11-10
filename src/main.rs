// Noodle — plug in a MIDI keyboard and play. Instantly.
// Copyright (C) 2026 Fancy Butler
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. See LICENSE.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod audio;
mod drums;
mod engine;
mod midi;
mod settings;
mod song;
mod sounds;
mod theory;
mod ui;

use engine::{Cmd, Engine, Params, UiEvt};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--render-test") {
        let out = args.get(i + 1).cloned().unwrap_or_else(|| "noodle-test.wav".into());
        let sf = args.get(i + 2).cloned();
        if let Err(e) = render_test(&out, sf) {
            eprintln!("render test failed: {e}");
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("noodle {VERSION}");
        return;
    }

    let p = Arc::new(Params::new());
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, urx) = crossbeam_channel::bounded::<UiEvt>(16384);
    let devices = Arc::new(Mutex::new(Vec::new()));
    midi::spawn(tx.clone(), devices.clone());

    let rate = audio::default_rate();
    p.sample_rate.store(rate, Relaxed);
    let engine = Engine::new(p.clone(), rx, utx, rate as f32, 8192);
    let (audio, sample_rate, audio_info) = match audio::start(engine) {
        Ok(a) => {
            let info = Ok(format!("{} · {} kHz", a.device, a.sample_rate / 1000));
            let sr = a.sample_rate;
            (Some(a), sr, info)
        }
        Err((e, engine)) => {
            eprintln!("noodle: no audio output ({e}); running silently");
            if let Some(engine) = engine {
                audio::run_silent(engine, rate);
            }
            (None, rate, Err(format!("No audio: {e}")))
        }
    };

    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(format!("Noodle {VERSION}"))
            .with_app_id("noodle")
            .with_inner_size([1420.0, 900.0])
            .with_min_inner_size([980.0, 640.0]),
        ..Default::default()
    };
    let handles = ui::Handles { p, tx, rx: urx, devices, sample_rate, audio_info };
    let res = eframe::run_native("Noodle", options, Box::new(move |cc| Ok(Box::new(ui::App::new(cc, handles)))));
    drop(audio);
    if let Err(e) = res {
        eprintln!("noodle: {e}");
        std::process::exit(1);
    }
}

/// Offline smoke test: drives the real engine with scripted input and writes a WAV,
/// printing the level of each section so silence or clipping is obvious.
fn render_test(out: &str, sf: Option<String>) -> Result<(), String> {
    let sr = 48000u32;
    let path = match sf {
        Some(s) => std::path::PathBuf::from(s),
        None => sounds::find_soundfonts().into_iter().next().ok_or("no .sf2 found")?,
    };
    println!("soundfont: {}", path.display());
    let t = std::time::Instant::now();
    let loaded = sounds::load_soundfont(&path, sr)?;
    println!("loaded {} presets in {:.2}s", loaded.presets.len(), t.elapsed().as_secs_f32());
    let p = Arc::new(Params::new());
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, urx) = crossbeam_channel::bounded::<UiEvt>(1 << 20);
    let mut eng = Engine::new(p.clone(), rx, utx, sr as f32, 8192);
    tx.send(Cmd::Synths(loaded.synths)).unwrap();

    let mut wav = hound::WavWriter::create(out, hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int })
        .map_err(|e| e.to_string())?;
    let block = 256usize;
    let mut l = vec![0.0f32; block];
    let mut r = vec![0.0f32; block];
    let mut render = |secs: f32, label: &str, eng: &mut Engine| {
        let n = (secs * sr as f32) as usize / block;
        let (mut peak, mut sum, mut cnt) = (0.0f32, 0.0f64, 0usize);
        let t = std::time::Instant::now();
        for _ in 0..n {
            eng.process(&mut l, &mut r);
            for i in 0..block {
                peak = peak.max(l[i].abs()).max(r[i].abs());
                sum += (l[i] * l[i]) as f64;
                cnt += 1;
                wav.write_sample((l[i] * 32767.0) as i16).unwrap();
                wav.write_sample((r[i] * 32767.0) as i16).unwrap();
            }
        }
        let rt = secs / t.elapsed().as_secs_f32();
        println!("{label:<34} peak {:>5.3}  rms {:>6.4}  ({:.0}x realtime)", peak, (sum / cnt.max(1) as f64).sqrt(), rt);
        (peak, (sum / cnt.max(1) as f64).sqrt())
    };
    let note = |k: u8, on: bool| Cmd::Midi(if on { [0x90, k, 100] } else { [0x80, k, 0] });
    let mut fails = vec![];
    fn check_(fails: &mut Vec<String>, name: &str, (pk, rms): (f32, f64), want_sound: bool) {
        if want_sound && rms < 0.002 {
            fails.push(format!("{name}: silent"));
        }
        if !want_sound && rms > 0.01 {
            fails.push(format!("{name}: should be quiet"));
        }
        if pk >= 0.999 {
            fails.push(format!("{name}: clipping"));
        }
    }
    macro_rules! check {
        ($n:expr, $r:expr, $w:expr) => {{
            let r = $r;
            check_(&mut fails, $n, r, $w)
        }};
    }

    check!("silence", render(0.3, "silence before playing", &mut eng), false);
    for k in [60, 64, 67] {
        tx.send(note(k, true)).unwrap();
    }
    check!("chord", render(1.0, "C major chord, piano", &mut eng), true);
    for k in [60, 64, 67] {
        tx.send(note(k, false)).unwrap();
    }
    render(1.0, "release tail", &mut eng);

    tx.send(Cmd::Program { layer: false, bank: 0, prog: 81 }).unwrap(); // saw lead
    p.arp_on.store(true, Relaxed);
    for k in [57, 60, 64] {
        tx.send(note(k, true)).unwrap();
    }
    check!("arp", render(2.0, "arp on Am, saw lead", &mut eng), true);
    for k in [57, 60, 64] {
        tx.send(note(k, false)).unwrap();
    }
    p.arp_on.store(false, Relaxed);
    render(0.5, "arp released", &mut eng);

    for (i, v) in drums::PATTERNS[1].1.iter().enumerate() {
        p.drum_rows[i].store(*v, Relaxed);
    }
    p.drums_on.store(true, Relaxed);
    check!("drums", render(2.2, "drums: synthpop pattern", &mut eng), true);
    let steps_hit = p.drum_step.load(Relaxed);
    println!("  drum step pointer now at {steps_hit}");

    p.delay_mix.set(0.4);
    p.cutoff.set(0.45);
    p.resonance.set(0.6);
    tx.send(Cmd::Program { layer: false, bank: 0, prog: 0 }).unwrap();
    tx.send(Cmd::LoopButton).unwrap(); // start recording
    for k in [48u8, 55, 52, 55] {
        tx.send(note(k, true)).unwrap();
        render(0.5, "  loop rec note", &mut eng);
        tx.send(note(k, false)).unwrap();
    }
    tx.send(Cmd::LoopButton).unwrap(); // close
    p.drums_on.store(false, Relaxed);
    let lp = render(2.4, "looper playback (+ delay, filter)", &mut eng);
    check!("looper", lp, true);
    println!("  loop state {} len {} beats, layers {}", p.loop_state.load(Relaxed), p.loop_beats.get(), p.loop_passes.load(Relaxed));
    if p.loop_beats.get() != 4.0 {
        fails.push(format!("loop length {} beats, expected 4", p.loop_beats.get()));
    }
    tx.send(Cmd::LoopStop).unwrap();
    p.delay_mix.set(0.0);
    p.cutoff.set(1.0);
    render(1.0, "loop stopped (tail)", &mut eng);

    // song playback through the second synth
    let up: Vec<u8> = theory::SCALES[5].steps.iter().map(|s| 62 + s).chain(std::iter::once(74)).collect();
    let s = song::scale_song(&up, 160.0, 0, 71);
    let dur = s.dur;
    tx.send(Cmd::LoadSong(Some(Arc::new(s)))).unwrap();
    tx.send(Cmd::SongPlay(true)).unwrap();
    check!("song", render(dur as f32 * 0.5, "song: D freygish on clarinet", &mut eng), true);
    render(dur as f32 * 0.5 + 0.5, "song: rest", &mut eng);
    println!("  song playing={} pos={:.2}/{:.2}", p.song_playing.load(Relaxed), p.song_pos.get(), dur);
    if p.song_playing.load(Relaxed) {
        fails.push("song never finished".into());
    }

    // wait mode: song must pause on the user's notes
    let s = song::scale_song(&up, 160.0, 0, 0);
    let first = up[0];
    tx.send(Cmd::LoadSong(Some(Arc::new(s)))).unwrap();
    p.song_user.store(1, Relaxed);
    p.song_wait.store(true, Relaxed);
    tx.send(Cmd::SongPlay(true)).unwrap();
    render(0.4, "wait mode (should pause)", &mut eng);
    let waiting = p.song_waiting.load(Relaxed);
    let pos_before = p.song_pos.get();
    tx.send(note(first, true)).unwrap();
    render(0.1, "  user plays first note", &mut eng);
    tx.send(note(first, false)).unwrap();
    render(0.3, "  continues", &mut eng);
    println!("  waited={waiting} at {:.2}s, after playing pos={:.2}s", pos_before, p.song_pos.get());
    if !waiting || p.song_pos.get() <= pos_before {
        fails.push("wait mode didn't pause/resume".into());
    }

    // UI events made it out
    let mut notes = 0;
    while let Ok(e) = urx.try_recv() {
        if let UiEvt::Note { .. } = e {
            notes += 1;
        }
    }
    println!("visual note events: {notes}");
    wav.finalize().map_err(|e| e.to_string())?;
    if fails.is_empty() {
        println!("ALL GOOD → {out}");
        Ok(())
    } else {
        Err(fails.join("; "))
    }
}
