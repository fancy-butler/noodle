// Noodle — plug in a MIDI keyboard and play. Instantly.
// Copyright (C) 2026 Fancy Butler
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. See LICENSE.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod abc;
mod accordion;
mod analog;
mod chip;
mod collections;
mod controllers;
mod audio;
mod drums;
mod game;
mod drone_rec;
mod engine;
mod fingering;
mod midi;
mod native;
mod packs;
mod omr;
mod pipe_art;
mod pipe_pack;
mod rec_build;
mod pipes;
mod reed;
mod sampler;
mod score;
mod session;
mod sets;
mod settings;
mod song;
mod sounds;
mod theory;
mod ui;
mod update;

use engine::{Cmd, Engine, Params, UiEvt};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if let Some(i) = args.iter().position(|a| a == "--tune-demo") {
        // --tune-demo out.wav events.txt: an event list (from a script) on the uilleann pipes
        tune_demo(&args[i + 1], &args[i + 2]);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--regs-demo") {
        let out = args.get(i + 1).cloned().unwrap_or("regs.wav".into());
        regs_demo(&out);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--pipe-demo") {
        let out = args.get(i + 1).cloned().unwrap_or("pipes.wav".into());
        pipe_demo(&out);
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--render-test") {
        let out = args.get(i + 1).cloned().unwrap_or_else(|| "noodle-test.wav".into());
        let sf = args.get(i + 2).cloned();
        if let Err(e) = render_test(&out, sf) {
            eprintln!("render test failed: {e}");
            std::process::exit(1);
        }
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--level-survey") {
        if let Err(e) = level_survey(&args[i + 1..]) {
            eprintln!("level survey failed: {e}");
            std::process::exit(1);
        }
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("noodle {VERSION}");
        return;
    }

    update::cleanup();
    let p = Arc::new(Params::new());
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, urx) = crossbeam_channel::bounded::<UiEvt>(16384);
    let devices = Arc::new(Mutex::new(Vec::new()));
    let blocked = Arc::new(Mutex::new(settings::Settings::load().midi_off));
    midi::spawn(tx.clone(), devices.clone(), blocked.clone());

    let rate = audio::default_rate();
    p.sample_rate.store(rate, Relaxed);
    let mut eng = Engine::new(p.clone(), rx, utx, rate as f32, 8192);
    let (mtx, mrx) = crossbeam_channel::bounded::<[u8; 3]>(1024);
    eng.set_midi_out(mtx);
    let midi_out_status = Arc::new(Mutex::new(String::new()));
    midi::spawn_out(mrx, midi_out_status.clone());
    let engine = Arc::new(Mutex::new(eng));
    let latency = settings::Settings::load().latency;
    let (audio, audio_info) = match audio::start(engine.clone(), p.clone(), latency) {
        Ok(a) => {
            let info = Ok(format!("{} · {} kHz", a.device, a.sample_rate / 1000));
            (Some(a), info)
        }
        Err(e) => {
            eprintln!("noodle: no audio output ({e}); running silently");
            (None, Err(format!("No audio: {e}")))
        }
    };
    let sample_rate = audio.as_ref().map(|a| a.sample_rate).unwrap_or(p.sample_rate.load(Relaxed));
    audio::spawn_watchdog(engine.clone(), p.clone(), sample_rate);

    let mut vp = eframe::egui::ViewportBuilder::default().with_title(format!("Noodle {VERSION}"));
    // Wayland needs the app id to match noodle.desktop. On X11, egui would turn the app id
    // into WM_CLASS ("", "noodle") with an empty instance, and docks like Cairo-Dock then
    // can't match the window to its launcher; leaving it unset gives ("noodle", "noodle").
    if std::env::var("WAYLAND_DISPLAY").map(|v| !v.is_empty()).unwrap_or(false) || !cfg!(target_os = "linux") {
        vp = vp.with_app_id("noodle");
    }
    let options = eframe::NativeOptions {
        viewport: vp
            .with_icon(std::sync::Arc::new(eframe::egui::IconData { rgba: include_bytes!("../assets/noodle-icon.rgba").to_vec(), width: 256, height: 256 }))
            .with_inner_size(if std::env::var("NOODLE_WIDE").is_ok() { [2500.0, 1250.0] } else { [1420.0, 900.0] })
            .with_min_inner_size([980.0, 640.0]),
        ..Default::default()
    };
    let handles = ui::Handles { p, tx, rx: urx, devices, blocked, sample_rate, audio_info, engine: engine.clone(), audio, midi_out_status };
    let res = eframe::run_native("Noodle", options, Box::new(move |cc| Ok(Box::new(ui::App::new(cc, handles)))));
    if let Err(e) = res {
        eprintln!("noodle: {e}");
        std::process::exit(1);
    }
}

