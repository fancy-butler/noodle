// Noodle — make a playable instrument from one recording of a scale.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Play (or find) a recording of an instrument going up a scale one steady note at a time.
// Noodle follows the pitch every 10 ms, finds the steady notes, keeps the best take of each,
// loops it with a baked crossfade, and maps them across the keyboard. The overall offset from
// A = 440 is removed (so it plays in tune with everything else); each note keeps its own
// small tuning. The instrument is saved in your own instruments folder.

use std::path::{Path, PathBuf};

/// Decode any common audio file to (left, right, rate).
pub fn decode(path: &Path) -> Result<(Vec<f32>, Vec<f32>, u32), String> {
    use symphonia::core::{audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint};
    let f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mss = MediaSourceStream::new(Box::new(f), Default::default());
    let mut hint = Hint::new();
    if let Some(e) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(e);
    }
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|_| "Noodle can't read that kind of file. WAV, FLAC, MP3, OGG and M4A work (a YouTube .webm/opus doesn't — convert it to .wav first).".to_string())?;
    let mut fmt = probed.format;
    let track = fmt.tracks().iter().find(|t| t.codec_params.sample_rate.is_some()).ok_or("no audio in the file")?.clone();
    let mut dec = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| format!("can't decode this audio: {e}"))?;
    let rate = track.codec_params.sample_rate.unwrap_or(44100);
    let (mut l, mut r) = (Vec::new(), Vec::new());
    while let Ok(pkt) = fmt.next_packet() {
        if pkt.track_id() != track.id {
            continue;
        }
        let Ok(buf) = dec.decode(&pkt) else { continue };
        let spec = *buf.spec();
        let ch = spec.channels.count().max(1);
        let mut sb = SampleBuffer::<f32>::new(buf.capacity() as u64, spec);
        sb.copy_interleaved_ref(buf);
        for fr in sb.samples().chunks(ch) {
            l.push(fr[0]);
            r.push(if ch > 1 { fr[1] } else { fr[0] });
        }
    }
    if l.len() < rate as usize {
        return Err("the recording is under a second long".into());
    }
    Ok((l, r, rate))
}

/// One steady note found in the recording.
#[derive(Clone, Debug)]
pub struct Found {
    pub key: u8,
    pub hz: f32,
    /// cents from equal temperament, before the overall offset is removed
    pub cents: f32,
    pub start: usize,
    pub end: usize,
}

/// Pitch every 10 ms → steady stretches (≥ 0.25 s within ±35 cents) → one per key (the longest).
pub fn find_notes(mono: &[f32], rate: u32, lo: f32, hi: f32) -> Vec<Found> {
    let hop = (rate / 100) as usize;
    let win = ((rate as f32 / lo) as usize * 3).max(1024);
    let mut track: Vec<(f32, f32)> = Vec::new(); // (hz or 0, rms)
    let mut i = 0;
    while i + win + (rate as f32 / lo) as usize + 2 < mono.len() {
        let w = &mono[i..i + win + (rate as f32 / lo) as usize + 2];
        let rms = (w[..win].iter().map(|v| v * v).sum::<f32>() / win as f32).sqrt();
        let hz = crate::pipe_pack::pitch(w, rate, lo, hi).unwrap_or(0.0);
        track.push((hz, rms));
        i += hop;
    }
    let loud = track.iter().map(|t| t.1).fold(0f32, f32::max);
    let cents = |hz: f32| 1200.0 * (hz / 440.0).log2() + 6900.0;
    let mut runs: Vec<(usize, usize, f32)> = Vec::new(); // frames, median cents
    let mut s = 0;
    while s < track.len() {
        if track[s].0 <= 0.0 || track[s].1 < loud * 0.08 {
            s += 1;
            continue;
        }
        let c0 = cents(track[s].0);
        let mut e = s + 1;
        let mut run: Vec<f32> = vec![c0];
        while e < track.len() && track[e].0 > 0.0 && track[e].1 >= loud * 0.08 {
            let c = cents(track[e].0);
            let mut sorted = run.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if (c - sorted[sorted.len() / 2]).abs() > 35.0 {
                break;
            }
            run.push(c);
            e += 1;
        }
        if (e - s) * hop >= rate as usize / 4 {
            run.sort_by(|a, b| a.partial_cmp(b).unwrap());
            runs.push((s, e, run[run.len() / 2]));
        }
        s = e;
    }
    // one take per key: the longest
    let mut best: Vec<Found> = Vec::new();
    for (s, e, c) in runs {
        let key = (c / 100.0).round().clamp(0.0, 127.0) as u8;
        let f = Found { key, hz: 440.0 * 2f32.powf((c - 6900.0) / 1200.0), cents: c - key as f32 * 100.0, start: s * hop, end: e * hop + win / 2 };
        match best.iter_mut().find(|b| b.key == key) {
            Some(b) if b.end - b.start >= f.end - f.start => {}
            Some(b) => *b = f,
            None => best.push(f),
        }
    }
    best.sort_by_key(|b| b.key);
    best
}

