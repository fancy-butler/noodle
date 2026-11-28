//! Recorded drones: one stereo loop of a whole drone set (all drones together), repitched to the key.

use std::path::Path;

pub struct DroneLoop {
    pub l: Vec<f32>,
    pub r: Vec<f32>,
    pub loop_start: usize,
    pub loop_end: usize,
    pub rate: f32,
    /// the pitch the recording sounds at, as absolute cents (MIDI note × 100) of its top drone
    pub cents: f64,
}

/// Load the loop from its one-region .sfz (sample, loop points, pitch_keycenter, tune).
pub fn load(sfz: &Path) -> Result<DroneLoop, String> {
    load_regions(sfz)?.into_iter().next().map(|r| r.2).ok_or_else(|| "no regions".into())
}

/// Every region of a simple looped .sfz: (group, key, loop). Supports sample, key/pitch_keycenter,
/// tune, loop_start, loop_end, group and <control> default_path — all a region needs here.
pub fn load_regions(sfz: &Path) -> Result<Vec<(u32, u8, DroneLoop)>, String> {
    let text = std::fs::read_to_string(sfz).map_err(|e| e.to_string())?;
    let mut dpath = String::new();
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.split("//").next().unwrap_or("");
        if !line.contains("<region>") {
            for (k, v) in crate::sampler::opcodes_pub(&line.replace(['<', '>'], " ")) {
                if k == "default_path" {
                    dpath = v;
                }
            }
            continue;
        }
        let (mut sample, mut ls, mut le, mut key, mut tune, mut group) = (String::new(), 0usize, 0usize, 62.0f64, 0.0f64, 0u32);
        for (k, v) in crate::sampler::opcodes_pub(&line.replace("<region>", " ")) {
            match k.as_str() {
                "sample" => sample = v,
                "loop_start" => ls = v.parse().unwrap_or(0),
                "loop_end" => le = v.parse().unwrap_or(0),
                "pitch_keycenter" | "key" => key = v.parse().unwrap_or(62.0),
                "tune" => tune = v.parse().unwrap_or(0.0),
                "group" => group = v.parse().unwrap_or(0),
                _ => {}
            }
        }
        let wav = sfz.parent().unwrap_or(Path::new(".")).join(format!("{dpath}{sample}"));
        out.push((group, key as u8, read(&wav, ls, le, key * 100.0 - tune)?));
    }
    Ok(out)
}

fn read(wav: &Path, ls: usize, mut le: usize, cents: f64) -> Result<DroneLoop, String> {
    let mut rd = hound::WavReader::open(wav).map_err(|e| format!("{}: {e}", wav.display()))?;
    let spec = rd.spec();
    let ch = spec.channels as usize;
    let xs: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Int => {
            let s = (1i64 << (spec.bits_per_sample - 1)) as f32;
            rd.samples::<i32>().map(|x| x.unwrap_or(0) as f32 / s).collect()
        }
        hound::SampleFormat::Float => rd.samples::<f32>().map(|x| x.unwrap_or(0.0)).collect(),
    };
    let n = xs.len() / ch.max(1);
    let l: Vec<f32> = (0..n).map(|i| xs[i * ch]).collect();
    let r: Vec<f32> = (0..n).map(|i| xs[i * ch + (ch > 1) as usize]).collect();
    if le == 0 || le + 2 >= n {
        le = n.saturating_sub(3);
    }
    if ls >= le {
        return Err(format!("{}: bad loop points", wav.display()));
    }
    // the recording sounds `tune` cents away from its key (tune corrects it to the key)
    Ok(DroneLoop { l, r, loop_start: ls, loop_end: le, rate: spec.sample_rate as f32, cents })
}

/// Recorded regulators: the looped notes of each regulator (0 tenor, 1 baritone, 2 bass), by key.
pub struct RegSet {
    pub rows: [Vec<(u8, DroneLoop)>; 3],
}

pub fn load_regs(sfz: &Path) -> Result<RegSet, String> {
    let mut rows: [Vec<(u8, DroneLoop)>; 3] = Default::default();
    for (g, k, d) in load_regions(sfz)? {
        if (1..=3).contains(&g) {
            rows[g as usize - 1].push((k, d));
        }
    }
    if rows.iter().any(|r| r.is_empty()) {
        return Err("regulator set needs groups 1–3".into());
    }
    Ok(RegSet { rows })
}

/// One regulator's player: the sounding note, plus the one it's crossfading away from.
#[derive(Default)]
pub struct RegPlay {
    cur: Option<(usize, f64)>,
    /// (index, position, step)
    old: Option<(usize, f64, f64)>,
    xf: f32,
    gain: f32,
    target: f32,
    cents: f64,
}