/// How loud each kind of sound comes out for the same played note (C4-ish, velocity 100), relative to
/// the General MIDI piano. `noodle --level-survey bank.sf2 [instrument.sfz ...]`
fn level_survey(args: &[String]) -> Result<(), String> {
    let sr = 48000u32;
    let sf = args.first().ok_or("give a .sf2")?;
    let loaded = sounds::load_soundfont(std::path::Path::new(sf), sr)?;
    let p = Arc::new(Params::new());
    p.master.set(0.2); // stay under the limiter: we want the levels, not the limiter's work
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, _urx) = crossbeam_channel::bounded::<UiEvt>(1 << 20);
    let mut eng = Engine::new(p.clone(), rx, utx, sr as f32, 8192);
    tx.send(Cmd::Synths(loaded.synths)).unwrap();
    let block = 256usize;
    let (mut l, mut r) = (vec![0.0f32; block], vec![0.0f32; block]);
    let mut run = |eng: &mut Engine, secs: f32| -> (f32, f64) {
        let n = (secs * sr as f32) as usize / block;
        let (mut pk, mut sum, mut cnt) = (0f32, 0f64, 0usize);
        for _ in 0..n {
            eng.process(&mut l, &mut r);
            for i in 0..block {
                pk = pk.max(l[i].abs()).max(r[i].abs());
                sum += ((l[i] * l[i] + r[i] * r[i]) * 0.5) as f64;
                cnt += 1;
            }
        }
        (pk, (sum / cnt.max(1) as f64).sqrt())
    };
    let note = |k: u8, on: bool| Cmd::Midi(if on { [0x90, k, 100] } else { [0x80, k, 0] });
    let mut measure = |eng: &mut Engine, label: &str, prog: Cmd, keys: &[u8], reference: &mut Option<f64>| {
        tx.send(prog).unwrap();
        run(eng, 0.2);
        for &k in keys {
            tx.send(note(k, true)).unwrap();
        }
        run(eng, 0.1);
        let (pk, rms) = run(eng, 0.8);
        for &k in keys {
            tx.send(note(k, false)).unwrap();
        }
        tx.send(Cmd::Panic).unwrap();
        run(eng, 0.6);
        let refv = *reference.get_or_insert(rms);
        println!("{label:<40} rms {:>7.4}  peak {:>6.3}  {:>+6.1} dB vs piano", rms, pk, 20.0 * (rms / refv).log10());
    };
    let mut reference = None;
    let sf2 = |prog: i32| Cmd::Program { layer: false, bank: 0, prog, inst: None };
    for (prog, name) in [(0, "GM piano"), (21, "GM accordion"), (40, "GM violin"), (48, "GM strings"), (56, "GM trumpet"), (65, "GM alto sax"), (73, "GM flute"), (74, "GM recorder"), (78, "GM whistle"), (79, "GM ocarina"), (109, "GM bagpipe"), (81, "GM saw lead")] {
        measure(&mut eng, name, sf2(prog), &[60], &mut reference);
    }
    for pt in chip::PATCHES.iter().take(6) {
        measure(&mut eng, &format!("chip {}", pt.name), Cmd::Program { layer: false, bank: 0, prog: 0, inst: Some(Arc::new(native::InstDef::Chip(*pt))) }, &[60], &mut reference);
    }
    for pt in analog::BANKS.iter().flat_map(|b| b.iter()).take(8) {
        measure(&mut eng, &format!("synthwave {}", pt.name), Cmd::Program { layer: false, bank: 0, prog: 0, inst: Some(Arc::new(native::InstDef::Analog(*pt))) }, &[60], &mut reference);
    }
    for f in args.iter().skip(1) {
        let inst = sampler::load(std::path::Path::new(f))?;
        let key = inst.regions.iter().map(|r| (r.lokey as u16 + r.hikey as u16) / 2).nth(inst.regions.len() / 2).unwrap_or(60) as u8;
        measure(&mut eng, &format!("sample {}", inst.name), Cmd::Program { layer: false, bank: 0, prog: 0, inst: Some(Arc::new(native::InstDef::Sampler(Arc::new(inst)))) }, &[key], &mut reference);
    }
    // accordion mode (first instrument, its first register), synth and recorded reeds:
    // one treble button, then treble + a bass chord, bellows pushing
    {
        let insts = accordion::instruments();
        let i = &insts[0];
        let reeds = {
            let path = packs::installed_path(packs::reed_pack());
            path.exists().then(|| sampler::load(&path).ok()).flatten().map(|mut inst| {
                sampler::loop_sustains(&mut inst);
                Arc::new(inst)
            })
        };
        for (label, smp) in [("synth reeds", None), ("real reeds", reeds)] {
            if label == "real reeds" && smp.is_none() {
                println!("accordion real reeds: pack not installed, skipped");
                continue;
            }
            for (what, chord) in [("1 button", false), ("button + bass chord", true)] {
                tx.send(Cmd::Acc(engine::AccMsg::Setup(Some((i.registers[0], i.bass_ranks, i.tuning, smp.clone()))))).unwrap();
                tx.send(Cmd::Acc(engine::AccMsg::Bellows(true))).unwrap();
                run(&mut eng, 0.2);
                tx.send(Cmd::Acc(engine::AccMsg::Treble { id: 1, note: 72, on: true })).unwrap();
                if chord {
                    let notes = [48u8, 52, 55];
                    tx.send(Cmd::Acc(engine::AccMsg::Bass { id: 2, notes, n: 3, chord: true, on: true })).unwrap();
                }
                run(&mut eng, 0.1);
                let (pk, rms) = run(&mut eng, 0.8);
                tx.send(Cmd::Acc(engine::AccMsg::Setup(None))).unwrap();
                run(&mut eng, 0.6);
                println!("{:<40} rms {:>7.4}  peak {:>6.3}  {:>+6.1} dB vs piano", format!("accordion {label}, {what}"), rms, pk, 20.0 * (rms / reference.unwrap()).log10());
            }
            // every register of a diatonic and a chromatic, one button
            for inst in [&insts[0], insts.iter().find(|x| x.registers.len() >= 10).unwrap_or(&insts[0])] {
                for reg in inst.registers {
                    tx.send(Cmd::Acc(engine::AccMsg::Setup(Some((*reg, inst.bass_ranks, inst.tuning, smp.clone()))))).unwrap();
                    tx.send(Cmd::Acc(engine::AccMsg::Bellows(true))).unwrap();
                    run(&mut eng, 0.2);
                    tx.send(Cmd::Acc(engine::AccMsg::Treble { id: 1, note: 72, on: true })).unwrap();
                    run(&mut eng, 0.1);
                    let (_, rms) = run(&mut eng, 0.8);
                    tx.send(Cmd::Acc(engine::AccMsg::Setup(None))).unwrap();
                    run(&mut eng, 0.6);
                    println!("    {label}: {:<34} {:>+6.1} dB vs piano", reg.name, 20.0 * (rms / reference.unwrap()).log10());
                }
            }
        }
    }
    // the SoundFont drum kit: one kick + snare, peak against the piano note's peak
    tx.send(sf2(0)).unwrap();
    run(&mut eng, 0.2);
    p.drums_on.store(false, Relaxed);
    for (k, name) in [(36u8, "SF2 kick"), (38, "SF2 snare")] {
        tx.send(Cmd::Midi([0x99, k, 100])).unwrap();
        let (pk, rms) = run(&mut eng, 0.25);
        tx.send(Cmd::Midi([0x89, k, 0])).unwrap();
        run(&mut eng, 0.4);
        println!("{name:<40} rms {:>7.4}  peak {:>6.3}  {:>+6.1} dB vs piano (rms over 0.25 s)", rms, pk, 20.0 * (rms / reference.unwrap()).log10());
    }
    Ok(())
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
    let mut t_total = 0f32;
    let mut render = |secs: f32, label: &str, eng: &mut Engine| {
        let n = (secs * sr as f32) as usize / block;
        if std::env::var("NOODLE_TIMES").is_ok() {
            println!("@{t_total:.2}s");
        }
        t_total += (n * block) as f32 / sr as f32;
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

    tx.send(Cmd::Program { layer: false, bank: 0, prog: 81, inst: None }).unwrap(); // saw lead
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
        p.drum_bank[0][i].store(*v, Relaxed);
    }
    p.drum_cur.store(0, Relaxed);
    p.drums_on.store(true, Relaxed);
    check!("drums", render(2.2, "drums: synthpop pattern", &mut eng), true);
    let steps_hit = p.drum_step.load(Relaxed);
    println!("  drum step pointer now at {steps_hit}");
    // live-PA drums: queue slot B, a fill, then a chain A×1 → B×1 (fill)
    for (i, v) in drums::PATTERNS[2].1.iter().enumerate() {
        p.drum_bank[1][i].store(*v, Relaxed);
    }
    p.drum_queue.store(1, Relaxed);
    render(2.3, "  queue pattern B", &mut eng);
    println!("  playing slot {}", p.drum_cur.load(Relaxed));
    if p.drum_cur.load(Relaxed) != 1 {
        fails.push("queued drum pattern didn't switch at the bar".into());
    }
    p.drum_fill.store(true, Relaxed);
    let mut saw_fill = false;
    for _ in 0..30 {
        render(0.1, "", &mut eng);
        saw_fill |= p.drum_fill_now.load(Relaxed);
    }
    println!("  fill played: {saw_fill}");
    if !saw_fill {
        fails.push("drum fill never played".into());
    }
    p.chain[0].store(0 | (1 << 4), Relaxed);
    p.chain[1].store(1 | 8 | (1 << 4), Relaxed);
    p.chain_len.store(2, Relaxed);
    p.chain_on.store(true, Relaxed);
    p.drums_on.store(false, Relaxed);
    render(0.1, "", &mut eng);
    p.drums_on.store(true, Relaxed);
    let mut seen = [false; 2];
    for _ in 0..50 {
        render(0.1, "", &mut eng);
        seen[(p.drum_cur.load(Relaxed) as usize).min(1)] = true;
    }
    println!("  chain visited A {} B {}", seen[0], seen[1]);
    if !(seen[0] && seen[1]) {
        fails.push("drum chain didn't move through its parts".into());
    }
    p.chain_on.store(false, Relaxed);
    tx.send(Cmd::DrumInst(Some(std::sync::Arc::new(native::InstDef::ChipDrums(0))))).unwrap();
    check!("chip drums", render(1.5, "drums: NES kit", &mut eng), true);
    tx.send(Cmd::DrumInst(Some(std::sync::Arc::new(native::InstDef::ChipDrums(5))))).unwrap();
    let d80 = render(1.5, "drums: 80s gated kit", &mut eng);
    if d80.1 < 0.003 || d80.0 >= 0.999 {
        fails.push(format!("80s kit level (peak {:.3})", d80.0));
    }
    tx.send(Cmd::DrumInst(None)).unwrap();
    check!("sf drums again", render(1.0, "drums: back to the SoundFont kit", &mut eng), true);

    p.delay_mix.set(0.4);
    p.cutoff.set(0.45);
    p.resonance.set(0.6);
    tx.send(Cmd::Program { layer: false, bank: 0, prog: 0, inst: None }).unwrap();
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
    let up: Vec<u8> = theory::SCALES[theory::scale_index("Freygish (Ahava Rabboh)")].steps.iter().map(|s| 62 + s).chain(std::iter::once(74)).collect();
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

    // controller mapping: MiniLab 3 knob 1 (CC74) → filter, fader 1 (CC82) → volume, encoder (CC114) → browse
    p.cc_map[74].store(engine::target::FILTER, Relaxed);
    p.cc_map[82].store(engine::target::MASTER, Relaxed);
    p.cc_map[114].store(engine::target::BROWSE, Relaxed);
    p.cc_map[102].store(engine::target::PAD0, Relaxed);
    while urx.try_recv().is_ok() {}
    tx.send(Cmd::Midi([0xB0, 74, 32])).unwrap();
    tx.send(Cmd::Midi([0xB0, 82, 127])).unwrap();
    tx.send(Cmd::Midi([0xB0, 114, 65])).unwrap();
    tx.send(Cmd::Midi([0xB0, 102, 100])).unwrap();
    render(0.05, "controller CCs", &mut eng);
    let (mut browse, mut pad) = (0, 0);
    while let Ok(e) = urx.try_recv() {
        match e {
            UiEvt::Browse(d) => browse += d as i32,
            UiEvt::Pad { vel, .. } if vel > 0 => pad += 1,
            _ => {}
        }
    }
    println!("  CC74→filter {:.2}, CC82→volume {:.2}, encoder steps {browse}, pad hits {pad}", p.cutoff.get(), p.master.get());
    if (p.cutoff.get() - 32.0 / 127.0).abs() > 0.01 || p.master.get() < 0.99 || browse != 1 || pad != 1 {
        fails.push("controller mapping".into());
    }
    // limiter: a fistful of loud notes at full volume is turned down, not squared off
    p.master.set(3.0); // way past full, to make sure the limiter catches it
    for k in [48u8, 52, 55, 60, 64, 67, 72, 76, 79, 84, 88, 91] {
        tx.send(Cmd::Midi([0x90, k, 127])).unwrap();
    }
    let (pk, _) = render(1.0, "12 loud notes, master 3x", &mut eng);
    for k in [48u8, 52, 55, 60, 64, 67, 72, 76, 79, 84, 88, 91] {
        tx.send(Cmd::Midi([0x80, k, 0])).unwrap();
    }
    render(0.5, "released", &mut eng);
    let pre = p.out_peak.get();
    println!("  limiter: before {pre:.2}, out {pk:.2}");
    if pk > 0.86 {
        fails.push("limiter let peaks into the clipper".into());
    }
    p.master.set(0.8);
    // mode arp: D freygish (D E♭ F♯ G A B♭ C), hold D4, "Mode run" walks the mode
    {
        let mask: u16 = [2, 3, 6, 7, 9, 10, 0].iter().fold(0, |m, pc| m | (1 << pc));
        p.scale_mask.store(mask, Relaxed);
        p.arp_notes.store(2, Relaxed);
        p.arp_mode.store(0, Relaxed);
        p.arp_oct.store(1, Relaxed);
        p.arp_rate.store(3, Relaxed);
        p.arp_on.store(true, Relaxed);
        while urx.try_recv().is_ok() {}
        tx.send(Cmd::Midi([0x90, 62, 100])).unwrap();
        render(3.0, "arp: D freygish run", &mut eng);
        tx.send(Cmd::Midi([0x80, 62, 0])).unwrap();
        p.arp_on.store(false, Relaxed);
        render(0.3, "arp off", &mut eng);
        let mut got = Vec::new();
        while let Ok(e) = urx.try_recv() {
            if let UiEvt::Note { key, vel, src, .. } = e {
                if src == engine::src::ARP && vel > 0 && got.len() < 8 {
                    got.push(key);
                }
            }
        }
        println!("  mode arp notes: {got:?}");
        if got.len() < 8 || got[..7] != [62, 63, 66, 67, 69, 70, 72] {
            fails.push("mode arp didn't walk D freygish".into());
        }
        p.scale_mask.store(0, Relaxed);
        p.arp_notes.store(0, Relaxed);
    }
    // real range mode: an ocarina-range instrument refuses C4 and plays C5
    p.range_lo.store(69, Relaxed);
    p.range_hi.store(89, Relaxed);
    while urx.try_recv().is_ok() {}
    tx.send(Cmd::Midi([0x90, 60, 100])).unwrap();
    tx.send(Cmd::Midi([0x90, 72, 100])).unwrap();
    render(0.05, "real range", &mut eng);
    tx.send(Cmd::Midi([0x80, 60, 0])).unwrap();
    tx.send(Cmd::Midi([0x80, 72, 0])).unwrap();
    render(0.05, "real range off", &mut eng);
    p.range_lo.store(0, Relaxed);
    p.range_hi.store(127, Relaxed);
    let (mut refused, mut played60, mut played72) = (false, false, false);
    while let Ok(e) = urx.try_recv() {
        match e {
            UiEvt::OutOfRange(60) => refused = true,
            UiEvt::Note { key: 60, vel, .. } if vel > 0 => played60 = true,
            UiEvt::Note { key: 72, vel, .. } if vel > 0 => played72 = true,
            _ => {}
        }
    }
    println!("  real range: C4 refused {refused}, C4 played {played60}, C5 played {played72}");
    if !refused || played60 || !played72 {
        fails.push("real range mode".into());
    }
    // "Hear it": a scale plays on the player's own sound, at the right pitches
    {
        tx.send(Cmd::Program { layer: false, bank: 0, prog: 73, inst: None }).unwrap(); // flute
        p.song_user.store(0, Relaxed); // "Hear it" plays every note
        p.song_wait.store(false, Relaxed);
        let up: Vec<u8> = [0u8, 2, 4, 5, 7, 9, 11, 12].iter().map(|s| 60 + s).collect();
        // bank/program of a recorded sound: not a GM program, must not reach the song synth
        let song = song::scale_song(&up, 120.0, sounds::SAMPLE_BANK, 5);
        tx.send(Cmd::LoadSong(Some(Arc::new(song)))).unwrap();
        tx.send(Cmd::SongPlay(true)).unwrap();
        let grab = |secs: f32, eng: &mut Engine| {
            let (mut gl, mut gr) = (vec![0.0f32; block], vec![0.0f32; block]);
            let mut v = Vec::new();
            for _ in 0..(secs * sr as f32) as usize / block {
                eng.process(&mut gl, &mut gr);
                v.extend_from_slice(&gl);
            }
            v
        };
        let f0 = |x: &[f32]| -> f32 {
            let (mut best, mut lag) = (f32::MIN, 0);
            for t in 60..400 {
                let c: f32 = (0..x.len() - t).map(|i| x[i] * x[i + t]).sum();
                if c > best {
                    best = c;
                    lag = t;
                }
            }
            sr as f32 / lag as f32
        };
        // eighth notes at 120 bpm: note i sounds from 0.05 + i*0.25 s for 0.225 s
        grab(0.15, &mut eng);
        let a = grab(0.10, &mut eng); // C4
        grab(0.90, &mut eng);
        let b = grab(0.10, &mut eng); // G4
        let (fa, fb) = (f0(&a), f0(&b));
        let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
        println!("  hear it: {fa:.0} Hz then {fb:.0} Hz (want ~262, ~392), rms {:.3} {:.3}", rms(&a), rms(&b));
        if rms(&a) < 0.002 || rms(&b) < 0.002 || (fa - 261.6).abs() > 12.0 || (fb - 392.0).abs() > 18.0 {
            fails.push("hear it on own sound".into());
        }
        tx.send(Cmd::SongPlay(false)).unwrap();
        tx.send(Cmd::LoadSong(None)).unwrap();
        render(0.3, "hear it stopped", &mut eng);
    }
    // accordion: a button's off ends the note it started even if the bellows changed the note
    {
        while urx.try_recv().is_ok() {}
        tx.send(Cmd::Acc(engine::AccMsg::Treble { id: 7, note: 60, on: true })).unwrap();
        tx.send(Cmd::Acc(engine::AccMsg::Treble { id: 7, note: 62, on: false })).unwrap();
        render(0.02, "acc off after bellows flip", &mut eng);
        let mut open = std::collections::HashSet::new();
        while let Ok(e) = urx.try_recv() {
            if let UiEvt::Note { key, vel, src, .. } = e {
                if src == engine::src::LIVE {
                    if vel > 0 {
                        open.insert(key);
                    } else {
                        open.remove(&key);
                    }
                }
            }
        }
        println!("  accordion bars left open: {open:?}");
        if !open.is_empty() {
            fails.push("accordion note left stuck in the waterfall".into());
        }
    }
    // the same knob must still work in accordion mode
    p.acc_on.store(true, Relaxed);
    tx.send(Cmd::Midi([0xB0, 74, 100])).unwrap();
    render(0.05, "knob in accordion mode", &mut eng);
    p.acc_on.store(false, Relaxed);
    println!("  accordion mode: CC74→filter {:.2}", p.cutoff.get());
    if (p.cutoff.get() - 100.0 / 127.0).abs() > 0.01 {
        fails.push("knobs dead in accordion mode".into());
    }
    p.master.set(0.8);

    // wind controller: breath (CC2) must control loudness
    p.wind_on.store(true, Relaxed);
    tx.send(Cmd::Program { layer: false, bank: 0, prog: 66, inst: None }).unwrap(); // tenor sax
    tx.send(Cmd::Midi([0xB0, 2, 120])).unwrap();
    tx.send(note(62, true)).unwrap();
    render(0.1, "  (wind settle)", &mut eng);
    let loud = render(0.6, "wind: sax, strong breath", &mut eng);
    tx.send(Cmd::Midi([0xB0, 2, 25])).unwrap();
    render(0.1, "  (breath drops)", &mut eng);
    let soft = render(0.6, "wind: sax, soft breath", &mut eng);
    tx.send(note(62, false)).unwrap();
    println!("  breath lock = CC{}, rms strong {:.4} vs soft {:.4}", p.breath_locked.load(Relaxed), loud.1, soft.1);
    if soft.1 * 2.0 > loud.1 || p.breath_locked.load(Relaxed) != 2 {
        fails.push("breath doesn't control volume".into());
    }
    p.wind_on.store(false, Relaxed);
    render(0.3, "  (wind off)", &mut eng);

    // chip voices: every patch makes sound through the engine, then hands the channel back
    p.cutoff.set(1.0); // the controller test above closed the filter
    for (i, pt) in chip::PATCHES.iter().enumerate() {
        tx.send(Cmd::Program { layer: false, bank: 0, prog: 0, inst: Some(std::sync::Arc::new(native::InstDef::Chip(*pt))) }).unwrap();
        for k in [60, 64, 67] {
            tx.send(note(k, true)).unwrap();
        }
        let r = render(0.25, if i == 0 { "chip voices (each patch, a C chord)" } else { "" }, &mut eng);
        for k in [60, 64, 67] {
            tx.send(note(k, false)).unwrap();
        }
        render(0.15, "", &mut eng);
        if r.1 < 0.003 || !r.1.is_finite() {
            fails.push(format!("chip patch {} is silent", pt.name));
        }
    }
    for pt in analog::BANKS.iter().flat_map(|b| b.iter()) {
        tx.send(Cmd::Program { layer: false, bank: 0, prog: 0, inst: Some(std::sync::Arc::new(native::InstDef::Analog(*pt))) }).unwrap();
        for k in [57, 60, 64] {
            tx.send(note(k, true)).unwrap();
        }
        let r = render(0.6, "", &mut eng);
        for k in [57, 60, 64] {
            tx.send(note(k, false)).unwrap();
        }
        render(0.3, "", &mut eng);
        println!("  synthwave {:<18} peak {:.3} rms {:.4}", pt.name, r.0, r.1);
        if r.1 < 0.003 || r.0 >= 0.999 {
            fails.push(format!("synthwave {} level (peak {:.3}, rms {:.4})", pt.name, r.0, r.1));
        }
    }
    // a sample instrument (generated SFZ) plays legato
    {
        let d = std::env::temp_dir().join("noodle_render_sfz");
        let _ = std::fs::create_dir_all(&d);
        let sr = 44100u32;
        let mut data = Vec::new();
        for i in 0..(sr as usize) {
            let v = ((i as f32 / sr as f32 * 440.0 * std::f32::consts::TAU).sin() * 16000.0) as i16;
            data.extend_from_slice(&v.to_le_bytes());
        }
        let mut b = Vec::new();
        b.extend_from_slice(b"RIFF");
        b.extend_from_slice(&((36 + data.len()) as u32).to_le_bytes());
        b.extend_from_slice(b"WAVEfmt ");
        for v in [16u32] {
            b.extend_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&1u16.to_le_bytes());
        b.extend_from_slice(&sr.to_le_bytes());
        b.extend_from_slice(&(sr * 2).to_le_bytes());
        b.extend_from_slice(&2u16.to_le_bytes());
        b.extend_from_slice(&16u16.to_le_bytes());
        b.extend_from_slice(b"data");
        b.extend_from_slice(&(data.len() as u32).to_le_bytes());
        b.extend_from_slice(&data);
        let _ = std::fs::write(d.join("a4.wav"), b);
        let _ = std::fs::write(d.join("Test Flute.sfz"), "<region> sample=a4.wav pitch_keycenter=69 loop_mode=loop_continuous loop_start=4410 loop_end=44099 ampeg_release=0.1\n");
        match sampler::load(&d.join("Test Flute.sfz")) {
            Ok(inst) => {
                tx.send(Cmd::Program { layer: false, bank: 0, prog: 0, inst: Some(std::sync::Arc::new(native::InstDef::Sampler(std::sync::Arc::new(inst)))) }).unwrap();
                tx.send(note(69, true)).unwrap();
                check!("sample instrument", render(0.5, "sample instrument (SFZ)", &mut eng), true);
                tx.send(note(71, true)).unwrap();
                tx.send(note(69, false)).unwrap();
                check!("sample legato", render(1.5, "  legato to B, held past the sample's end (loops)", &mut eng), true);
                tx.send(note(71, false)).unwrap();
                check!("sample release", render(0.5, "  released", &mut eng), false);
            }
            Err(e) => fails.push(format!("sfz load: {e}")),
        }
    }
    tx.send(Cmd::Program { layer: false, bank: 0, prog: 0, inst: None }).unwrap();
    // metronome clicks
    p.metro_on.store(true, Relaxed);
    let mr = render(1.2, "metronome", &mut eng);
    if mr.0 < 0.1 {
        fails.push("metronome: no click".into());
    }
    p.metro_on.store(false, Relaxed);
    render(0.3, "", &mut eng);

    // accordion: B/C box, a few buttons in each bellows direction, a bass, a reversal
    {
        use engine::AccMsg;
        let insts = accordion::instruments();
        let bc = &insts[0];
        render(0.5, "  (before accordion)", &mut eng);
        tx.send(Cmd::Acc(AccMsg::Setup(Some((bc.registers[2], bc.bass_ranks, bc.tuning, None))))).unwrap();
        tx.send(Cmd::Acc(AccMsg::Bellows(true))).unwrap();
        let tune = [(1usize, 2usize, true), (1, 2, false), (1, 3, true), (1, 3, false), (1, 4, true), (1, 4, false), (1, 5, true)];
        let (bn, chord) = accordion::bass_notes(accordion::BassSnd::Major(0));
        tx.send(Cmd::Acc(AccMsg::Bass { id: 300, notes: [bn[0], bn[1], bn[2]], n: 3, chord, on: true })).unwrap();
        let mut loud = 0.0f32;
        for (r, i, push) in tune {
            tx.send(Cmd::Acc(AccMsg::Bellows(push))).unwrap();
            let b = bc.rows[r][i];
            tx.send(Cmd::Acc(AccMsg::Treble { id: 1, note: if push { b.push } else { b.pull }, on: true })).unwrap();
            let (pk, _) = render(0.25, "", &mut eng);
            loud = loud.max(pk);
            tx.send(Cmd::Acc(AccMsg::Treble { id: 1, note: 0, on: false })).unwrap();
        }
        tx.send(Cmd::Acc(AccMsg::Bass { id: 300, notes: [0; 3], n: 0, chord, on: false })).unwrap();
        println!("accordion B/C scale + C chord, wet register: peak {loud:.3}");
        render(1.5, "  accordion released (reverb tail)", &mut eng);
        let quiet = render(0.6, "  then quiet", &mut eng);
        if loud < 0.05 || loud > 1.2 {
            fails.push(format!("accordion level {loud}"));
        }
        if quiet.1 > 0.003 {
            fails.push("accordion kept sounding".into());
        }
        // MIDI keyboard in accordion mode plays the accordion, not the piano
        tx.send(note(64, true)).unwrap();
        let m = render(0.4, "  MIDI key → accordion", &mut eng);
        tx.send(note(64, false)).unwrap();
        if m.0 < 0.02 {
            fails.push("MIDI key silent in accordion mode".into());
        }
        tx.send(Cmd::Acc(AccMsg::Setup(None))).unwrap();
        render(0.5, "", &mut eng);
    }

    // drones + bagpipe chanter: GHB in A
    p.wind_on.store(false, Relaxed);
    tx.send(Cmd::Program { layer: false, bank: 0, prog: 109, inst: None }).unwrap(); // GM bagpipe
    p.drone_tone.store(0, Relaxed);
    p.drone_cents[0].store(5700, Relaxed); // A3 tenor
    p.drone_cents[1].store(5700, Relaxed);
    p.drone_cents[2].store(4500, Relaxed); // A2 bass
    p.drone_on.store(true, Relaxed);
    check!("drones", render(1.0, "bagpipe drones (A3 A3 A2)", &mut eng), true);
    p.pipe_mode.store(true, Relaxed);
    p.pipe_continuous.store(true, Relaxed);
    p.pipe_tonic.store(69, Relaxed);
    tx.send(note(69, true)).unwrap();
    render(0.3, "  chanter low A", &mut eng);
    tx.send(note(71, true)).unwrap();
    tx.send(note(71, false)).unwrap(); // a quick cut: falls back to the A still held
    render(0.2, "  cut, back to A", &mut eng);
    tx.send(note(69, false)).unwrap(); // all fingers up: chanter keeps sounding
    let sustained = render(0.6, "chanter sustains (fingers up)", &mut eng);
    p.drone_on.store(false, Relaxed);
    p.pipe_mode.store(false, Relaxed);
    tx.send(Cmd::Panic).unwrap();
    let after = render(1.2, "drones off + panic (should fade)", &mut eng);
    let silent = render(1.0, "  then silence", &mut eng);
    println!("  chanter+drones rms {:.4}, after stop {:.4}, then {:.5}", sustained.1, after.1, silent.1);
    if sustained.1 < 0.01 {
        fails.push("chanter/drones not sustaining".into());
    }
    if silent.1 > 0.002 {
        fails.push("chanter/drones still sounding after stop".into());
    }
    // leaving bagpipe mode must also stop a sustaining chanter
    p.pipe_mode.store(true, Relaxed);
    tx.send(note(69, true)).unwrap();
    tx.send(note(69, false)).unwrap();
    render(0.3, "  chanter held by never-stops", &mut eng);
    p.pipe_mode.store(false, Relaxed);
    render(0.5, "  leave bagpipe mode", &mut eng);
    let gone = render(0.5, "  chanter gone", &mut eng);
    if gone.1 > 0.002 {
        fails.push("chanter kept sounding after leaving bagpipe mode".into());
    }
    p.pipe_continuous.store(false, Relaxed);

    // each chanter's harmonics, measured off the audio, match its table (MacAdam's measurements)
    {
        p.drone_on.store(false, Relaxed);
        p.pipe_mode.store(true, Relaxed);
        p.pipe_just.store(false, Relaxed);
        p.pipe_tune.store(0, Relaxed);
        p.chanter_level.set(0.7);
        let goertzel = |x: &[f32], f: f32| -> f32 {
            let w = std::f32::consts::TAU * f / sr as f32;
            let (mut re, mut im) = (0.0f32, 0.0f32);
            for (i, v) in x.iter().enumerate() {
                // Hann window
                let h = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / x.len() as f32).cos();
                re += v * h * (w * i as f32).cos();
                im += v * h * (w * i as f32).sin();
            }
            (re * re + im * im).sqrt()
        };
        for (voice, key, name) in [(0u8, 69u8, "Highland"), (1, 69, "Scottish smallpipes"), (2, 69, "Border"), (3, 64, "Uilleann (low E)"), (4, 69, "Northumbrian")] {
            p.chanter_voice.store(voice, Relaxed);
            p.pipe_tonic.store(if voice == 3 { 62 } else { key }, Relaxed);
            tx.send(note(key, true)).unwrap();
            let (mut gl, mut gr) = (vec![0.0f32; block], vec![0.0f32; block]);
            let mut buf = Vec::new();
            for i in 0..(0.6 * sr as f32) as usize / block {
                eng.process(&mut gl, &mut gr);
                if i * block > (0.25 * sr as f32) as usize {
                    buf.extend_from_slice(&gl);
                }
            }
            tx.send(note(key, false)).unwrap();
            tx.send(Cmd::Panic).unwrap();
            let mut z = (vec![0.0f32; block], vec![0.0f32; block]);
            for _ in 0..40 {
                eng.process(&mut z.0, &mut z.1);
            }
            let f0 = 440.0 * 2f32.powf((key as f32 - 69.0) / 12.0);
            let a1 = goertzel(&buf, f0);
            let got: Vec<f32> = (1..=8).map(|n| 20.0 * (goertzel(&buf, f0 * n as f32) / a1).log10()).collect();
            let want = engine_tables(voice);
            let worst = got.iter().zip(want.iter()).map(|(g, w)| (g - w).abs()).fold(0.0f32, f32::max);
            println!("  {name:<22} H1–8 dB: {}  (worst off by {worst:.1} dB)", got.iter().map(|g| format!("{g:+.0}")).collect::<Vec<_>>().join(" "));
            if worst > 3.0 {
                fails.push(format!("{name} chanter spectrum off its measurements"));
            }
        }
        // closed fingering: a brief silence between two legato notes
        {
            p.chanter_voice.store(4, Relaxed);
            p.pipe_tonic.store(67, Relaxed);
            p.pipe_closed.store(true, Relaxed);
            tx.send(note(67, true)).unwrap();
            let (mut gl, mut gr) = (vec![0.0f32; 64], vec![0.0f32; 64]);
            let mut env = Vec::new();
            for i in 0..(0.6 * sr as f32) as usize / 64 {
                if i == (0.3 * sr as f32) as usize / 64 {
                    tx.send(note(69, true)).unwrap();
                    tx.send(note(67, false)).unwrap();
                }
                eng.process(&mut gl, &mut gr);
                env.push((gl.iter().map(|v| v * v).sum::<f32>() / 64.0).sqrt());
            }
            let held = env[(0.2 * sr as f32) as usize / 64];
            let mid = (0.3 * sr as f32) as usize / 64;
            let dip = env[mid..mid + 30].iter().cloned().fold(f32::MAX, f32::min);
            let after = env[env.len() - 5];
            println!("  closed fingering: held {held:.3}, between notes {dip:.4}, next note {after:.3}");
            if dip > held * 0.1 || after < held * 0.5 {
                fails.push("closed fingering gap".into());
            }
            tx.send(note(69, false)).unwrap();
            tx.send(Cmd::Panic).unwrap();
            p.pipe_closed.store(false, Relaxed);
        }
        // uilleann regulators: B below the chanter strikes the G row; one regulator sounds its top open key
        {
            p.chanter_voice.store(3, Relaxed);
            p.pipe_tonic.store(62, Relaxed);
            p.regs_on.store(true, Relaxed);
            tx.send(note(59, true)).unwrap(); // B3
            let (mut gl, mut gr) = (vec![0.0f32; block], vec![0.0f32; block]);
            let mut e = 0.0f32;
            for _ in 0..(0.3 * sr as f32) as usize / block {
                eng.process(&mut gl, &mut gr);
                e += gl.iter().map(|v| v * v).sum::<f32>();
            }
            let got: Vec<u8> = (0..3).map(|r| p.reg_sounding[r].load(Relaxed)).collect();
            println!("  regulators, B row: {:?} (want [71, 67, 59]), energy {:.2}", got, e);
            if got != vec![71, 67, 59] || e < 1.0 {
                fails.push("regulator row".into());
            }
            tx.send(note(59, false)).unwrap();
            p.reg_keys[0].store((1 << 1) | (1 << 3), Relaxed); // tenor B and G together
            for _ in 0..8 {
                eng.process(&mut gl, &mut gr);
            }
            let t = p.reg_sounding[0].load(Relaxed);
            let b = p.reg_sounding[2].load(Relaxed);
            println!("  tenor with B+G open sounds {t} (want 71), bass after release {b} (want 255)");
            if t != 71 || b != 255 {
                fails.push("regulator top key".into());
            }
            p.reg_keys[0].store(0, Relaxed);
            p.regs_on.store(false, Relaxed);
            for _ in 0..20 {
                eng.process(&mut gl, &mut gr);
            }
        }
        // recorded Highland chanter (built pack, if one is at NOODLE_HIGHLAND_PACK): legato, own tuning
        if let Ok(dir) = std::env::var("NOODLE_HIGHLAND_PACK") {
            let inst = sampler::load(&std::path::Path::new(&dir).join("Great Highland chanter (recorded).sfz")).expect("pack");
            let inst = Some(Arc::new(native::InstDef::Sampler(Arc::new(inst))));
            tx.send(Cmd::Program { layer: false, bank: sounds::SAMPLE_BANK, prog: 1, inst }).unwrap();
            p.pipe_rec.store(true, Relaxed);
            p.pipe_closed.store(false, Relaxed);
            let (mut gl, mut gr) = (vec![0.0f32; block], vec![0.0f32; block]);
            let mut grab = |secs: f32, eng: &mut Engine| {
                let mut v = Vec::new();
                for _ in 0..(secs * sr as f32) as usize / block {
                    eng.process(&mut gl, &mut gr);
                    v.extend_from_slice(&gl);
                }
                v
            };
            let yin = |x: &[f32]| pipe_pack::pitch(x, sr, 300.0, 700.0).unwrap_or(0.0);
            tx.send(note(69, true)).unwrap();
            grab(0.5, &mut eng);
            let a = grab(0.3, &mut eng);
            tx.send(note(71, true)).unwrap();
            tx.send(note(69, false)).unwrap();
            grab(0.3, &mut eng);
            let b = grab(0.3, &mut eng);
            let held = grab(5.0, &mut eng); // through several loops
            tx.send(note(71, false)).unwrap();
            grab(1.0, &mut eng);
            let tail = grab(0.6, &mut eng); // after the reverb has mostly gone
            let (fa, fb) = (yin(&a), yin(&b));
            let cents_b = 1200.0 * (fb / 493.883).log2();
            let rms = |x: &[f32]| (x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32).sqrt();
            let chunks: Vec<f32> = held.chunks(2400).map(rms).collect();
            let (mn, mx) = chunks.iter().fold((f32::MAX, 0f32), |m, v| (m.0.min(*v), m.1.max(*v)));
            println!("  recorded Highland: low A {fa:.1} Hz, B {fb:.1} Hz ({cents_b:+.1} ¢), held 5 s level {mn:.3}–{mx:.3}, after release {:.4}", rms(&tail[tail.len() / 2..]));
            if (fa - 440.0).abs() > 1.5 || (cents_b - 3.0).abs() > 3.0 || mn < mx * 0.6 || rms(&tail[tail.len() / 2..]) > mx * 0.1 {
                fails.push("recorded Highland chanter".into());
            }
            p.pipe_rec.store(false, Relaxed);
            tx.send(Cmd::Panic).unwrap();
        }
        p.pipe_mode.store(false, Relaxed);
        fn engine_tables(v: u8) -> [f32; 8] {
            crate::pipes::CHANTERS[v as usize].harm
        }
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

