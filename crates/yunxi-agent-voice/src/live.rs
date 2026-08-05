use crate::{AudioInput, VoiceError, VoiceResult, validate_wav};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioDeviceSummary {
    pub default_input: Option<String>,
    pub default_output: Option<String>,
    pub input_devices: Vec<String>,
}

#[derive(Debug)]
pub struct CapturedAudio {
    pub input: AudioInput,
    pub device_name: String,
    pub sample_rate: u32,
    pub duration_ms: u128,
    pub reached_limit: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VoiceActivityConfig {
    pub rms_threshold: u16,
    pub minimum_speech_ms: u32,
    pub trailing_silence_ms: u32,
    pub idle_timeout_ms: u32,
    pub pre_roll_ms: u32,
    pub tail_ms: u32,
}

#[derive(Clone, Debug, Default)]
pub struct PlaybackCancellationToken {
    cancelled: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

impl PlaybackCancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(std::sync::atomic::Ordering::Acquire)
    }
}

impl Default for VoiceActivityConfig {
    fn default() -> Self {
        Self {
            rms_threshold: 650,
            minimum_speech_ms: 300,
            trailing_silence_ms: 850,
            idle_timeout_ms: 30_000,
            pre_roll_ms: 250,
            tail_ms: 300,
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    use cpal::{FromSample, Sample, SizedSample};
    use std::io::Cursor;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    const ACTIVITY_POLL_MS: u64 = 50;

    pub struct LiveRecording {
        stream: cpal::Stream,
        samples: Arc<Mutex<Vec<i16>>>,
        stream_error: Arc<Mutex<Option<String>>>,
        reached_limit: Arc<AtomicBool>,
        device_name: String,
        sample_rate: u32,
    }

    impl LiveRecording {
        pub fn start(input_device: Option<&str>, maximum_seconds: u32) -> VoiceResult<Self> {
            if maximum_seconds == 0 {
                return Err(VoiceError::InvalidConfiguration(
                    "maximum recording duration must be positive".to_string(),
                ));
            }
            let host = cpal::default_host();
            let device = select_input_device(&host, input_device)?;
            let device_name = device
                .name()
                .map_err(|error| VoiceError::AudioDevice(error.to_string()))?;
            let supported = device
                .default_input_config()
                .map_err(|error| VoiceError::AudioDevice(error.to_string()))?;
            let config = supported.config();
            let channels = usize::from(config.channels);
            let sample_rate = config.sample_rate.0;
            let maximum_samples = usize::try_from(sample_rate)
                .unwrap_or(usize::MAX)
                .saturating_mul(usize::try_from(maximum_seconds).unwrap_or(usize::MAX));
            let samples = Arc::new(Mutex::new(Vec::with_capacity(
                maximum_samples.min(1_920_000),
            )));
            let stream_error = Arc::new(Mutex::new(None));
            let reached_limit = Arc::new(AtomicBool::new(false));

            let stream = match supported.sample_format() {
                cpal::SampleFormat::I8 => build_stream::<i8>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::I16 => build_stream::<i16>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::I32 => build_stream::<i32>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::I64 => build_stream::<i64>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::U8 => build_stream::<u8>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::U16 => build_stream::<u16>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::U32 => build_stream::<u32>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::U64 => build_stream::<u64>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::F32 => build_stream::<f32>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                cpal::SampleFormat::F64 => build_stream::<f64>(
                    &device,
                    &config,
                    channels,
                    maximum_samples,
                    Arc::clone(&samples),
                    Arc::clone(&stream_error),
                    Arc::clone(&reached_limit),
                ),
                format => Err(VoiceError::AudioDevice(format!(
                    "unsupported microphone sample format {format}"
                ))),
            }?;
            stream
                .play()
                .map_err(|error| VoiceError::AudioDevice(error.to_string()))?;

            Ok(Self {
                stream,
                samples,
                stream_error,
                reached_limit,
                device_name,
                sample_rate,
            })
        }

        pub fn finish(self, maximum_bytes: usize) -> VoiceResult<CapturedAudio> {
            self.finish_window(maximum_bytes, None)
        }

        pub fn capture_until_silence(
            self,
            maximum_bytes: usize,
            config: VoiceActivityConfig,
        ) -> VoiceResult<Option<CapturedAudio>> {
            self.capture_until_silence_with_control(maximum_bytes, config, || false)
        }

        pub fn capture_until_silence_with_control(
            self,
            maximum_bytes: usize,
            config: VoiceActivityConfig,
            mut should_cancel: impl FnMut() -> bool,
        ) -> VoiceResult<Option<CapturedAudio>> {
            validate_voice_activity_config(config)?;
            let mut tracker = VoiceActivityTracker::new(self.sample_rate, config);
            let mut observed_samples = 0usize;

            loop {
                if should_cancel() {
                    return Ok(None);
                }
                thread::sleep(Duration::from_millis(ACTIVITY_POLL_MS));
                let window = self.activity_since(observed_samples)?;
                if window.end_sample == observed_samples {
                    continue;
                }
                observed_samples = window.end_sample;
                let active = window.rms >= u32::from(config.rms_threshold);
                match tracker.observe(window.start_sample, window.end_sample, active) {
                    VoiceActivityDecision::Continue => {}
                    VoiceActivityDecision::IdleTimeout => return Ok(None),
                    VoiceActivityDecision::Capture { start, end } => {
                        return self
                            .finish_window(maximum_bytes, Some((start, end)))
                            .map(Some);
                    }
                }

                if self.reached_limit.load(Ordering::Relaxed) {
                    return match tracker.finish_at_limit(observed_samples) {
                        Some((start, end)) => self
                            .finish_window(maximum_bytes, Some((start, end)))
                            .map(Some),
                        None => Ok(None),
                    };
                }
            }
        }

        fn activity_since(&self, start_sample: usize) -> VoiceResult<ActivityWindow> {
            if let Some(error) = self
                .stream_error
                .lock()
                .map_err(|_| VoiceError::AudioDevice("audio error lock was poisoned".to_string()))?
                .clone()
            {
                return Err(VoiceError::AudioDevice(error));
            }
            let samples = self.samples.lock().map_err(|_| {
                VoiceError::AudioDevice("audio sample lock was poisoned".to_string())
            })?;
            let end_sample = samples.len();
            let start_sample = start_sample.min(end_sample);
            let window = &samples[start_sample..end_sample];
            let rms = if window.is_empty() {
                0
            } else {
                let sum_of_squares = window.iter().fold(0_u64, |sum, sample| {
                    let sample = i64::from(*sample);
                    sum.saturating_add((sample * sample) as u64)
                });
                let mean = sum_of_squares / window.len() as u64;
                (mean as f64).sqrt().round() as u32
            };
            Ok(ActivityWindow {
                start_sample,
                end_sample,
                rms,
            })
        }

        fn finish_window(
            self,
            maximum_bytes: usize,
            window: Option<(usize, usize)>,
        ) -> VoiceResult<CapturedAudio> {
            drop(self.stream);
            if let Some(error) = self
                .stream_error
                .lock()
                .map_err(|_| VoiceError::AudioDevice("audio error lock was poisoned".to_string()))?
                .clone()
            {
                return Err(VoiceError::AudioDevice(error));
            }
            let samples = self
                .samples
                .lock()
                .map_err(|_| VoiceError::AudioDevice("audio sample lock was poisoned".to_string()))?
                .clone();
            let (start, end) = window
                .map(|(start, end)| (start.min(samples.len()), end.min(samples.len())))
                .unwrap_or((0, samples.len()));
            let samples = samples
                .get(start..end)
                .ok_or_else(|| VoiceError::InvalidInput("invalid VAD sample window".to_string()))?;
            let minimum_samples = usize::try_from(self.sample_rate / 5).unwrap_or(usize::MAX);
            if samples.len() < minimum_samples {
                return Err(VoiceError::InvalidInput(
                    "recording is shorter than 200 ms".to_string(),
                ));
            }
            let duration_ms = (samples.len() as u128)
                .saturating_mul(1_000)
                .checked_div(u128::from(self.sample_rate))
                .unwrap_or_default();
            let bytes = encode_mono_wav(&samples, self.sample_rate)?;
            let input = AudioInput::wav(bytes, maximum_bytes)?;
            Ok(CapturedAudio {
                input,
                device_name: self.device_name,
                sample_rate: self.sample_rate,
                duration_ms,
                reached_limit: self.reached_limit.load(Ordering::Relaxed),
            })
        }
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    struct ActivityWindow {
        start_sample: usize,
        end_sample: usize,
        rms: u32,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum VoiceActivityDecision {
        Continue,
        IdleTimeout,
        Capture { start: usize, end: usize },
    }

    #[derive(Clone, Copy, Debug)]
    struct VoiceActivityTracker {
        sample_rate: u32,
        config: VoiceActivityConfig,
        speech_start: Option<usize>,
        last_active: Option<usize>,
    }

    impl VoiceActivityTracker {
        fn new(sample_rate: u32, config: VoiceActivityConfig) -> Self {
            Self {
                sample_rate,
                config,
                speech_start: None,
                last_active: None,
            }
        }

        fn observe(
            &mut self,
            window_start: usize,
            window_end: usize,
            active: bool,
        ) -> VoiceActivityDecision {
            if active {
                self.speech_start.get_or_insert(window_start);
                self.last_active = Some(window_end);
            }

            let Some(speech_start) = self.speech_start else {
                return if window_end >= self.samples_for_ms(self.config.idle_timeout_ms) {
                    VoiceActivityDecision::IdleTimeout
                } else {
                    VoiceActivityDecision::Continue
                };
            };
            let last_active = self.last_active.unwrap_or(speech_start);
            let active_span = last_active.saturating_sub(speech_start);
            let silence = window_end.saturating_sub(last_active);
            let silence_limit = self.samples_for_ms(self.config.trailing_silence_ms);
            if silence < silence_limit {
                return VoiceActivityDecision::Continue;
            }
            if active_span < self.samples_for_ms(self.config.minimum_speech_ms) {
                self.speech_start = None;
                self.last_active = None;
                return VoiceActivityDecision::Continue;
            }
            let start = speech_start.saturating_sub(self.samples_for_ms(self.config.pre_roll_ms));
            let end = last_active
                .saturating_add(self.samples_for_ms(self.config.tail_ms))
                .min(window_end);
            VoiceActivityDecision::Capture { start, end }
        }

        fn finish_at_limit(self, end_sample: usize) -> Option<(usize, usize)> {
            let speech_start = self.speech_start?;
            let last_active = self.last_active?;
            if last_active.saturating_sub(speech_start)
                < self.samples_for_ms(self.config.minimum_speech_ms)
            {
                return None;
            }
            Some((
                speech_start.saturating_sub(self.samples_for_ms(self.config.pre_roll_ms)),
                end_sample,
            ))
        }

        fn samples_for_ms(self, millis: u32) -> usize {
            usize::try_from(
                u64::from(self.sample_rate)
                    .saturating_mul(u64::from(millis))
                    .saturating_div(1_000),
            )
            .unwrap_or(usize::MAX)
        }
    }

    fn validate_voice_activity_config(config: VoiceActivityConfig) -> VoiceResult<()> {
        if config.rms_threshold == 0
            || config.minimum_speech_ms == 0
            || config.trailing_silence_ms == 0
            || config.idle_timeout_ms == 0
        {
            return Err(VoiceError::InvalidConfiguration(
                "voice activity thresholds and durations must be positive".to_string(),
            ));
        }
        Ok(())
    }

    pub fn audio_devices() -> VoiceResult<AudioDeviceSummary> {
        let host = cpal::default_host();
        let default_input = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let default_output = host
            .default_output_device()
            .and_then(|device| device.name().ok());
        let mut input_devices = host
            .input_devices()
            .map_err(|error| VoiceError::AudioDevice(error.to_string()))?
            .filter_map(|device| device.name().ok())
            .collect::<Vec<_>>();
        input_devices.sort();
        input_devices.dedup();
        Ok(AudioDeviceSummary {
            default_input,
            default_output,
            input_devices,
        })
    }

    pub fn play_wav(bytes: &[u8]) -> VoiceResult<()> {
        play_wav_cancellable(bytes, &PlaybackCancellationToken::new()).map(|_| ())
    }

    pub fn play_wav_cancellable(
        bytes: &[u8],
        cancellation: &PlaybackCancellationToken,
    ) -> VoiceResult<bool> {
        validate_wav(bytes, bytes.len())?;
        let mut stream = rodio::OutputStreamBuilder::open_default_stream()
            .map_err(|error| VoiceError::AudioDevice(error.to_string()))?;
        stream.log_on_drop(false);
        let sink = rodio::Sink::connect_new(stream.mixer());
        let source = rodio::Decoder::try_from(Cursor::new(bytes.to_vec()))
            .map_err(|error| VoiceError::InvalidResponse(error.to_string()))?;
        sink.append(source);
        while !sink.empty() {
            if cancellation.is_cancelled() {
                sink.stop();
                drop(stream);
                return Ok(false);
            }
            thread::sleep(Duration::from_millis(20));
        }
        drop(stream);
        Ok(true)
    }

    fn select_input_device(
        host: &cpal::Host,
        requested: Option<&str>,
    ) -> VoiceResult<cpal::Device> {
        let Some(requested) = requested.map(str::trim).filter(|value| !value.is_empty()) else {
            return host.default_input_device().ok_or_else(|| {
                VoiceError::AudioDevice("no default microphone is available".to_string())
            });
        };
        host.input_devices()
            .map_err(|error| VoiceError::AudioDevice(error.to_string()))?
            .find(|device| device.name().is_ok_and(|name| name == requested))
            .ok_or_else(|| VoiceError::AudioDevice(format!("microphone not found: {requested}")))
    }

    fn build_stream<T>(
        device: &cpal::Device,
        config: &cpal::StreamConfig,
        channels: usize,
        maximum_samples: usize,
        samples: Arc<Mutex<Vec<i16>>>,
        stream_error: Arc<Mutex<Option<String>>>,
        reached_limit: Arc<AtomicBool>,
    ) -> VoiceResult<cpal::Stream>
    where
        T: Sample + SizedSample,
        i16: FromSample<T>,
    {
        device
            .build_input_stream(
                config,
                move |data: &[T], _| {
                    append_input_data(data, channels, maximum_samples, &samples, &reached_limit);
                },
                move |error| {
                    if let Ok(mut slot) = stream_error.lock()
                        && slot.is_none()
                    {
                        *slot = Some(error.to_string());
                    }
                },
                None,
            )
            .map_err(|error| VoiceError::AudioDevice(error.to_string()))
    }

    fn append_input_data<T>(
        input: &[T],
        channels: usize,
        maximum_samples: usize,
        samples: &Arc<Mutex<Vec<i16>>>,
        reached_limit: &Arc<AtomicBool>,
    ) where
        T: Sample,
        i16: FromSample<T>,
    {
        let Ok(mut samples) = samples.try_lock() else {
            return;
        };
        for frame in input.chunks(channels.max(1)) {
            if samples.len() >= maximum_samples {
                reached_limit.store(true, Ordering::Relaxed);
                break;
            }
            let total = frame
                .iter()
                .map(|sample| i64::from(i16::from_sample(*sample)))
                .sum::<i64>();
            let average = total / i64::try_from(frame.len()).unwrap_or(1);
            samples.push(average.clamp(i64::from(i16::MIN), i64::from(i16::MAX)) as i16);
        }
    }

    fn encode_mono_wav(samples: &[i16], sample_rate: u32) -> VoiceResult<Vec<u8>> {
        let mut cursor = Cursor::new(Vec::new());
        {
            let spec = hound::WavSpec {
                channels: 1,
                sample_rate,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            };
            let mut writer = hound::WavWriter::new(&mut cursor, spec)
                .map_err(|error| VoiceError::FileOperation(error.to_string()))?;
            for sample in samples {
                writer
                    .write_sample(*sample)
                    .map_err(|error| VoiceError::FileOperation(error.to_string()))?;
            }
            writer
                .finalize()
                .map_err(|error| VoiceError::FileOperation(error.to_string()))?;
        }
        Ok(cursor.into_inner())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn wav_encoder_produces_valid_mono_pcm() {
            let samples = vec![0_i16, 1_000, -1_000, i16::MAX];
            let bytes = encode_mono_wav(&samples, 16_000).expect("encode");
            validate_wav(&bytes, 1_024).expect("valid wav");
            let mut reader = hound::WavReader::new(Cursor::new(bytes)).expect("reader");
            assert_eq!(reader.spec().channels, 1);
            assert_eq!(reader.spec().sample_rate, 16_000);
            assert_eq!(
                reader
                    .samples::<i16>()
                    .collect::<Result<Vec<_>, _>>()
                    .expect("samples"),
                samples
            );
        }

        #[test]
        fn input_callback_downmixes_channels_and_honors_limit() {
            let samples = Arc::new(Mutex::new(Vec::new()));
            let reached_limit = Arc::new(AtomicBool::new(false));
            append_input_data(
                &[1_000_i16, -1_000, 2_000, 0, 4_000, 0],
                2,
                2,
                &samples,
                &reached_limit,
            );
            assert_eq!(*samples.lock().expect("samples"), vec![0, 1_000]);
            assert!(reached_limit.load(Ordering::Relaxed));
        }

        #[test]
        fn voice_activity_tracker_captures_speech_after_trailing_silence() {
            let config = VoiceActivityConfig {
                idle_timeout_ms: 1_000,
                ..VoiceActivityConfig::default()
            };
            let mut tracker = VoiceActivityTracker::new(1_000, config);

            assert_eq!(
                tracker.observe(0, 100, true),
                VoiceActivityDecision::Continue
            );
            assert_eq!(
                tracker.observe(100, 400, true),
                VoiceActivityDecision::Continue
            );
            assert_eq!(
                tracker.observe(400, 1_300, false),
                VoiceActivityDecision::Capture { start: 0, end: 700 }
            );
        }

        #[test]
        fn voice_activity_tracker_discards_short_noise_and_times_out_when_idle() {
            let config = VoiceActivityConfig {
                idle_timeout_ms: 1_000,
                ..VoiceActivityConfig::default()
            };
            let mut tracker = VoiceActivityTracker::new(1_000, config);

            assert_eq!(
                tracker.observe(0, 50, true),
                VoiceActivityDecision::Continue
            );
            assert_eq!(
                tracker.observe(50, 950, false),
                VoiceActivityDecision::Continue
            );
            assert_eq!(
                tracker.observe(950, 1_000, false),
                VoiceActivityDecision::IdleTimeout
            );
        }

        #[test]
        fn voice_activity_tracker_keeps_valid_speech_at_recording_limit() {
            let config = VoiceActivityConfig::default();
            let mut tracker = VoiceActivityTracker::new(1_000, config);
            assert_eq!(
                tracker.observe(500, 900, true),
                VoiceActivityDecision::Continue
            );

            assert_eq!(tracker.finish_at_limit(1_000), Some((250, 1_000)));
        }

        #[test]
        fn playback_cancellation_token_is_shared_and_monotonic() {
            let token = PlaybackCancellationToken::new();
            let observer = token.clone();
            assert!(!observer.is_cancelled());

            token.cancel();

            assert!(observer.is_cancelled());
        }
    }
}

#[cfg(windows)]
pub use platform::{LiveRecording, audio_devices, play_wav, play_wav_cancellable};

#[cfg(not(windows))]
pub struct LiveRecording;

#[cfg(not(windows))]
impl LiveRecording {
    pub fn start(_input_device: Option<&str>, _maximum_seconds: u32) -> VoiceResult<Self> {
        Err(VoiceError::AudioDevice(
            "live microphone capture is currently supported on Windows only".to_string(),
        ))
    }

    pub fn finish(self, _maximum_bytes: usize) -> VoiceResult<CapturedAudio> {
        Err(VoiceError::AudioDevice(
            "live microphone capture is currently supported on Windows only".to_string(),
        ))
    }

    pub fn capture_until_silence(
        self,
        _maximum_bytes: usize,
        _config: VoiceActivityConfig,
    ) -> VoiceResult<Option<CapturedAudio>> {
        Err(VoiceError::AudioDevice(
            "live microphone capture is currently supported on Windows only".to_string(),
        ))
    }

    pub fn capture_until_silence_with_control(
        self,
        _maximum_bytes: usize,
        _config: VoiceActivityConfig,
        _should_cancel: impl FnMut() -> bool,
    ) -> VoiceResult<Option<CapturedAudio>> {
        Err(VoiceError::AudioDevice(
            "live microphone capture is currently supported on Windows only".to_string(),
        ))
    }
}

#[cfg(not(windows))]
pub fn audio_devices() -> VoiceResult<AudioDeviceSummary> {
    Err(VoiceError::AudioDevice(
        "live audio devices are currently supported on Windows only".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn play_wav(_bytes: &[u8]) -> VoiceResult<()> {
    Err(VoiceError::AudioDevice(
        "live audio playback is currently supported on Windows only".to_string(),
    ))
}

#[cfg(not(windows))]
pub fn play_wav_cancellable(
    _bytes: &[u8],
    _cancellation: &PlaybackCancellationToken,
) -> VoiceResult<bool> {
    Err(VoiceError::AudioDevice(
        "live audio playback is currently supported on Windows only".to_string(),
    ))
}
