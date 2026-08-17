//! Incremental, bounded compressed-audio decoding for generic MLPL consumers.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{Decoder, DecoderOptions};
use symphonia::core::errors::Error as SymphoniaError;
use symphonia::core::formats::{FormatOptions, FormatReader};
use symphonia::core::formats::{SeekMode, SeekTo};
use symphonia::core::io::{MediaSourceStream, MediaSourceStreamOptions};
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;

/// Discovers a bounded, deterministic set of MP3 and Ogg paths under a root.
/// Symlinks are not followed.
///
/// # Errors
///
/// Returns an error when the root or one of its contained directories cannot
/// be read or represented relative to the root.
pub fn discover_audio_paths(root: &Path, limit: usize) -> Result<Vec<String>, String> {
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let mut pending = vec![root.clone()];
    let mut paths = Vec::new();
    while let Some(directory) = pending.pop() {
        let mut entries = std::fs::read_dir(&directory)
            .map_err(|error| error.to_string())?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        entries.sort_by_key(std::fs::DirEntry::file_name);
        for entry in entries.into_iter().rev() {
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_symlink() {
                continue;
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                let extension = entry
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if matches!(extension.as_str(), "mp3" | "ogg") {
                    let relative = entry
                        .path()
                        .strip_prefix(&root)
                        .map_err(|error| error.to_string())?
                        .to_string_lossy()
                        .replace('\\', "/");
                    paths.push(relative);
                    if paths.len() == limit {
                        paths.sort();
                        return Ok(paths);
                    }
                }
            }
        }
    }
    paths.sort();
    Ok(paths)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodeLimits {
    pub max_frames_per_chunk: usize,
    pub max_channels: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcmChunk {
    pub left: Vec<f64>,
    pub right: Vec<f64>,
    pub sample_rate_hz: u32,
    pub start_frame: u64,
}

#[derive(Debug)]
pub struct PlaybackBuffer {
    samples: VecDeque<f32>,
    capacity: usize,
}

impl PlaybackBuffer {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        Self {
            samples: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push_stereo(&mut self, left: &[f64], right: &[f64], source_rate: u32, target_rate: u32) {
        if left.is_empty() || left.len() != right.len() || source_rate == 0 || target_rate == 0 {
            return;
        }
        let Ok(target) = usize::try_from(target_rate) else {
            return;
        };
        let Ok(source) = usize::try_from(source_rate) else {
            return;
        };
        let output_frames = left.len().saturating_mul(target).div_ceil(source);
        for output in 0..output_frames {
            let numerator = u64::try_from(output)
                .unwrap_or(u64::MAX)
                .saturating_mul(u64::from(source_rate));
            let lower = usize::try_from(numerator / u64::from(target_rate))
                .unwrap_or(usize::MAX)
                .min(left.len() - 1);
            let upper = (lower + 1).min(left.len() - 1);
            let remainder =
                u32::try_from(numerator % u64::from(target_rate)).unwrap_or(target_rate - 1);
            let blend = f64::from(remainder) / f64::from(target_rate);
            let l = left[lower] * (1.0 - blend) + left[upper] * blend;
            let r = right[lower] * (1.0 - blend) + right[upper] * blend;
            self.samples.extend([
                <f32 as cpal::FromSample<f64>>::from_sample_(l),
                <f32 as cpal::FromSample<f64>>::from_sample_(r),
            ]);
            while self.samples.len() > self.capacity {
                self.samples.pop_front();
            }
        }
    }

    pub fn fill(&mut self, output: &mut [f32]) {
        for sample in output {
            *sample = self.samples.pop_front().unwrap_or(0.0);
        }
    }

    fn pop_stereo(&mut self) -> [f32; 2] {
        [
            self.samples.pop_front().unwrap_or(0.0),
            self.samples.pop_front().unwrap_or(0.0),
        ]
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

pub struct PcmOutput {
    buffer: Arc<Mutex<PlaybackBuffer>>,
    stream: cpal::Stream,
    sample_rate_hz: u32,
}

impl PcmOutput {
    /// Opens the platform default output with a bounded stereo ring buffer.
    ///
    /// # Errors
    ///
    /// Returns an error when no default device/configuration is available.
    pub fn open(source_rate_hz: u32, max_frames: usize) -> Result<Self, String> {
        let device = cpal::default_host()
            .default_output_device()
            .ok_or_else(|| "no default audio output device".to_owned())?;
        let supported = device
            .supported_output_configs()
            .map_err(|error| error.to_string())?
            .find(|range| {
                range.channels() >= 2
                    && range.min_sample_rate().0 <= source_rate_hz
                    && range.max_sample_rate().0 >= source_rate_hz
                    && matches!(
                        range.sample_format(),
                        cpal::SampleFormat::F32 | cpal::SampleFormat::I16 | cpal::SampleFormat::U16
                    )
            })
            .map(|range| range.with_sample_rate(cpal::SampleRate(source_rate_hz)))
            .map_or_else(
                || {
                    device
                        .default_output_config()
                        .map_err(|error| error.to_string())
                },
                Ok,
            )?;
        let sample_rate_hz = supported.sample_rate().0;
        let config: cpal::StreamConfig = supported.clone().into();
        let channels = usize::from(config.channels);
        let buffer = Arc::new(Mutex::new(PlaybackBuffer::new(
            max_frames.saturating_mul(2),
        )));
        let stream = match supported.sample_format() {
            cpal::SampleFormat::F32 => {
                build_output::<f32>(&device, &config, channels, buffer.clone())?
            }
            cpal::SampleFormat::I16 => {
                build_output::<i16>(&device, &config, channels, buffer.clone())?
            }
            cpal::SampleFormat::U16 => {
                build_output::<u16>(&device, &config, channels, buffer.clone())?
            }
            format => return Err(format!("unsupported output sample format {format:?}")),
        };
        Ok(Self {
            buffer,
            stream,
            sample_rate_hz,
        })
    }

    pub fn enqueue(&self, chunk: &PcmChunk) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.push_stereo(
                &chunk.left,
                &chunk.right,
                chunk.sample_rate_hz,
                self.sample_rate_hz,
            );
        }
    }

    /// Starts or resumes the platform output stream.
    ///
    /// # Errors
    ///
    /// Returns the platform device error when the stream cannot be started.
    pub fn play(&self) -> Result<(), String> {
        self.stream.play().map_err(|error| error.to_string())
    }

    /// Pauses the platform output stream.
    ///
    /// # Errors
    ///
    /// Returns the platform device error when the stream cannot be paused.
    pub fn pause(&self) -> Result<(), String> {
        self.stream.pause().map_err(|error| error.to_string())
    }

    pub fn clear(&self) {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.clear();
        }
    }

    #[must_use]
    pub fn queued_frames(&self) -> usize {
        self.buffer.lock().map_or(0, |buffer| buffer.len() / 2)
    }
}

fn build_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    buffer: Arc<Mutex<PlaybackBuffer>>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _| {
                let mut guard = buffer.lock().ok();
                for frame in output.chunks_mut(channels) {
                    let [left, right] = guard
                        .as_mut()
                        .map_or([0.0, 0.0], |samples| samples.pop_stereo());
                    for (channel, sample) in frame.iter_mut().enumerate() {
                        *sample = T::from_sample(if channel == 0 { left } else { right });
                    }
                }
            },
            |error| eprintln!("audio output stream error: {error}"),
            None,
        )
        .map_err(|error| error.to_string())
}