/// Every bagpipe playing the same phrase over its own drones, one after another.
fn pipe_demo(out: &str) {
    let sr = 48000u32;
    let p = Arc::new(Params::new());
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, _urx) = crossbeam_channel::bounded::<UiEvt>(1 << 20);
    let mut eng = Engine::new(p.clone(), rx, utx, sr as f32, 8192);
    if let Some(sf) = sounds::find_soundfonts().into_iter().next() {
        if let Ok(l) = sounds::load_soundfont(&sf, sr) {
            tx.send(Cmd::Synths(l.synths)).unwrap();
        }
    }
    let mut wav = hound::WavWriter::create(out, hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int }).unwrap();
    let block = 256;
    let (mut l, mut r) = (vec![0.0f32; block], vec![0.0f32; block]);
    let mut run = |secs: f32, eng: &mut Engine| {
        for _ in 0..(secs * sr as f32) as usize / block {
            eng.process(&mut l, &mut r);
            for i in 0..block {
                wav.write_sample((l[i] * 32767.0) as i16).unwrap();
                wav.write_sample((r[i] * 32767.0) as i16).unwrap();
            }
        }
    };
    p.chanter_level.set(0.7);
    p.drone_level.set(0.45);
    p.pipe_just.store(true, Relaxed);
    // degrees of the scale: up and home again
    let phrase: [(i32, f32); 9] = [(0, 0.5), (2, 0.25), (4, 0.25), (5, 0.5), (7, 0.5), (5, 0.25), (4, 0.25), (2, 0.5), (0, 1.2)];
    for (pi, closed) in [(0usize, false), (2, false), (1, false), (11, true), (3, false), (4, true)] {
        let pp = &pipes::PIPES[pi];
        println!("{:>5.1}s  {}", 0.0, pp.name);
        p.chanter_voice.store(pi as u8, Relaxed);
        p.pipe_tonic.store(pp.tonic, Relaxed);
        p.drone_tone.store(pp.tone, Relaxed);
        p.pipe_closed.store(closed, Relaxed);
        for i in 0..13 {
            p.drone_cents[i].store(i32::MIN, Relaxed);
        }
        for (i, d) in pp.drones.iter().enumerate() {
            if d.2 {
                p.drone_cents[i].store((pp.tonic as i32 + d.0 as i32) * 100, Relaxed);
            }
        }
        p.drone_on.store(true, Relaxed);
        p.pipe_mode.store(true, Relaxed);
        run(0.8, &mut eng); // drones strike in
        let mut prev: Option<u8> = None;
        for (deg, len) in phrase {
            let k = (pp.tonic as i32 + deg) as u8;
            tx.send(Cmd::Midi([0x90, k, 100])).unwrap();
            if let Some(pk) = prev {
                tx.send(Cmd::Midi([0x80, pk, 0])).unwrap();
            }
            prev = Some(k);
            run(len * 0.75, &mut eng);
        }
        if let Some(pk) = prev {
            tx.send(Cmd::Midi([0x80, pk, 0])).unwrap();
        }
        p.drone_on.store(false, Relaxed);
        tx.send(Cmd::Panic).unwrap();
        run(0.9, &mut eng);
    }
}

