// Noodle — audio output via cpal (ALSA/PipeWire/JACK on Linux, WASAPI on Windows).
// Copyright (C) 2026 Fancy Butler. GPL-3.0-or-later.

use crate::engine::Engine;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

const CHUNK: usize = 2048;

pub struct Audio {
    pub _stream: cpal::Stream,
    pub sample_rate: u32,
    pub device: String,
}

pub fn default_rate() -> u32 {
    cpal::default_host()
        .default_output_device()
        .and_then(|d| d.default_output_config().ok())
        .map(|c| c.sample_rate().0)
        .unwrap_or(48000)
}

pub fn start(engine: Engine) -> Result<Audio, (String, Option<Engine>)> {
    let host = cpal::default_host();
    let Some(device) = host.default_output_device() else { return Err(("No audio output device found".into(), Some(engine))) };
    let name = device.name().unwrap_or_else(|_| "default".into());
    let sup = match device.default_output_config() {
        Ok(s) => s,
        Err(e) => return Err((e.to_string(), Some(engine))),
    };
    let rate = sup.sample_rate().0;
    let fmt = sup.sample_format();
    let mut cfg: cpal::StreamConfig = sup.config();
    let try_build = |cfg: &cpal::StreamConfig, engine: Engine| -> Result<cpal::Stream, (String, Option<Engine>)> {
        match fmt {
            cpal::SampleFormat::F32 => build::<f32>(&device, cfg, engine),
            cpal::SampleFormat::I16 => build::<i16>(&device, cfg, engine),
            cpal::SampleFormat::U16 => build::<u16>(&device, cfg, engine),
            cpal::SampleFormat::I32 => build::<i32>(&device, cfg, engine),
            other => Err((format!("Unsupported sample format {other:?}"), Some(engine))),
        }
    };
    // Ask for a small buffer for low latency; fall back to the device default.
    let mut small = cfg.clone();
    small.buffer_size = cpal::BufferSize::Fixed(256);
    let stream = match try_build(&small, engine) {
        Ok(s) => s,
        Err((_, Some(engine))) => {
            cfg.buffer_size = cpal::BufferSize::Default;
            match try_build(&cfg, engine) {
                Ok(s) => s,
                Err(e) => return Err(e),
            }
        }
        Err(e) => return Err(e),
    };
    if let Err(e) = stream.play() {
        return Err((e.to_string(), None));
    }
    Ok(Audio { _stream: stream, sample_rate: rate, device: name })
}

/// No sound card? Keep the engine ticking in real time so the visuals,
/// looper and learning tools still work (silently).
pub fn run_silent(mut engine: Engine, rate: u32) {
    std::thread::Builder::new()
        .name("noodle-silent-clock".into())
        .spawn(move || {
            let n = 256usize;
            let mut l = vec![0.0f32; n];
            let mut r = vec![0.0f32; n];
            let dur = std::time::Duration::from_secs_f64(n as f64 / rate as f64);
            let mut next = std::time::Instant::now();
            loop {
                engine.process(&mut l, &mut r);
                next += dur;
                let now = std::time::Instant::now();
                if next > now {
                    std::thread::sleep(next - now);
                } else {
                    next = now;
                }
            }
        })
        .expect("spawn silent clock");
}

fn build<T>(device: &cpal::Device, cfg: &cpal::StreamConfig, engine: Engine) -> Result<cpal::Stream, (String, Option<Engine>)>
where
    T: SizedSample + FromSample<f32>,
{
    let channels = cfg.channels as usize;
    // The engine moves into the callback only if the stream builds; keep it in an Option
    // inside an Arc<Mutex> just for the hand-off (the lock is never contended).
    let slot = std::sync::Arc::new(std::sync::Mutex::new(Some(engine)));
    let slot_cb = slot.clone();
    let mut eng: Option<Engine> = None;
    let mut l = vec![0.0f32; CHUNK];
    let mut r = vec![0.0f32; CHUNK];
    let res = device.build_output_stream(
        cfg,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            if eng.is_none() {
                if let Ok(mut g) = slot_cb.try_lock() {
                    eng = g.take();
                }
            }
            let Some(e) = eng.as_mut() else {
                for s in data.iter_mut() {
                    *s = T::from_sample(0.0);
                }
                return;
            };
            let frames = data.len() / channels;
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
        |err| eprintln!("audio error: {err}"),
        None,
    );
    match res {
        Ok(s) => Ok(s),
        Err(e) => {
            let back = slot.lock().ok().and_then(|mut g| g.take());
            Err((e.to_string(), back))
        }
    }
}
