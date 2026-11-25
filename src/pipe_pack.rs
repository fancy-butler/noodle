// Noodle — builds a playable Great Highland chanter from recorded notes.
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// Source: "Great Highland bagpipe samples" by IowaSpaceWizard on Freesound (pack 29644),
// CC0 — one long recording per chanter note. Noodle downloads them, measures each note's
// pitch, removes only the chanter's overall offset (Highland chanters sit well sharp of
// A = 440) so the notes keep their own just tuning against the drones, finds a smooth loop,
// bakes a crossfade into it, and writes an .sfz the sampler plays.

use std::path::{Path, PathBuf};

pub const CREDIT: &str = "Great Highland chanter · IowaSpaceWizard (Freesound pack 29644) · CC0";
const USER: u32 = 5364823;
/// (Freesound id, note name, written MIDI key: low A = A4)
pub const NOTES: [(u32, &str, u8); 9] = [
    (527278, "Low G", 67),
    (527276, "Low A", 69),
    (527269, "B", 71),
    (527268, "C", 73),
    (527275, "D", 74),
    (527274, "E", 76),
    (527273, "F", 78),
    (527277, "High G", 79),
    (527272, "High A", 81),
];

pub fn url(id: u32) -> String {
    format!("https://cdn.freesound.org/previews/{}/{}_{}-hq.mp3", id / 1000, id, USER)
}

pub fn dir() -> PathBuf {
    crate::packs::root().join("freesound-highland")
}
pub fn sfz_path() -> PathBuf {
    dir().join("Great Highland chanter (recorded).sfz")
}

/// Decode an mp3 to (left, right, rate).
pub fn decode_mp3(bytes: Vec<u8>) -> Result<(Vec<f32>, Vec<f32>, u32), String> {
    use symphonia::core::{audio::SampleBuffer, codecs::DecoderOptions, formats::FormatOptions, io::MediaSourceStream, meta::MetadataOptions, probe::Hint};
    let mss = MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
    let mut hint = Hint::new();
    hint.with_extension("mp3");
    let probed = symphonia::default::get_probe().format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default()).map_err(|e| format!("not an mp3: {e}"))?;
    let mut fmt = probed.format;
    let track = fmt.default_track().ok_or("no audio in the file")?.clone();
    let mut dec = symphonia::default::get_codecs().make(&track.codec_params, &DecoderOptions::default()).map_err(|e| e.to_string())?;
    let rate = track.codec_params.sample_rate.unwrap_or(44100);
    let (mut l, mut r) = (Vec::new(), Vec::new());
    loop {
        let pkt = match fmt.next_packet() {
            Ok(p) => p,
            Err(_) => break,
        };
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
        return Err("the recording is too short".into());
    }
    Ok((l, r, rate))
}

/// Fundamental by normalised autocorrelation (YIN-style difference), Hz.
pub fn pitch(x: &[f32], rate: u32, lo: f32, hi: f32) -> Option<f32> {
    let n = x.len();
    let (min_lag, max_lag) = ((rate as f32 / hi) as usize, (rate as f32 / lo) as usize);
    if n < max_lag * 3 {
        return None;
    }
    let w = n - max_lag - 1;
    let mut d = vec![0f32; max_lag + 2];
    for lag in 1..=max_lag + 1 {
        let mut s = 0.0;
        for i in 0..w {
            let e = x[i] - x[i + lag];
            s += e * e;
        }
        d[lag] = s;
    }
    // cumulative mean normalised difference
    let mut cm = vec![1f32; max_lag + 2];
    let mut run = 0.0;
    for lag in 1..=max_lag + 1 {
        run += d[lag];
        cm[lag] = d[lag] * lag as f32 / run.max(1e-12);
    }
    let mut best = None;
    for lag in min_lag.max(2)..=max_lag {
        if cm[lag] < 0.15 && cm[lag] <= cm[lag - 1] && cm[lag] <= cm[lag + 1] {
            best = Some(lag);
            break;
        }
    }
    let lag = best.or_else(|| (min_lag.max(2)..=max_lag).min_by(|a, b| cm[*a].partial_cmp(&cm[*b]).unwrap()))?;
    // parabolic refinement
    let (a, b, c) = (cm[lag - 1], cm[lag], cm[lag + 1]);
    let den = a - 2.0 * b + c;
    let off = if den.abs() > 1e-9 { 0.5 * (a - c) / den } else { 0.0 };
    Some(rate as f32 / (lag as f32 + off))
}