impl RegPlay {
    /// `note` = (sample index in the row, cents to sound at), None = keys up.
    pub fn render(&mut self, row: &[(u8, DroneLoop)], note: Option<(usize, f64)>, level: f32, pan: f32, sr: f32, out_l: &mut [f32], out_r: &mut [f32]) {
        match note {
            Some((idx, cents)) => {
                let same = self.cur.map(|c| c.0) == Some(idx);
                if !same {
                    if self.gain > 0.02 {
                        // already sounding: a key change on a stopped pipe — no new attack, a quick crossfade
                        if let Some((ci, cp)) = self.cur {
                            let st = 2f64.powf((self.cents - row[ci].1.cents) / 1200.0) * row[ci].1.rate as f64 / sr as f64;
                            self.old = Some((ci, cp, st));
                            self.xf = 1.0;
                        }
                        self.cur = Some((idx, row[idx].1.loop_start as f64));
                    } else {
                        self.cur = Some((idx, 0.0));
                        self.old = None;
                    }
                }
                self.cents = cents;
                self.target = 1.0;
            }
            None => self.target = 0.0,
        }
        if self.target == 0.0 && self.gain < 1e-4 {
            self.gain = 0.0;
            self.cur = None;
            self.old = None;
            return;
        }
        let att = 1.0 - (-1.0 / (0.004 * sr)).exp();
        let rel = 1.0 - (-1.0 / (0.025 * sr)).exp();
        let xf_step = 1.0 / (0.012 * sr);
        let step_of = |d: &DroneLoop, c: f64| 2f64.powf((c - d.cents) / 1200.0) * d.rate as f64 / sr as f64;
        let Some((ci, mut cp)) = self.cur else { return };
        let cd = &row[ci].1;
        let cs = step_of(cd, self.cents);
        let mut old = self.old;
        for i in 0..out_l.len() {
            self.gain += (self.target - self.gain) * if self.target > self.gain { att } else { rel };
            let (mut yl, mut yr) = tap(cd, &mut cp, cs);
            if let Some((oi, op, os)) = old.as_mut() {
                let (ol, or) = tap(&row[*oi].1, op, *os);
                yl = yl * (1.0 - self.xf) + ol * self.xf;
                yr = yr * (1.0 - self.xf) + or * self.xf;
                self.xf -= xf_step;
                if self.xf <= 0.0 {
                    self.xf = 0.0;
                    old = None;
                }
            }
            let g = self.gain * level;
            out_l[i] += yl * g * (1.0 - pan);
            out_r[i] += yr * g * (1.0 + pan);
        }
        self.cur = Some((ci, cp));
        self.old = old;
    }
}

fn tap(d: &DroneLoop, pos: &mut f64, step: f64) -> (f32, f32) {
    let p = *pos as usize;
    let f = (*pos - p as f64) as f32;
    let (a, b) = (p.min(d.l.len() - 1), (p + 1).min(d.l.len() - 1));
    let y = (d.l[a] + (d.l[b] - d.l[a]) * f, d.r[a] + (d.r[b] - d.r[a]) * f);
    *pos += step;
    let (ls, le) = (d.loop_start as f64, d.loop_end as f64);
    if *pos >= le + 1.0 {
        *pos -= le + 1.0 - ls;
    }
    y
}

/// Playback state on the audio thread.
#[derive(Default)]
pub struct DronePlay {
    pub pos: f64,
    pub gain: f32,
}

impl DronePlay {
    /// Mix the loop into the output at `cents` (absolute, top drone), fading in/out with `on`.
    pub fn render(&mut self, d: &DroneLoop, on: bool, cents: f64, level: f32, sr: f32, out_l: &mut [f32], out_r: &mut [f32]) {
        if !on && self.gain < 1e-4 {
            self.gain = 0.0;
            self.pos = 0.0;
            return;
        }
        let step = 2f64.powf((cents - d.cents) / 1200.0) * d.rate as f64 / sr as f64;
        let attack = 1.0 - (-1.0 / (0.15 * sr)).exp();
        let release = 1.0 - (-1.0 / (0.35 * sr)).exp();
        let target = if on { 1.0 } else { 0.0 };
        let (ls, le) = (d.loop_start as f64, d.loop_end as f64);
        for i in 0..out_l.len() {
            self.gain += (target - self.gain) * if target > self.gain { attack } else { release };
            let p = self.pos as usize;
            let f = (self.pos - p as f64) as f32;
            let (a, b) = (p.min(d.l.len() - 1), (p + 1).min(d.l.len() - 1));
            let g = self.gain * level;
            out_l[i] += (d.l[a] + (d.l[b] - d.l[a]) * f) * g;
            out_r[i] += (d.r[a] + (d.r[b] - d.r[a]) * f) * g;
            self.pos += step;
            // loop_end is the last sample played before loop_start (sfz)
            if self.pos >= le + 1.0 {
                self.pos -= le + 1.0 - ls;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn shipped_loop_loads_and_is_in_tune() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sounds/uilleann-drones-2/Uilleann drones (recorded).sfz");
        let d = super::load(&p).unwrap();
        assert!(d.loop_end > d.loop_start + 44100, "loop too short");
        assert!((d.cents - 6210.0).abs() < 5.0, "recorded pitch {}", d.cents);
        // seam: the jump across the loop is no bigger than the biggest ordinary step nearby
        assert_eq!(d.l[d.loop_end + 1], d.l[d.loop_start], "tail past the loop end must repeat the loop start");
        let seam = (d.l[d.loop_start] - d.l[d.loop_end]).abs();
        let near = (d.loop_start + 1..d.loop_start + 2000).map(|i| (d.l[i] - d.l[i - 1]).abs()).fold(0f32, f32::max);
        assert!(seam <= near * 1.2, "seam {seam} vs {near}");
    }

    #[test]
    fn shipped_regulators_load() {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("sounds/uilleann-regulators/Uilleann regulators (recorded).sfz");
        let set = super::load_regs(&p).unwrap();
        // every key on each regulator (pipes::REGS, D set) has its note
        for (r, (_, keys)) in crate::pipes::REGS.iter().enumerate() {
            for k in keys.iter() {
                let key = (62 + *k as i32) as u8;
                assert!(set.rows[r].iter().any(|x| x.0 == key), "regulator {r} missing key {key}");
            }
        }
        for row in &set.rows {
            for (k, d) in row {
                assert_eq!(d.l[d.loop_end + 1], d.l[d.loop_start], "key {k} loop tail");
                assert!((d.cents - *k as f64 * 100.0).abs() < 40.0, "key {k} at {}", d.cents);
            }
        }
    }
}