/// Uilleann chanter over drones with regulator stabs: G row and D row on the beats.
fn regs_demo(out: &str) {
    let sr = 48000u32;
    let p = Arc::new(Params::new());
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, _urx) = crossbeam_channel::bounded::<UiEvt>(1 << 20);
    let mut eng = Engine::new(p.clone(), rx, utx, sr as f32, 8192);
    if let Some(sf) = sounds::find_soundfonts().into_iter().next() {
        if let Ok(l) = sounds::load_soundfont(&sf, sr) {
            tx.send(Cmd::Synths(l.synths)).unwrap();
        }
    }
    let mut wav = hound::WavWriter::create(out, hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int }).unwrap();
    let block = 256;
    let (mut l, mut r) = (vec![0.0f32; block], vec![0.0f32; block]);
    let mut run = |secs: f32, eng: &mut Engine| {
        for _ in 0..(secs * sr as f32) as usize / block {
            eng.process(&mut l, &mut r);
            for i in 0..block {
                wav.write_sample((l[i] * 32767.0) as i16).unwrap();
                wav.write_sample((r[i] * 32767.0) as i16).unwrap();
            }
        }
    };
    let pp = &pipes::PIPES[3];
    p.chanter_voice.store(3, Relaxed);
    p.pipe_tonic.store(62, Relaxed);
    p.drone_tone.store(pp.tone, Relaxed);
    p.chanter_level.set(0.7);
    p.drone_level.set(0.4);
    p.pipe_just.store(true, Relaxed);
    p.regs_on.store(true, Relaxed);
    p.reg_level.set(0.6);
    for i in 0..13 {
        p.drone_cents[i].store(i32::MIN, Relaxed);
    }
    for (i, d) in pp.drones.iter().enumerate() {
        if d.2 {
            p.drone_cents[i].store((62 + d.0 as i32) * 100, Relaxed);
        }
    }
    // a recorded chanter instead of the model (NOODLE_DEMO_SFZ)
    if let Ok(f) = std::env::var("NOODLE_DEMO_SFZ") {
        let inst = sampler::load(std::path::Path::new(&f)).expect("sfz");
        tx.send(Cmd::Program { layer: false, bank: sounds::SAMPLE_BANK, prog: 1, inst: Some(Arc::new(native::InstDef::Sampler(Arc::new(inst)))) }).unwrap();
        p.pipe_rec.store(true, Relaxed);
    }
    // recorded regulators instead of the model (NOODLE_DEMO_REGS = the regulators .sfz)
    if let Ok(f) = std::env::var("NOODLE_DEMO_REGS") {
        let r = drone_rec::load_regs(std::path::Path::new(&f)).expect("regs");
        tx.send(Cmd::RegSet(Some(Arc::new(r)))).unwrap();
    }
    // recorded drones instead of the model (NOODLE_DEMO_DRONES = the drones .sfz)
    if let Ok(f) = std::env::var("NOODLE_DEMO_DRONES") {
        let d = drone_rec::load(std::path::Path::new(&f)).expect("drones");
        tx.send(Cmd::DroneLoop(Some(Arc::new(d)))).unwrap();
        p.drone_rec_cents.store(6200, Relaxed);
        for i in 0..13 {
            p.drone_cents[i].store(i32::MIN, Relaxed);
        }
    }
    p.drone_on.store(true, Relaxed);
    p.pipe_mode.store(true, Relaxed);
    run(1.0, &mut eng);
    // the scale up and down first
    for k in [62u8, 64, 66, 67, 69, 71, 73, 74, 76, 78, 79, 81, 83, 86, 83, 81, 79, 78, 76, 74, 73, 71, 69, 67, 66, 64, 62] {
        tx.send(Cmd::Midi([0x90, k, 100])).unwrap();
        run(0.3, &mut eng);
        tx.send(Cmd::Midi([0x80, k, 0])).unwrap();
    }
    run(0.4, &mut eng);
    // a jig in G: 6/8, eighths at ~0.2 s; row keys: B3 = G row, A3 = D row
    let bars: [([u8; 6], u8); 4] = [([67, 71, 74, 67, 71, 74], 59), ([66, 69, 74, 66, 69, 74], 57), ([67, 71, 74, 76, 74, 71], 59), ([69, 66, 62, 67, 67, 67], 57)];
    let eighth = 0.2;
    let mut prev: Option<u8> = None;
    for (notes, row) in bars.iter().chain(bars.iter()) {
        for (i, n) in notes.iter().enumerate() {
            tx.send(Cmd::Midi([0x90, *n, 100])).unwrap();
            if let Some(pk) = prev {
                if pk != *n {
                    tx.send(Cmd::Midi([0x80, pk, 0])).unwrap();
                }
            }
            prev = Some(*n);
            // regulator stab on each dotted-quarter beat
            if i % 3 == 0 {
                tx.send(Cmd::Midi([0x90, *row, 100])).unwrap();
                run(eighth * 0.6, &mut eng);
                tx.send(Cmd::Midi([0x80, *row, 0])).unwrap();
                run(eighth * 0.4, &mut eng);
            } else {
                run(eighth, &mut eng);
            }
        }
    }
    if let Some(pk) = prev {
        tx.send(Cmd::Midi([0x80, pk, 0])).unwrap();
    }
    tx.send(Cmd::Midi([0x90, 59, 100])).unwrap();
    tx.send(Cmd::Midi([0x90, 67, 100])).unwrap();
    run(1.2, &mut eng);
    tx.send(Cmd::Midi([0x80, 59, 0])).unwrap();
    tx.send(Cmd::Midi([0x80, 67, 0])).unwrap();
    p.drone_on.store(false, Relaxed);
    tx.send(Cmd::Panic).unwrap();
    run(1.0, &mut eng);
}

