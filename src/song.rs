// Noodle — MIDI file loading into a flat, time-sorted song.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub struct SongNote {
    pub start: f64,
    pub end: f64,
    pub key: u8,
    pub vel: u8,
    pub ch: u8,
}

#[derive(Clone, Copy, Debug)]
pub struct SongEv {
    pub t: f64,
    pub ch: u8,
    pub st: u8, // status high nibble (0x80, 0x90, 0xB0, 0xC0, 0xD0, 0xE0, 0xA0)
    pub d1: u8,
    pub d2: u8,
}

#[derive(Clone)]
pub struct Song {
    pub name: String,
    pub notes: Vec<SongNote>,
    pub evs: Vec<SongEv>,
    pub dur: f64,
    /// bitmask of channels that contain notes
    pub channels: u16,
    /// first program seen on each channel (-1 = none)
    pub programs: [i16; 16],
    pub notes_per_ch: [u32; 16],
    /// play the notes through the player's own sound instead of the GM song synth
    pub own_sound: bool,
    /// quarter notes per minute (the first tempo), for drawing note values
    pub qpm: f32,
}

pub fn load(path: &Path) -> Result<Song, String> {
    let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    parse(&bytes, name)
}

pub fn parse(bytes: &[u8], name: String) -> Result<Song, String> {
    let smf = Smf::parse(bytes).map_err(|e| format!("Not a readable MIDI file: {e}"))?;

    // Gather every event with its absolute tick.
    struct Raw {
        tick: u64,
        order: u32,
        kind: RawKind,
    }
    enum RawKind {
        Tempo(u32),
        TimeSig(u8, u8),
        Ch(u8, u8, u8, u8),
    }
    let mut raws: Vec<Raw> = Vec::new();
    let mut order = 0u32;
    for track in &smf.tracks {
        let mut tick = 0u64;
        for ev in track {
            tick += ev.delta.as_int() as u64;
            order += 1;
            match ev.kind {
                TrackEventKind::Meta(MetaMessage::Tempo(t)) => raws.push(Raw { tick, order, kind: RawKind::Tempo(t.as_int()) }),
                TrackEventKind::Meta(MetaMessage::TimeSignature(n, d, _, _)) => raws.push(Raw { tick, order, kind: RawKind::TimeSig(n, d) }),
                TrackEventKind::Midi { channel, message } => {
                    let ch = channel.as_int();
                    let (st, d1, d2) = match message {
                        MidiMessage::NoteOn { key, vel } => (0x90, key.as_int(), vel.as_int()),
                        MidiMessage::NoteOff { key, .. } => (0x80, key.as_int(), 0),
                        MidiMessage::Controller { controller, value } => (0xB0, controller.as_int(), value.as_int()),
                        MidiMessage::ProgramChange { program } => (0xC0, program.as_int(), 0),
                        MidiMessage::PitchBend { bend } => {
                            let v = bend.0.as_int();
                            (0xE0, (v & 0x7F) as u8, (v >> 7) as u8)
                        }
                        MidiMessage::ChannelAftertouch { vel } => (0xD0, vel.as_int(), 0),
                        MidiMessage::Aftertouch { key, vel } => (0xA0, key.as_int(), vel.as_int()),
                    };
                    raws.push(Raw { tick, order, kind: RawKind::Ch(ch, st, d1, d2) });
                }
                _ => {}
            }
        }
    }
    // Note-offs before note-ons at the same tick so repeated notes don't get eaten.
    raws.sort_by_key(|r| {
        let pri = match r.kind {
            RawKind::Tempo(_) | RawKind::TimeSig(..) => 0,
            RawKind::Ch(_, 0x80, _, _) => 1,
            RawKind::Ch(_, 0x90, _, 0) => 1,
            RawKind::Ch(_, 0x90, _, _) => 3,
            _ => 2,
        };
        (r.tick, pri, r.order)
    });

    let (ppq, fixed_sec_per_tick) = match smf.header.timing {
        Timing::Metrical(t) => (t.as_int().max(1) as f64, None),
        Timing::Timecode(fps, sub) => (1.0, Some(1.0 / (fps.as_f32() as f64 * sub.max(1) as f64))),
    };
    // Parts exported with every note at one velocity (common from notation apps)
    // get accents from the time signature, like a player would add.
    {
        let mut per_ch: [std::collections::HashMap<u8, u32>; 16] = Default::default();
        for r in &raws {
            if let RawKind::Ch(ch, 0x90, _, v) = r.kind {
                if v > 0 {
                    *per_ch[ch as usize].entry(v).or_default() += 1;
                }
            }
        }
        let flat: Vec<bool> = per_ch.iter().map(|m| {
            let total: u32 = m.values().sum();
            total >= 8 && m.values().copied().max().unwrap_or(0) * 10 >= total * 8
        }).collect();
        if flat.iter().enumerate().any(|(c, f)| *f && c != 9) && fixed_sec_per_tick.is_none() {
            let rhythm = name.to_lowercase();
            let (mut num, mut den, mut bar_tick, mut sig_tick) = (4u32, 4u32, 0u64, 0u64);
            for r in raws.iter_mut() {
                match r.kind {
                    RawKind::TimeSig(n, d) => {
                        num = n.max(1) as u32;
                        den = 1u32 << d.min(6);
                        sig_tick = r.tick;
                        bar_tick = r.tick;
                    }
                    RawKind::Ch(ch, 0x90, key, v) if v > 0 && flat[ch as usize] && ch != 9 => {
                        let bar_ticks = (ppq * 4.0 * num as f64 / den as f64).max(1.0) as u64;
                        let since = r.tick - sig_tick;
                        bar_tick = sig_tick + since / bar_ticks * bar_ticks;
                        let pos = (r.tick - bar_tick) as f64 / (ppq * 4.0);
                        let st = crate::abc::strength(&rhythm, num, den, pos);
                        let nv = (v as f64 * (0.55 + 0.5 * st)).round().clamp(1.0, 127.0) as u8;
                        r.kind = RawKind::Ch(ch, 0x90, key, nv);
                    }
                    _ => {}
                }
            }
            let _ = bar_tick;
        }
    }
    let mut us_per_beat = 500_000.0f64;
    let mut first_tempo: Option<f64> = None;
    let mut last_tick = 0u64;
    let mut t = 0.0f64;
    let mut evs = Vec::with_capacity(raws.len());
    for r in &raws {
        let dt = (r.tick - last_tick) as f64;
        t += match fixed_sec_per_tick {
            Some(s) => dt * s,
            None => dt * us_per_beat / 1_000_000.0 / ppq,
        };
        last_tick = r.tick;
        match r.kind {
            RawKind::Tempo(v) => {
                us_per_beat = v.max(1) as f64;
                if first_tempo.is_none() {
                    first_tempo = Some(us_per_beat);
                }
            }
            RawKind::TimeSig(..) => {}
            RawKind::Ch(ch, st, d1, d2) => evs.push(SongEv { t, ch, st, d1, d2 }),
        }
    }

    // Pair notes for display.
    let mut open: Vec<Vec<(f64, u8)>> = vec![Vec::new(); 16 * 128];
    let mut notes = Vec::new();
    let mut programs = [-1i16; 16];
    let mut notes_per_ch = [0u32; 16];
    let mut channels = 0u16;
    for e in &evs {
        let idx = e.ch as usize * 128 + e.d1 as usize;
        match (e.st, e.d2) {
            (0x90, v) if v > 0 => {
                open[idx].push((e.t, v));
                channels |= 1 << e.ch;
                notes_per_ch[e.ch as usize] += 1;
            }
            (0x80, _) | (0x90, 0) => {
                if !open[idx].is_empty() {
                    let (s, v) = open[idx].remove(0);
                    notes.push(SongNote { start: s, end: e.t.max(s + 0.03), key: e.d1, vel: v, ch: e.ch });
                }
            }
            (0xC0, _) => {
                if programs[e.ch as usize] < 0 {
                    programs[e.ch as usize] = e.d1 as i16;
                }
            }
            _ => {}
        }
    }
    let dur = evs.last().map(|e| e.t).unwrap_or(0.0);
    for (idx, v) in open.iter().enumerate() {
        for (s, vel) in v {
            notes.push(SongNote { start: *s, end: dur.max(s + 0.1), key: (idx % 128) as u8, vel: *vel, ch: (idx / 128) as u8 });
        }
    }
    notes.sort_by(|a, b| a.start.partial_cmp(&b.start).unwrap());
    if notes.is_empty() {
        return Err("That MIDI file has no notes in it.".into());
    }
    Ok(Song { name, notes, evs, dur: dur + 0.5, channels, programs, notes_per_ch, own_sound: false, qpm: (60_000_000.0 / first_tempo.unwrap_or(500_000.0)) as f32 })
}

