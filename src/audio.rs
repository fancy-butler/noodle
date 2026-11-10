// Noodle — audio output via cpal (ALSA/PipeWire/JACK on Linux, WASAPI on Windows).
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.
//
// The engine lives behind a mutex that only the audio callback normally touches
// (try_lock, never blocks). A watchdog thread checks that the sound card is
// actually pulling audio; if it stalls or never starts, the watchdog drives the
// engine itself in real time so MIDI, visuals and the controller panel keep
// working — you just don't hear anything — and it says so on screen.

use crate::engine::{Engine, Params};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};
use std::sync::atomic::Ordering::Relaxed;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CHUNK: usize = 2048;

pub type SharedEngine = Arc<Mutex<Engine>>;

pub struct Audio {
    pub _stream: cpal::Stream,
    pub sample_rate: u32,
    pub device: String,
    pub buffer: String,
}

pub fn default_rate() -> u32 {
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.default_output_config().ok())
        .map(|c| c.sample_rate().0)
        .unwrap_or(48000)
}

/// Open the default output and prove it's really running (the callback fires).
/// Tries a small low-latency buffer first, then the device's default.
pub fn start(engine: SharedEngine, p: Arc<Params>) -> Result<Audio, String> {
    let host = cpal::default_host();
    eprintln!("noodle: audio host = {:?}", host.id());
    let device = host.default_output_device().ok_or("no audio output device found")?;
    let name = device.name().unwrap_or_else(|_| "default".into());
    let sup = device.default_output_config().map_err(|e| e.to_string())?;
    let rate = sup.sample_rate().0;
    let fmt = sup.sample_format();
    eprintln!("noodle: audio device = {name}, {rate} Hz, {} ch, {fmt:?}", sup.channels());
    let mut last_err = String::from("audio stream never started");
    for (label, size) in [("256 frames", cpal::BufferSize::Fixed(256)), ("device default", cpal::BufferSize::Default)] {
        let mut cfg: cpal::StreamConfig = sup.config();
        cfg.buffer_size = size;
        let built = match fmt {
            cpal::SampleFormat::F32 => build::<f32>(&device, &cfg, engine.clone(), p.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, &cfg, engine.clone(), p.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, &cfg, engine.clone(), p.clone()),
            cpal::SampleFormat::I32 => build::<i32>(&device, &cfg, engine.clone(), p.clone()),
            other => Err(format!("unsupported sample format {other:?}")),
        };
        let stream = match built {
            Ok(s) => s,
            Err(e) => {
                eprintln!("noodle: audio buffer {label}: couldn't open ({e})");
                last_err = e;
                continue;
            }
        };
        if let Err(e) = stream.play() {
            eprintln!("noodle: audio buffer {label}: couldn't start ({e})");
            last_err = e.to_string();
            continue;
        }
        // wait for proof of life
        let before = p.audio_beats.load(Relaxed);
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(800) {
            if p.audio_beats.load(Relaxed) > before + 2 {
                eprintln!("noodle: audio running ({label})");
                return Ok(Audio { _stream: stream, sample_rate: rate, device: name, buffer: label.into() });
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        eprintln!("noodle: audio buffer {label}: stream opened but the sound card never asked for audio");
        last_err = "the sound card never asked for audio".into();
        drop(stream);
    }
    Err(last_err)
}

/// Keeps the engine ticking whenever the sound card isn't (no device, stalled
/// stream, device unplugged). Sets `p.audio_ok` for the UI.
pub fn spawn_watchdog(engine: SharedEngine, p: Arc<Params>, rate: u32) {
    std::thread::Builder::new()
        .name("noodle-watchdog".into())
        .spawn(move || {
            let n = 256usize;
            let mut l = vec![0.0f32; n];
            let mut r = vec![0.0f32; n];
            let dur = Duration::from_secs_f64(n as f64 / rate as f64);
            let mut last = p.audio_beats.load(Relaxed);
            let mut last_change = Instant::now();
            let mut silent = false;
            let mut next = Instant::now();
            loop {
                let beats = p.audio_beats.load(Relaxed);
                if beats != last {
                    last = beats;
                    last_change = Instant::now();
                    if silent {
                        eprintln!("noodle: audio is back");
                        silent = false;
                    }
                    p.audio_ok.store(true, Relaxed);
                } else if !silent && last_change.elapsed() > Duration::from_millis(400) {
                    eprintln!("noodle: audio isn't running — keeping the engine going silently so the screen still responds");
                    silent = true;
                    p.audio_ok.store(false, Relaxed);
                    next = Instant::now();
                }
                if silent {
                    {
                        let mut e = engine.lock().unwrap_or_else(|x| x.into_inner());
                        e.process(&mut l, &mut r);
                    }
                    next += dur;
                    let now = Instant::now();
                    if next > now {
                        std::thread::sleep(next - now);
                    } else {
                        next = now;
                    }
                } else {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        })
        .expect("spawn watchdog");
}

fn build<T>(device: &cpal::Device, cfg: &cpal::StreamConfig, engine: SharedEngine, p: Arc<Params>) -> Result<cpal::Stream, String>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = cfg.channels as usize;
    let mut l = vec![0.0f32; CHUNK];
    let mut r = vec![0.0f32; CHUNK];
    device
        .build_output_stream(
            cfg,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                p.audio_beats.fetch_add(1, Relaxed);
                let Ok(mut e) = engine.try_lock() else {
                    for s in data.iter_mut() {
                        *s = T::from_sample(0.0);
                    }
                    return;
                };
                let frames = data.len() / channels.max(1);
                let mut done = 0;
                while done < frames {
                    let n = (frames - done).min(CHUNK);
                    e.process(&mut l[..n], &mut r[..n]);
                    for i in 0..n {
                        let base = (done + i) * channels;
                        for c in 0..channels {
                            let v = if c % 2 == 0 { l[i] } else { r[i] };
                            data[base + c] = T::from_sample(v);
                        }
                    }
                    done += n;
                }
            },
            |err| eprintln!("noodle: audio error: {err}"),
            None,
        )
        .map_err(|e| e.to_string())
}