/// Build the instrument into `<dir>/<name>/<name>.sfz`. Returns the .sfz and a line per note.
pub fn build(src: &Path, name: &str, dir: &Path, lo: f32, hi: f32) -> Result<(PathBuf, Vec<String>), String> {
    let (l, r, rate) = decode(src)?;
    let mono: Vec<f32> = l.iter().zip(&r).map(|(a, b)| 0.5 * (a + b)).collect();
    let notes = find_notes(&mono, rate, lo, hi);
    if notes.len() < 2 {
        return Err(format!("Found {} steady note{} — play each note of the scale for at least half a second, one at a time.", notes.len(), if notes.len() == 1 { "" } else { "s" }));
    }
    // the instrument's overall offset from A440: the median of the notes' deviations
    let mut devs: Vec<f32> = notes.iter().map(|n| n.cents).collect();
    devs.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let offset = devs[devs.len() / 2];
    let out = dir.join(name);
    let _ = std::fs::remove_dir_all(&out);
    std::fs::create_dir_all(out.join("samples")).map_err(|e| e.to_string())?;
    let mut sfz = format!("// {name}: built by Noodle from one recording of a scale ({}).\n<control> default_path=samples/\n<global> loop_mode=loop_continuous ampeg_attack=0.006 ampeg_release=0.06\n", src.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default());
    let mut report = vec![format!("{} notes · overall {offset:+.0} ¢ from A440 (removed)", notes.len())];
    let keys: Vec<u8> = notes.iter().map(|n| n.key).collect();
    for (i, n) in notes.iter().enumerate() {
        // the steady middle: skip 60 ms at each end (the slide in from the last note, the change out)
        let edge = (rate as f32 * 0.06) as usize;
        let (s0, s1) = (n.start + edge, n.end.saturating_sub(edge).min(l.len() - 1));
        if s1 <= s0 + rate as usize / 8 {
            continue;
        }
        let period = (rate as f32 / n.hz).max(2.0) as usize;
        // loop: from a quarter in to near the end, the end where the wave best matches the start
        let ls = s0 + (s1 - s0) / 4;
        let target = s1.saturating_sub(period * 3);
        let cmp = period * 2;
        let mut best = (f32::MAX, target);
        for le in target.saturating_sub(period * 3)..target {
            if le <= ls + period * 4 || le + cmp >= mono.len() {
                continue;
            }
            let e: f32 = (0..cmp).map(|k| (mono[ls + k] - mono[le + k]).abs()).sum();
            if e < best.0 {
                best = (e, le);
            }
        }
        let le = best.1;
        if le <= ls + period * 4 {
            continue;
        }
        let xf = ((rate as f32 * 0.04) as usize).min((le - ls) / 3).min(ls - s0);
        let mut nl: Vec<f32> = l[s0..=le].to_vec();
        let mut nr: Vec<f32> = r[s0..=le].to_vec();
        let (lls, lle) = (ls - s0, le - s0);
        for k in 0..xf {
            let t = (k + 1) as f32 / xf as f32;
            let (a, b) = (lle + 1 - xf + k, lls - xf + k);
            nl[a] = nl[a] * (1.0 - t) + nl[b] * t;
            nr[a] = nr[a] * (1.0 - t) + nr[b] * t;
        }
        for k in 1..=64 {
            nl.push(nl[lls + k]);
            nr.push(nr[lls + k]);
        }
        let rms = (nl[lls..lle].iter().chain(&nr[lls..lle]).map(|v| v * v).sum::<f32>() / (2 * (lle - lls)) as f32).sqrt();
        let g = 0.18 / rms.max(1e-4);
        let fade = (rate / 200) as usize;
        let file = format!("{:03}.wav", n.key);
        let spec = hound::WavSpec { channels: 2, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut w = hound::WavWriter::create(out.join("samples").join(&file), spec).map_err(|e| e.to_string())?;
        for k in 0..nl.len() {
            let f = if k < fade { k as f32 / fade as f32 } else { 1.0 };
            w.write_sample(((nl[k] * g * f).clamp(-1.0, 1.0) * 32767.0) as i16).map_err(|e| e.to_string())?;
            w.write_sample(((nr[k] * g * f).clamp(-1.0, 1.0) * 32767.0) as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
        let lo_k = if i == 0 { n.key.saturating_sub(4) } else { (keys[i - 1] + n.key) / 2 + 1 };
        let hi_k = if i + 1 == keys.len() { n.key.saturating_add(4).min(127) } else { (n.key + keys[i + 1]) / 2 };
        sfz.push_str(&format!("<region> sample={file} lokey={lo_k} hikey={hi_k} pitch_keycenter={} tune={:.0} loop_start={lls} loop_end={lle}\n", n.key, -offset));
        report.push(format!("{}: {:.1} Hz, {:+.0} ¢ of its own, {:.1} s steady", crate::theory::note_name_oct(n.key), n.hz, n.cents - offset, (n.end - n.start) as f32 / rate as f32));
    }
    let path = out.join(format!("{name}.sfz"));
    std::fs::write(&path, sfz).map_err(|e| e.to_string())?;
    let _ = std::fs::write(out.join("SOURCE.txt"), format!("Built by Noodle from {}\nFor your own use unless the recording's owner has said otherwise.\n\n{}\n", src.display(), report.join("\n")));
    Ok((path, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fake chanter going up D major, with a drone under it and slides between notes.
    #[test]
    fn finds_a_scale_over_a_drone() {
        let rate = 44100u32;
        let dir = std::env::temp_dir().join("noodle-recbuild-test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let scale = [62u8, 64, 66, 67, 69, 71, 73, 74];
        let quirks = [0.0f32, 4.0, -6.0, 2.0, 0.0, -3.0, 8.0, 1.0];
        let sharp = 30.0; // the whole set a bit sharp
        let mut x = Vec::new();
        let mut ph = 0.0f64;
        let mut dph = 0.0f64;
        for (i, k) in scale.iter().enumerate() {
            let f = 440.0 * 2f64.powf((*k as f64 - 69.0 + (sharp + quirks[i]) as f64 / 100.0) / 12.0);
            for n in 0..(rate as f64 * 0.7) as usize {
                // 40 ms glide from the previous note
                let ff = if n < (rate as usize / 25) && i > 0 { let p = 440.0 * 2f64.powf((scale[i - 1] as f64 - 69.0 + (sharp + quirks[i - 1]) as f64 / 100.0) / 12.0); p + (f - p) * n as f64 / (rate as f64 / 25.0) } else { f };
                ph += ff / rate as f64;
                dph += 146.83 * 2f64.powf(sharp as f64 / 1200.0) / rate as f64;
                let s: f64 = (1..8).map(|h| (std::f64::consts::TAU * ph * h as f64).sin() / h as f64).sum::<f64>() * 0.3 + (std::f64::consts::TAU * dph).sin() * 0.06;
                x.push(s as f32);
            }
        }
        let wav = dir.join("scale.wav");
        let mut w = hound::WavWriter::create(&wav, hound::WavSpec { channels: 1, sample_rate: rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int }).unwrap();
        for v in &x {
            w.write_sample((v * 32767.0) as i16).unwrap();
        }
        w.finalize().unwrap();
        let (sfz, report) = build(&wav, "Test chanter", &dir, 200.0, 1400.0).expect("build");
        for l in &report {
            println!("{l}");
        }
        let text = std::fs::read_to_string(&sfz).unwrap();
        assert_eq!(text.matches("<region>").count(), 8, "{text}");
        // planted: +30 ¢ for the set, each note with its own small quirk on top
        let off: f32 = report[0].split("overall ").nth(1).unwrap().split(' ').next().unwrap().parse().unwrap();
        assert!((off - 31.0).abs() <= 3.0, "{}", report[0]);
        let inst = crate::sampler::load(&sfz).expect("loads");
        assert_eq!(inst.regions.len(), 8);
    }
}