/// Build a tiny song that plays a scale up and back down — used by "Hear it".
pub fn scale_song(keys: &[u8], bpm: f32, bank: i32, program: i32) -> Song {
    let step = 60.0 / bpm.max(30.0) as f64 / 2.0; // eighth notes
    let mut seq: Vec<u8> = keys.to_vec();
    let mut down: Vec<u8> = keys.iter().rev().skip(1).copied().collect();
    seq.append(&mut down);
    let mut evs = vec![
        SongEv { t: 0.0, ch: 0, st: 0xB0, d1: 0, d2: bank.clamp(0, 127) as u8 },
        SongEv { t: 0.0, ch: 0, st: 0xC0, d1: program.clamp(0, 127) as u8, d2: 0 },
    ];
    let mut notes = Vec::new();
    for (i, k) in seq.iter().enumerate() {
        let s = 0.05 + i as f64 * step;
        let len = if i + 1 == seq.len() { step * 3.0 } else { step * 0.9 };
        evs.push(SongEv { t: s, ch: 0, st: 0x90, d1: *k, d2: 96 });
        evs.push(SongEv { t: s + len, ch: 0, st: 0x80, d1: *k, d2: 0 });
        notes.push(SongNote { start: s, end: s + len, key: *k, vel: 96, ch: 0 });
    }
    evs.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap());
    let dur = evs.last().map(|e| e.t).unwrap_or(0.0) + 0.5;
    let mut programs = [-1i16; 16];
    programs[0] = program as i16;
    let mut npc = [0u32; 16];
    npc[0] = notes.len() as u32;
    Song { name: "Scale".into(), notes, evs, dur, channels: 1, programs, notes_per_ch: npc, own_sound: true, qpm: bpm.max(30.0) }
}
