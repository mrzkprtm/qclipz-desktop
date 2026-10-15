// Audio capture through WASAPI (cpal): system sound is loopback of the default output
// device, the microphone is a normal input device. Each track is written as raw PCM to
// its own file, placed on a timeline shared with the video (`start`), and muxed in when
// the recording stops. Keeping audio out of the live ffmpeg process matters: real-time
// audio inputs make ffmpeg pace the screen grab to them, which dropped capture to ~5 fps.
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub enum Source {
    // Whatever plays on the default speakers or headphones.
    System,
    // A microphone (or other input) by name; empty = default input.
    Mic(String),
}

pub struct Track {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<std::io::Result<()>>>,
    path: PathBuf,
    format: &'static str,
    rate: u32,
    channels: u16,
}

// Formats whose silence is all-zero bytes (so padding is just zeros).
fn ffmpeg_format(f: cpal::SampleFormat) -> Option<&'static str> {
    match f {
        cpal::SampleFormat::F32 => Some("f32le"),
        cpal::SampleFormat::F64 => Some("f64le"),
        cpal::SampleFormat::I16 => Some("s16le"),
        cpal::SampleFormat::I32 => Some("s32le"),
        _ => None,
    }
}

fn name_of(d: &cpal::Device) -> String {
    d.description().map(|d| d.name().to_string()).unwrap_or_default()
}

// Name of the device system sound is recorded from.
pub fn output_name() -> Option<String> {
    cpal::default_host().default_output_device().map(|d| name_of(&d))
}

// Microphones and other capture devices, default first.
pub fn input_names() -> Vec<String> {
    let host = cpal::default_host();
    let default = host.default_input_device().map(|d| name_of(&d));
    let mut names: Vec<String> = host
        .input_devices()
        .map(|it| it.map(|d| name_of(&d)).filter(|n| !n.is_empty()).collect())
        .unwrap_or_default();
    names.sort_by_key(|n| Some(n) != default.as_ref());
    names.dedup();
    names
}

fn open(source: &Source) -> Result<(cpal::Device, cpal::SupportedStreamConfig), String> {
    let host = cpal::default_host();
    match source {
        Source::System => {
            let d = host.default_output_device().ok_or("no speakers or headphones to record system sound from")?;
            let c = d.default_output_config().map_err(|e| format!("system sound: {e}"))?;
            Ok((d, c))
        }
        Source::Mic(name) => {
            let d = if name.is_empty() {
                host.default_input_device()
            } else {
                host.input_devices().ok().and_then(|mut it| it.find(|d| name_of(d) == *name))
            }
            .ok_or_else(|| format!("microphone \"{name}\" not found. Plug it in, or pick another one"))?;
            let c = d.default_input_config().map_err(|e| format!("microphone: {e}"))?;
            Ok((d, c))
        }
    }
}

impl Track {
    // Start capturing into `path`. Byte 0 of the file is the moment `start`.
    pub fn start(source: &Source, path: PathBuf, start: Instant) -> Result<Track, String> {
        let (device, config) = open(source)?;
        let format = ffmpeg_format(config.sample_format())
            .ok_or_else(|| format!("unsupported audio sample format {:?}", config.sample_format()))?;
        let file = File::create(&path).map_err(|e| format!("create {}: {e}", path.display()))?;
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let (rate, channels) = (config.sample_rate(), config.channels());
        let thread = std::thread::spawn(move || run(device, config, BufWriter::new(file), start, flag));
        Ok(Track { stop, thread: Some(thread), path, format, rate, channels })
    }

    // ffmpeg input arguments that read this track's file.
    pub fn ffmpeg_input(&self) -> Vec<String> {
        vec![
            "-f".into(), self.format.into(),
            "-ar".into(), self.rate.to_string(),
            "-ac".into(), self.channels.to_string(),
            "-i".into(), self.path.to_string_lossy().into_owned(),
        ]
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    // Stop, fill silence up to now, and flush the file.
    pub fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Relaxed);
        match self.thread.take().map(|t| t.join()) {
            Some(Ok(Err(e))) => Err(format!("writing audio: {e}")),
            Some(Err(_)) => Err("audio capture crashed".into()),
            _ => Ok(()),
        }
    }
}

impl Drop for Track {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

fn run(
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    mut out: BufWriter<File>,
    start: Instant,
    stop: Arc<AtomicBool>,
) -> std::io::Result<()> {
    // Each packet carries the moment it was captured (WASAPI's QPC stamp), so it can be
    // placed exactly on the timeline instead of wherever it happened to arrive.
    let (tx, rx) = mpsc::channel::<(Instant, Vec<u8>)>();
    let stream = device.build_input_stream_raw(
        config.config(),
        config.sample_format(),
        move |data: &cpal::Data, info: &cpal::InputCallbackInfo| {
            let ts = info.timestamp();
            let age = ts.callback.saturating_duration_since(ts.capture);
            let captured = Instant::now().checked_sub(age).unwrap_or_else(Instant::now);
            let _ = tx.send((captured, data.bytes().to_vec()));
        },
        |_| {}, // device unplugged etc.: the silence fill below keeps the timeline going
        None,
    );
    // If the stream can't start, still write silence so the recording keeps its length.
    let _stream = stream.ok().inspect(|s| {
        let _ = s.play();
    });

    let frame = config.channels() as u64 * config.sample_format().sample_size() as u64;
    let rate = config.sample_rate() as f64;
    // Byte offset of a moment on the timeline, frame-aligned.
    let pos = |t: Instant| (t.saturating_duration_since(start).as_secs_f64() * rate) as u64 * frame;
    let ms = |n: u64| (rate * n as f64 / 1000.0) as u64 * frame;
    let tolerance = ms(20); // smaller jitter is written as-is to avoid clicks
    let silence_lag = Duration::from_millis(250); // stay behind "now" so late packets still fit
    let zeros = vec![0u8; ms(20) as usize];
    let mut written: u64 = 0;

    let pad = |out: &mut BufWriter<File>, written: &mut u64, to: u64| -> std::io::Result<()> {
        while *written < to {
            let n = (to - *written).min(zeros.len() as u64);
            out.write_all(&zeros[..n as usize])?;
            *written += n;
        }
        Ok(())
    };
    let place = |out: &mut BufWriter<File>, written: &mut u64, captured: Instant, buf: &[u8]| -> std::io::Result<()> {
        let target = pos(captured);
        let mut data = buf;
        if target > *written + tolerance {
            // Gap (playback just resumed): silence up to where this packet belongs.
            pad(out, written, target)?;
        } else if target + tolerance < *written {
            // Overlaps what's already written: drop the overlapping head.
            let skip = ((*written - target) as usize).min(data.len()) / frame as usize * frame as usize;
            data = &data[skip..];
        }
        out.write_all(data)?;
        *written += data.len() as u64;
        Ok(())
    };

    while !stop.load(Ordering::Relaxed) {
        if let Ok((captured, buf)) = rx.recv_timeout(Duration::from_millis(20)) {
            place(&mut out, &mut written, captured, &buf)?;
        }
        // Nothing playing means no packets at all; keep the timeline moving with silence.
        let now = Instant::now().checked_sub(silence_lag).unwrap_or(start);
        pad(&mut out, &mut written, pos(now))?;
    }
    // Drain what's left and run the timeline up to the stop moment.
    let end = Instant::now();
    while let Ok((captured, buf)) = rx.try_recv() {
        place(&mut out, &mut written, captured, &buf)?;
    }
    pad(&mut out, &mut written, pos(end))?;
    out.flush()
}
