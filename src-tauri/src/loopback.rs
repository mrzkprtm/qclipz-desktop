// System sound: WASAPI loopback of the default output device (whatever plays on
// the speakers or headphones), streamed to ffmpeg as raw PCM over a local socket.
use std::io::Write;
use std::net::TcpListener;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct Source {
    device: cpal::Device,
    config: cpal::SupportedStreamConfig,
    listener: TcpListener,
    // Sample 0 of the stream is this moment; `epoch_us` is the same moment on the wall clock,
    // which ffmpeg uses to line the audio up with the video.
    start: Instant,
    epoch_us: i64,
}

pub struct Loopback {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
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

// Name of the device system sound is recorded from.
pub fn output_name() -> Option<String> {
    let device = cpal::default_host().default_output_device()?;
    device.description().ok().map(|d| d.name().to_string())
}

impl Source {
    pub fn open() -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or("no speakers or headphones to record system sound from")?;
        let config = device.default_output_config().map_err(|e| format!("system sound: {e}"))?;
        if ffmpeg_format(config.sample_format()).is_none() {
            return Err(format!("system sound: unsupported sample format {:?}", config.sample_format()));
        }
        let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| format!("system sound: {e}"))?;
        let epoch_us = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_micros() as i64)
            .unwrap_or(0);
        Ok(Self { device, config, listener, start: Instant::now(), epoch_us })
    }

    pub fn epoch_us(&self) -> i64 {
        self.epoch_us
    }

    // ffmpeg input arguments that read this stream.
    pub fn ffmpeg_input(&self) -> Vec<String> {
        let port = self.listener.local_addr().map(|a| a.port()).unwrap_or(0);
        [
            "-thread_queue_size", "1024",
            "-f", ffmpeg_format(self.config.sample_format()).unwrap_or("f32le"),
            "-ar", &self.config.sample_rate().to_string(),
            "-ac", &self.config.channels().to_string(),
            "-i", &format!("tcp://127.0.0.1:{port}"),
        ]
        .map(String::from)
        .to_vec()
    }

    pub fn start(self) -> Loopback {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let thread = std::thread::spawn(move || run(self, flag));
        Loopback { stop, thread: Some(thread) }
    }
}

impl Loopback {
    pub fn stop(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

fn run(src: Source, stop: Arc<AtomicBool>) {
    // Wait for ffmpeg to connect.
    let _ = src.listener.set_nonblocking(true);
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut sock = loop {
        match src.listener.accept() {
            Ok((s, _)) => break s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if stop.load(Ordering::Relaxed) || Instant::now() > deadline {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return,
        }
    };
    let _ = sock.set_nonblocking(false);
    let _ = sock.set_nodelay(true);

    // Each packet carries the moment it was captured (WASAPI's QPC stamp), so it can be
    // placed exactly on the timeline instead of wherever it happened to arrive.
    let (tx, rx) = mpsc::channel::<(Instant, Vec<u8>)>();
    let stream = src.device.build_input_stream_raw(
        src.config.config(),
        src.config.sample_format(),
        move |data: &cpal::Data, info: &cpal::InputCallbackInfo| {
            let ts = info.timestamp();
            let age = ts.callback.saturating_duration_since(ts.capture);
            let captured = Instant::now().checked_sub(age).unwrap_or_else(Instant::now);
            let _ = tx.send((captured, data.bytes().to_vec()));
        },
        |_| {}, // device unplugged etc.: the silence fill below keeps the timeline going
        None,
    );
    // If the stream can't start, keep feeding silence so the recording still works.
    let _stream = stream.ok().inspect(|s| {
        let _ = s.play();
    });

    let frame = src.config.channels() as u64 * src.config.sample_format().sample_size() as u64;
    let rate = src.config.sample_rate() as f64;
    let start = src.start;
    // Byte offset of a moment on the stream's timeline, frame-aligned.
    let pos = |t: Instant| (t.saturating_duration_since(start).as_secs_f64() * rate) as u64 * frame;
    let ms = |n: u64| (rate * n as f64 / 1000.0) as u64 * frame;
    let tolerance = ms(20); // smaller jitter is written as-is to avoid clicks
    let silence_lag = Duration::from_millis(150); // stay behind "now" so late packets still fit
    let zeros = vec![0u8; ms(20) as usize];
    let mut written: u64 = 0;

    let pad = |sock: &mut std::net::TcpStream, written: &mut u64, to: u64| -> bool {
        while *written < to {
            let n = (to - *written).min(zeros.len() as u64);
            if sock.write_all(&zeros[..n as usize]).is_err() {
                return false;
            }
            *written += n;
        }
        true
    };

    while !stop.load(Ordering::Relaxed) {
        if let Ok((captured, buf)) = rx.recv_timeout(Duration::from_millis(10)) {
            let target = pos(captured);
            let mut data = &buf[..];
            if target > written + tolerance {
                // Gap (playback just resumed): silence up to where this packet belongs.
                if !pad(&mut sock, &mut written, target) {
                    return;
                }
            } else if target + tolerance < written {
                // Overlaps what's already written: drop the overlapping head.
                let skip = ((written - target) as usize).min(data.len()) / frame as usize * frame as usize;
                data = &data[skip..];
            }
            if sock.write_all(data).is_err() {
                return; // ffmpeg finished
            }
            written += data.len() as u64;
        }
        // Nothing playing means no packets at all; keep the timeline moving with silence.
        let now = Instant::now().checked_sub(silence_lag).unwrap_or(start);
        if !pad(&mut sock, &mut written, pos(now)) {
            return;
        }
    }
}