pub struct Prepared {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
    pub rate: u32,
    pub hz: f32,
    pub loop_start: usize,
    pub loop_end: usize,
}

/// Trim, measure, loop and crossfade one recorded note. `nominal` = its written pitch in Hz,
/// for the pitch search (the chanter is sharp of it by up to ~2 semitones).
pub fn prepare(mut l: Vec<f32>, mut r: Vec<f32>, rate: u32, nominal: f32) -> Result<Prepared, String> {
    let fr = (rate / 100) as usize; // 10 ms
    let env: Vec<f32> = l.chunks(fr).zip(r.chunks(fr)).map(|(a, b)| (a.iter().chain(b).map(|v| v * v).sum::<f32>() / (a.len() + b.len()) as f32).sqrt()).collect();
    // how loud the note is: the median of the frames that aren't near-silence
    let peak = env.iter().cloned().fold(0f32, f32::max);
    if peak < 1e-3 {
        return Err("silent recording".into());
    }
    let mut sounding: Vec<f32> = env.iter().cloned().filter(|e| *e > peak * 0.1).collect();
    sounding.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let loud = sounding[sounding.len() / 2];
    let on = env.iter().position(|e| *e > loud * 0.3).ok_or("no note found")? * fr;
    let off = (env.len() - env.iter().rev().position(|e| *e > loud * 0.3).unwrap_or(0)) * fr;
    let start = on.saturating_sub(fr);
    // the steady part: from 0.5 s in to 0.3 s before the end
    let s0 = on + rate as usize / 2;
    let s1 = off.saturating_sub(rate as usize * 3 / 10);
    if s1 < s0 + rate as usize * 6 / 5 {
        return Err(format!("the note is only {:.1} s long", (off - on) as f32 / rate as f32));
    }
    let mono: Vec<f32> = l.iter().zip(&r).map(|(a, b)| 0.5 * (a + b)).collect();
    let win = 8192.min((s1 - s0) / 2);
    let mut ests: Vec<f32> = (0..5)
        .filter_map(|i| {
            let at = s0 + (s1 - s0 - win) * i / 4;
            pitch(&mono[at..at + win], rate, nominal * 0.84, nominal * 1.4)
        })
        .collect();
    if ests.len() < 3 {
        return Err("couldn't hear a steady pitch".into());
    }
    ests.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let hz = ests[ests.len() / 2];
    // loop: up to 3 s of the steady part; the end where the waveform best matches the start
    let period = (rate as f32 / hz) as usize;
    let ls = s0;
    let span = (rate as usize * 3).min(s1 - s0 - period * 8);
    let target = ls + span;
    let cmp = period * 2;
    let mut best = (f32::MAX, target);
    for le in target - period * 3..target + period * 3 {
        let e: f32 = (0..cmp).map(|i| (mono[ls + i] - mono[le + i]).abs()).sum();
        if e < best.0 {
            best = (e, le);
        }
    }
    let le = best.1;
    // bake a 60 ms crossfade: the end of the loop fades into what comes just before its start
    let xf = (rate as f32 * 0.06) as usize;
    // through le itself: the last sample before the wrap becomes the one just before ls
    for i in 0..xf {
        let t = (i + 1) as f32 / xf as f32;
        let (a, b) = (le + 1 - xf + i, ls + 1 - xf + i - 1);
        l[a] = l[a] * (1.0 - t) + l[b] * t;
        r[a] = r[a] * (1.0 - t) + r[b] * t;
    }
    // keep from just before the note to the loop end; even out the level
    let mut l: Vec<f32> = l[start..le + 1].to_vec();
    let mut r: Vec<f32> = r[start..le + 1].to_vec();
    // a few samples past the loop end, continuing from the loop start, for the interpolator
    for i in 1..=64 {
        l.push(l[ls - start + i]);
        r.push(r[ls - start + i]);
    }
    let rms = (l[ls - start..].iter().chain(&r[ls - start..]).map(|v| v * v).sum::<f32>() / (2 * (le - ls)) as f32).sqrt();
    let g = 0.18 / rms.max(1e-4);
    let fade = (rate / 200) as usize;
    for i in 0..l.len() {
        let f = if i < fade { i as f32 / fade as f32 } else { 1.0 };
        l[i] = (l[i] * g * f).clamp(-1.0, 1.0);
        r[i] = (r[i] * g * f).clamp(-1.0, 1.0);
    }
    Ok(Prepared { l, r, rate, hz, loop_start: ls - start, loop_end: le - start })
}