pub struct PcmStream {
    format: Box<dyn FormatReader>,
    decoder: Box<dyn Decoder>,
    track_id: u32,
    sample_rate_hz: u32,
    limits: DecodeLimits,
    pending_left: Vec<f64>,
    pending_right: Vec<f64>,
    pending_offset: usize,
    emitted_frames: u64,
}

impl PcmStream {
    /// Opens an MP3 or Ogg/Vorbis source without reading its complete payload.
    ///
    /// # Errors
    ///
    /// Rejects invalid limits, inaccessible/unsupported files, and tracks
    /// without a declared sample rate.
    pub fn open(path: &Path, limits: DecodeLimits) -> Result<Self, String> {
        if limits.max_frames_per_chunk == 0 || !(1..=2).contains(&limits.max_channels) {
            return Err("audio decode limits must bound frames and one or two channels".into());
        }
        let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
        let stream = MediaSourceStream::new(Box::new(file), MediaSourceStreamOptions::default());
        let mut hint = Hint::new();
        if let Some(extension) = path.extension().and_then(|value| value.to_str()) {
            hint.with_extension(extension);
        }
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                stream,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|error| error.to_string())?;
        let track = probed
            .format
            .default_track()
            .ok_or_else(|| "audio source has no default track".to_owned())?;
        let sample_rate_hz = track
            .codec_params
            .sample_rate
            .ok_or_else(|| "audio track has no sample rate".to_owned())?;
        let track_id = track.id;
        let decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|error| error.to_string())?;
        Ok(Self {
            format: probed.format,
            decoder,
            track_id,
            sample_rate_hz,
            limits,
            pending_left: Vec::new(),
            pending_right: Vec::new(),
            pending_offset: 0,
            emitted_frames: 0,
        })
    }

    /// Decodes at most the configured number of stereo frames.
    ///
    /// # Errors
    ///
    /// Returns malformed packet and decoder failures. End-of-stream is `Ok(None)`.
    pub fn next_chunk(&mut self) -> Result<Option<PcmChunk>, String> {
        if self.pending_offset >= self.pending_left.len() && !self.decode_packet()? {
            return Ok(None);
        }
        let ending =
            (self.pending_offset + self.limits.max_frames_per_chunk).min(self.pending_left.len());
        let left = self.pending_left[self.pending_offset..ending].to_vec();
        let right = self.pending_right[self.pending_offset..ending].to_vec();
        self.pending_offset = ending;
        let start_frame = self.emitted_frames;
        self.emitted_frames += u64::try_from(left.len()).map_err(|error| error.to_string())?;
        Ok(Some(PcmChunk {
            left,
            right,
            sample_rate_hz: self.sample_rate_hz,
            start_frame,
        }))
    }

    #[must_use]
    pub const fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    /// Seeks to an absolute media time and discards any buffered PCM.
    ///
    /// # Errors
    ///
    /// Returns a format error when the stream is not seekable or the target
    /// cannot be resolved.
    pub fn seek_seconds(&mut self, seconds: f64) -> Result<(), String> {
        if !seconds.is_finite() || seconds < 0.0 {
            return Err("audio seek time must be finite and nonnegative".into());
        }
        let seeked = self
            .format
            .seek(
                SeekMode::Accurate,
                SeekTo::Time {
                    time: Time::from(seconds),
                    track_id: Some(self.track_id),
                },
            )
            .map_err(|error| error.to_string())?;
        self.decoder.reset();
        self.pending_left.clear();
        self.pending_right.clear();
        self.pending_offset = 0;
        self.emitted_frames = seeked.actual_ts;
        Ok(())
    }

    fn decode_packet(&mut self) -> Result<bool, String> {
        loop {
            let packet = match self.format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::ResetRequired) => {
                    self.decoder.reset();
                    continue;
                }
                Err(SymphoniaError::IoError(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    return Ok(false);
                }
                Err(error) => return Err(error.to_string()),
            };
            if packet.track_id() != self.track_id {
                continue;
            }
            let decoded = self
                .decoder
                .decode(&packet)
                .map_err(|error| error.to_string())?;
            let channels = decoded.spec().channels.count();
            if channels == 0 {
                continue;
            }
            let mut samples = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
            samples.copy_interleaved_ref(decoded);
            self.pending_left.clear();
            self.pending_right.clear();
            for frame in samples.samples().chunks(channels) {
                self.pending_left.push(f64::from(frame[0]));
                let right = if self.limits.max_channels == 1 || channels == 1 {
                    frame[0]
                } else {
                    frame[1]
                };
                self.pending_right.push(f64::from(right));
            }
            self.pending_offset = 0;
            if !self.pending_left.is_empty() {
                return Ok(true);
            }
        }
    }
}