/// Play an event list on the uilleann pipes, offline. Lines: `t on KEY` (the chanter moves to KEY,
/// legato), `t off`, `t regs BITS0 BITS1 BITS2` (keys held on tenor, baritone, bass), `t end`.
/// Levels from NOODLE_DEMO_LEVELS="chanter drones regs"; NOODLE_DEMO_PARTS = any of c,d,r (stems).
fn tune_demo(out: &str, events: &str) {
    let sr = 48000u32;
    let p = Arc::new(Params::new());
    let (tx, rx) = crossbeam_channel::bounded::<Cmd>(4096);
    let (utx, _urx) = crossbeam_channel::bounded::<UiEvt>(1 << 20);
    let mut eng = Engine::new(p.clone(), rx, utx, sr as f32, 8192);
    if let Some(sf) = sounds::find_soundfonts().into_iter().next() {
        if let Ok(l) = sounds::load_soundfont(&sf, sr) {
            tx.send(Cmd::Synths(l.synths)).unwrap();
        }
    }
    let parts = std::env::var("NOODLE_DEMO_PARTS").unwrap_or_else(|_| "cdr".into());
    let lv: Vec<f32> = std::env::var("NOODLE_DEMO_LEVELS").unwrap_or_else(|_| "0.8 0.3 0.4".into()).split_whitespace().filter_map(|x| x.parse().ok()).collect();
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sounds");
    let inst = sampler::load(&dir.join("uilleann-chanter-2/Uilleann chanter (recorded).sfz")).expect("chanter");
    tx.send(Cmd::Program { layer: false, bank: sounds::SAMPLE_BANK, prog: 1, inst: Some(Arc::new(native::InstDef::Sampler(Arc::new(inst)))) }).unwrap();
    p.pipe_rec.store(true, Relaxed);
    let r = drone_rec::load_regs(&dir.join("uilleann-regulators/Uilleann regulators (recorded).sfz")).expect("regs");
    tx.send(Cmd::RegSet(Some(Arc::new(r)))).unwrap();
    let d = drone_rec::load(&dir.join("uilleann-drones-2/Uilleann drones (recorded).sfz")).expect("drones");
    tx.send(Cmd::DroneLoop(Some(Arc::new(d)))).unwrap();
    let pp = &pipes::PIPES[3];
    p.chanter_voice.store(3, Relaxed);
    p.pipe_tonic.store(62, Relaxed);
    p.drone_tone.store(pp.tone, Relaxed);
    p.pipe_just.store(true, Relaxed);
    p.regs_on.store(true, Relaxed);
    p.chanter_level.set(if parts.contains('c') { lv[0] } else { 0.0 });
    p.drone_level.set(if parts.contains('d') { lv[1] } else { 0.0 });
    p.reg_level.set(if parts.contains('r') { lv[2] } else { 0.0 });
    for i in 0..13 {
        p.drone_cents[i].store(i32::MIN, Relaxed);
    }
    p.drone_rec_cents.store(if parts.contains('d') { 6200 } else { i32::MIN }, Relaxed);
    p.drone_on.store(true, Relaxed);
    p.pipe_mode.store(true, Relaxed);
    let mut wav = hound::WavWriter::create(out, hound::WavSpec { channels: 2, sample_rate: sr, bits_per_sample: 16, sample_format: hound::SampleFormat::Int }).unwrap();
    let block = 64;
    let (mut l, mut r) = (vec![0.0f32; block], vec![0.0f32; block]);
    let mut t = 0.0f64;
    let mut cur: Option<u8> = None;
    let text = std::fs::read_to_string(events).expect("events");
    let mut evs: Vec<(f64, Vec<String>)> = text.lines().filter_map(|ln| {
        let mut w = ln.split_whitespace();
        let at: f64 = w.next()?.parse().ok()?;
        Some((at, w.map(|x| x.to_string()).collect()))
    }).collect();
    evs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    let mut end = 0.0;
    for (at, w) in evs {
        while t < at {
            eng.process(&mut l, &mut r);
            for i in 0..block {
                wav.write_sample((l[i].clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
                wav.write_sample((r[i].clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
            }
            t += block as f64 / sr as f64;
        }
        match w.first().map(|s| s.as_str()) {
            Some("on") => {
                let k: u8 = w[1].parse().unwrap();
                tx.send(Cmd::Midi([0x90, k, 100])).unwrap();
                if let Some(pk) = cur {
                    if pk != k {
                        tx.send(Cmd::Midi([0x80, pk, 0])).unwrap();
                    }
                }
                cur = Some(k);
            }
            Some("off") => {
                if let Some(pk) = cur.take() {
                    tx.send(Cmd::Midi([0x80, pk, 0])).unwrap();
                }
            }
            Some("regs") => {
                for i in 0..3 {
                    p.reg_keys[i].store(w[1 + i].parse().unwrap_or(0), Relaxed);
                }
            }
            Some("dronesoff") => p.drone_on.store(false, Relaxed),
            Some("bend") => {
                // cents off the note (slides, finger vibrato); the sampler bends ±2 semitones
                let c: f32 = w[1].parse().unwrap_or(0.0);
                let v = (8192.0 + c / 200.0 * 8192.0).clamp(0.0, 16383.0) as u16;
                tx.send(Cmd::Midi([0xE0, (v & 0x7F) as u8, (v >> 7) as u8])).unwrap();
            }
            Some("end") => end = at,
            _ => {}
        }
    }
    let _ = end;
    tx.send(Cmd::Panic).unwrap();
    for _ in 0..(1.5 * sr as f64) as usize / block {
        eng.process(&mut l, &mut r);
        for i in 0..block {
            wav.write_sample((l[i].clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
            wav.write_sample((r[i].clamp(-1.0, 1.0) * 32767.0) as i16).unwrap();
        }
    }
}