/// Build the instrument from the nine recordings (`get` fetches a URL), into `out`.
/// Returns the .sfz path and a line per note (measured pitch, cents from the chanter's own scale).
pub fn build(out: &Path, get: &dyn Fn(&str) -> Result<Vec<u8>, String>, progress: &dyn Fn(usize, usize)) -> Result<(PathBuf, Vec<String>), String> {
    std::fs::create_dir_all(out).map_err(|e| e.to_string())?;
    let mut prepared = Vec::new();
    for (i, (id, name, key)) in NOTES.iter().enumerate() {
        progress(i, NOTES.len() + 1);
        let b = get(&url(*id))?;
        let (l, r, rate) = decode_mp3(b).map_err(|e| format!("{name}: {e}"))?;
        let nominal = 440.0 * 2f32.powf((*key as f32 - 69.0) / 12.0);
        let p = prepare(l, r, rate, nominal).map_err(|e| format!("{name}: {e}"))?;
        prepared.push((*name, *key, p));
    }
    // the chanter's offset from A = 440, taken from low A (the note the drones are tuned to)
    let low_a = prepared.iter().find(|p| p.0 == "Low A").ok_or("no low A")?.2.hz;
    let offset = 1200.0 * (low_a / 440.0).log2();
    let mut report = vec![format!("low A = {low_a:.1} Hz ({offset:+.0} ¢ from A440)")];
    let mut sfz = String::from("// Great Highland bagpipe chanter, recorded by IowaSpaceWizard (Freesound pack 29644), CC0.\n// Built by Noodle: looped, levelled, and lowered by the chanter's own offset so low A = A440.\n<control> default_path=samples/\n<global> loop_mode=loop_continuous ampeg_attack=0.004 ampeg_release=0.04\n");
    let keys: Vec<u8> = prepared.iter().map(|p| p.1).collect();
    for (i, (name, key, p)) in prepared.iter().enumerate() {
        let file = format!("{:02}-{}.wav", i, name.replace(' ', "_"));
        let spec = hound::WavSpec { channels: 2, sample_rate: p.rate, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        std::fs::create_dir_all(out.join("samples")).map_err(|e| e.to_string())?;
        let mut w = hound::WavWriter::create(out.join("samples").join(&file), spec).map_err(|e| e.to_string())?;
        for (a, b) in p.l.iter().zip(&p.r) {
            w.write_sample((a * 32767.0) as i16).map_err(|e| e.to_string())?;
            w.write_sample((b * 32767.0) as i16).map_err(|e| e.to_string())?;
        }
        w.finalize().map_err(|e| e.to_string())?;
        // the note's own tuning, relative to the chanter
        let own = 1200.0 * (p.hz / (440.0 * 2f32.powf((*key as f32 - 69.0) / 12.0))).log2() - offset;
        if own.abs() > 70.0 {
            return Err(format!("{name} came out {own:+.0} ¢ from where it should be: is that file the right note?"));
        }
        report.push(format!("{name}: {:.1} Hz, {own:+.0} ¢ against equal temperament", p.hz));
        let lo = if i == 0 { key.saturating_sub(7) } else { (keys[i - 1] + key) / 2 + 1 };
        let hi = if i + 1 == keys.len() { key + 7 } else { (key + keys[i + 1]) / 2 };
        sfz.push_str(&format!(
            "<region> sample={file} lokey={lo} hikey={hi} pitch_keycenter={key} tune={:.0} loop_start={} loop_end={}\n",
            -offset,
            p.loop_start,
            p.loop_end
        ));
    }
    progress(NOTES.len(), NOTES.len() + 1);
    let path = out.join("Great Highland chanter (recorded).sfz");
    std::fs::write(&path, sfz).map_err(|e| e.to_string())?;
    let _ = std::fs::write(out.join("CREDITS.txt"), format!("{CREDIT}\nhttps://freesound.org/people/IowaSpaceWizard/packs/29644/\nhttps://creativecommons.org/publicdomain/zero/1.0/\n\n{}\n", report.join("\n")));
    Ok((path, report))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pitch_finds_a_reedy_tone() {
        let rate = 44100;
        let f = 477.0;
        let x: Vec<f32> = (0..16384).map(|i| {
            let t = i as f32 / rate as f32;
            (1..12).map(|n| (std::f32::consts::TAU * f * n as f32 * t + n as f32).sin() / n as f32).sum()
        }).collect();
        let p = pitch(&x, rate, 380.0, 620.0).unwrap();
        assert!((p - f).abs() < 1.0, "{p}");
    }

    /// Downloads the nine recordings: `cargo test --release -- --ignored highland_live --nocapture`
    #[test]
    #[ignore]
    fn highland_live() {
        let out = std::env::temp_dir().join("noodle-highland-test");
        let _ = std::fs::remove_dir_all(&out);
        let get = |u: &str| -> Result<Vec<u8>, String> {
            if let Ok(dir) = std::env::var("NOODLE_HIGHLAND_DIR") {
                // local copies named <id>.mp3 (no network)
                let id = u.rsplit('/').next().unwrap().split('_').next().unwrap();
                return std::fs::read(format!("{dir}/{id}.mp3")).map_err(|e| e.to_string());
            }
            let mut r = ureq::get(u).call().map_err(|e| e.to_string())?;
            r.body_mut().with_config().limit(64 << 20).read_to_vec().map_err(|e| e.to_string())
        };
        // what actually comes back for one note
        for (id, name, _) in NOTES.iter().take(2) {
            match get(&url(*id)) {
                Ok(b) => {
                    let head: String = b.iter().take(16).map(|c| format!("{c:02x}")).collect();
                    let d = decode_mp3(b.clone());
                    match d {
                        Ok((l, _r, rate)) => {
                            let peak = l.iter().fold(0f32, |m, v| m.max(v.abs()));
                            let fr = rate as usize / 100;
                            let env: Vec<f32> = l.chunks(fr).map(|c| c.iter().map(|v| v.abs()).fold(0f32, f32::max)).collect();
                            let above: Vec<usize> = env.iter().enumerate().filter(|(_, e)| **e > peak * 0.2).map(|(i, _)| i).collect();
                            println!("{name}: {} bytes, {:.1} s at {rate} Hz, peak {peak:.3}, sound from {:.2} s to {:.2} s", b.len(), l.len() as f32 / rate as f32, above.first().copied().unwrap_or(0) as f32 / 100.0, above.last().copied().unwrap_or(0) as f32 / 100.0);
                        }
                        Err(e) => println!("{name}: {} bytes [{head}], decode failed: {e}", b.len()),
                    }
                }
                Err(e) => println!("{name}: fetch failed: {e}"),
            }
        }
        let (p, report) = build(&out, &get, &|_, _| {}).expect("build");
        for l in &report {
            println!("{l}");
        }
        let sfz = std::fs::read_to_string(&p).unwrap();
        println!("{sfz}");
        // loop seams: the jump at the wrap must be no bigger than an ordinary sample step
        for line in sfz.lines().filter(|l| l.starts_with("<region>")) {
            let get_op = |k: &str| line.split_whitespace().find_map(|w| w.strip_prefix(k)).unwrap().to_string();
            let file = get_op("sample=");
            let (ls, le): (usize, usize) = (get_op("loop_start=").parse().unwrap(), get_op("loop_end=").parse().unwrap());
            let mut rd = hound::WavReader::open(out.join("samples").join(&file)).unwrap();
            let l: Vec<i32> = rd.samples::<i16>().step_by(2).map(|v| v.unwrap() as i32).collect();
            let step = (ls..ls + 4000).map(|i| (l[i + 1] - l[i]).abs()).sum::<i32>() as f32 / 4000.0;
            let seam = (l[ls] - l[le]).abs();
            let ratio = seam as f32 / step.max(1.0);
            println!("seam {file}: {ratio:.2} × a normal step");
            assert!(ratio < 1.5, "{file} clicks at its loop");
        }
    }
}

