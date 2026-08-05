use async_trait::async_trait;
use reqwest::Url;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderValue};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use thiserror::Error;

mod live;

pub use live::{
    AudioDeviceSummary, CapturedAudio, LiveRecording, PlaybackCancellationToken,
    VoiceActivityConfig, audio_devices, play_wav, play_wav_cancellable,
};

pub const VOICE_API_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_VOICE_RUNTIME_URL: &str = "http://127.0.0.1:17862";
pub const DEFAULT_PRESET_VOICE: &str = "中文女";
pub const DEFAULT_MAX_INPUT_BYTES: usize = 25 * 1024 * 1024;
pub const DEFAULT_MAX_OUTPUT_BYTES: usize = 50 * 1024 * 1024;
pub const DEFAULT_MAX_SPEECH_CHARS: usize = 2_000;
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 180;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("voice input is invalid: {0}")]
    InvalidInput(String),
    #[error("voice runtime configuration is invalid: {0}")]
    InvalidConfiguration(String),
    #[error("voice runtime is unavailable: {0}")]
    RuntimeUnavailable(String),
    #[error("voice runtime request failed: {0}")]
    Request(String),
    #[error("voice runtime returned an invalid response: {0}")]
    InvalidResponse(String),
    #[error("voice file operation failed: {0}")]
    FileOperation(String),
    #[error("voice audio device failed: {0}")]
    AudioDevice(String),
}

pub type VoiceResult<T> = Result<T, VoiceError>;

#[derive(Clone)]
pub struct VoiceClientConfig {
    pub stt_base_url: String,
    pub tts_base_url: String,
    pub request_timeout: Duration,
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_speech_chars: usize,
    pub allow_remote: bool,
    auth_token: Option<String>,
}

impl fmt::Debug for VoiceClientConfig {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("VoiceClientConfig")
            .field("stt_base_url", &self.stt_base_url)
            .field("tts_base_url", &self.tts_base_url)
            .field("request_timeout", &self.request_timeout)
            .field("max_input_bytes", &self.max_input_bytes)
            .field("max_output_bytes", &self.max_output_bytes)
            .field("max_speech_chars", &self.max_speech_chars)
            .field("allow_remote", &self.allow_remote)
            .field("auth_token_configured", &self.auth_token.is_some())
            .finish()
    }
}

impl Default for VoiceClientConfig {
    fn default() -> Self {
        Self {
            stt_base_url: DEFAULT_VOICE_RUNTIME_URL.to_string(),
            tts_base_url: DEFAULT_VOICE_RUNTIME_URL.to_string(),
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECONDS),
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
            max_speech_chars: DEFAULT_MAX_SPEECH_CHARS,
            allow_remote: false,
            auth_token: None,
        }
    }
}

impl VoiceClientConfig {
    pub fn from_env() -> Self {
        let runtime_url = non_empty_env("YUNXI_VOICE_RUNTIME_URL")
            .unwrap_or_else(|| DEFAULT_VOICE_RUNTIME_URL.to_string());
        let request_timeout = non_empty_env("YUNXI_VOICE_REQUEST_TIMEOUT_SECONDS")
            .and_then(|value| value.parse::<u64>().ok())
            .map(|seconds| Duration::from_secs(seconds.clamp(1, 600)))
            .unwrap_or_else(|| Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECONDS));
        Self {
            stt_base_url: non_empty_env("YUNXI_VOICE_STT_URL")
                .unwrap_or_else(|| runtime_url.clone()),
            tts_base_url: non_empty_env("YUNXI_VOICE_TTS_URL").unwrap_or(runtime_url),
            request_timeout,
            max_input_bytes: parsed_size_env(
                "YUNXI_VOICE_MAX_INPUT_BYTES",
                DEFAULT_MAX_INPUT_BYTES,
            ),
            max_output_bytes: parsed_size_env(
                "YUNXI_VOICE_MAX_OUTPUT_BYTES",
                DEFAULT_MAX_OUTPUT_BYTES,
            ),
            max_speech_chars: parsed_size_env(
                "YUNXI_VOICE_MAX_SPEECH_CHARS",
                DEFAULT_MAX_SPEECH_CHARS,
            ),
            allow_remote: env_enabled("YUNXI_VOICE_ALLOW_REMOTE"),
            auth_token: non_empty_env("YUNXI_VOICE_AUTH_TOKEN"),
        }
    }

    pub fn with_runtime_url(mut self, value: impl Into<String>) -> Self {
        let value = value.into();
        self.stt_base_url = value.clone();
        self.tts_base_url = value;
        self
    }

    pub fn with_auth_token(mut self, value: impl Into<String>) -> Self {
        self.auth_token = Some(value.into());
        self
    }

    fn validate(&self) -> VoiceResult<()> {
        validate_base_url(&self.stt_base_url, self.allow_remote)?;
        validate_base_url(&self.tts_base_url, self.allow_remote)?;
        if self.max_input_bytes < 44 {
            return Err(VoiceError::InvalidConfiguration(
                "maximum input size is too small for a WAV file".to_string(),
            ));
        }
        if self.max_output_bytes < 44 {
            return Err(VoiceError::InvalidConfiguration(
                "maximum output size is too small for a WAV file".to_string(),
            ));
        }
        if self.max_speech_chars == 0 {
            return Err(VoiceError::InvalidConfiguration(
                "maximum speech text length must be positive".to_string(),
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VoiceComponentHealth {
    pub provider: String,
    pub model: String,
    pub device: String,
    pub ready: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct VoiceRuntimeHealth {
    pub schema_version: u32,
    pub status: String,
    pub stt: VoiceComponentHealth,
    pub tts: VoiceComponentHealth,
    #[serde(default)]
    pub preset_voices: Vec<String>,
}

impl VoiceRuntimeHealth {
    pub fn ready(&self) -> bool {
        self.schema_version == VOICE_API_SCHEMA_VERSION
            && self.status == "ok"
            && self.stt.ready
            && self.tts.ready
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AudioInput {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

impl AudioInput {
    pub fn wav(bytes: Vec<u8>, maximum_bytes: usize) -> VoiceResult<Self> {
        validate_wav(&bytes, maximum_bytes)?;
        Ok(Self {
            bytes,
            content_type: "audio/wav".to_string(),
        })
    }

    pub fn from_wav_file(path: impl AsRef<Path>, maximum_bytes: usize) -> VoiceResult<Self> {
        let path = path.as_ref();
        let label = file_label(path);
        let metadata = fs::metadata(path).map_err(|error| {
            VoiceError::FileOperation(format!("failed to inspect input file {label}: {error}"))
        })?;
        let size = usize::try_from(metadata.len()).unwrap_or(usize::MAX);
        if size > maximum_bytes {
            return Err(VoiceError::InvalidInput(format!(
                "input WAV exceeds the {maximum_bytes}-byte limit"
            )));
        }
        let bytes = fs::read(path).map_err(|error| {
            VoiceError::FileOperation(format!("failed to read input file {label}: {error}"))
        })?;
        Self::wav(bytes, maximum_bytes)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub emotion: Option<String>,
    #[serde(default)]
    pub audio_events: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SynthesisRequest {
    pub text: String,
    pub voice: String,
    pub format: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SynthesizedAudio {
    pub bytes: Vec<u8>,
    pub content_type: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeechRender {
    pub text: String,
    pub truncated: bool,
    pub omitted_code: bool,
    pub omitted_links: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpeechChunkRender {
    pub chunks: Vec<String>,
    pub truncated: bool,
    pub omitted_code: bool,
    pub omitted_links: bool,
}

#[async_trait]
pub trait SpeechToTextProvider: Send + Sync {
    async fn transcribe(
        &self,
        input: AudioInput,
        language: Option<&str>,
    ) -> VoiceResult<TranscriptionResult>;
}

#[async_trait]
pub trait TextToSpeechProvider: Send + Sync {
    async fn synthesize(&self, request: SynthesisRequest) -> VoiceResult<SynthesizedAudio>;
}

#[derive(Clone, Debug)]
pub struct VoiceRuntimeClient {
    config: VoiceClientConfig,
    client: reqwest::Client,
}

impl VoiceRuntimeClient {
    pub fn new(config: VoiceClientConfig) -> VoiceResult<Self> {
        config.validate()?;
        let client = reqwest::Client::builder()
            .timeout(config.request_timeout)
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("yunxi-agent-voice/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|error| VoiceError::InvalidConfiguration(error.to_string()))?;
        Ok(Self { config, client })
    }

    pub fn config(&self) -> &VoiceClientConfig {
        &self.config
    }

    pub async fn health(&self) -> VoiceResult<VoiceRuntimeHealth> {
        let response = self
            .authorized(
                self.client
                    .get(endpoint(&self.config.stt_base_url, "/health")?),
            )?
            .send()
            .await
            .map_err(request_error)?;
        let response = successful(response).await?;
        let health = response
            .json::<VoiceRuntimeHealth>()
            .await
            .map_err(|error| VoiceError::InvalidResponse(error.to_string()))?;
        if health.schema_version != VOICE_API_SCHEMA_VERSION {
            return Err(VoiceError::InvalidResponse(format!(
                "unsupported schema version {}",
                health.schema_version
            )));
        }
        Ok(health)
    }

    fn authorized(&self, request: reqwest::RequestBuilder) -> VoiceResult<reqwest::RequestBuilder> {
        let Some(token) = self.config.auth_token.as_deref() else {
            return Ok(request);
        };
        let value = HeaderValue::from_str(&format!("Bearer {token}"))
            .map_err(|_| VoiceError::InvalidConfiguration("voice auth token is invalid".into()))?;
        Ok(request.header(AUTHORIZATION, value))
    }
}

#[async_trait]
impl SpeechToTextProvider for VoiceRuntimeClient {
    async fn transcribe(
        &self,
        input: AudioInput,
        language: Option<&str>,
    ) -> VoiceResult<TranscriptionResult> {
        validate_wav(&input.bytes, self.config.max_input_bytes)?;
        let mut request = self
            .client
            .post(endpoint(&self.config.stt_base_url, "/v1/transcribe")?)
            .header(CONTENT_TYPE, input.content_type)
            .body(input.bytes);
        if let Some(language) = language.filter(|value| !value.trim().is_empty()) {
            request = request.header("x-yunxi-language", language.trim());
        }
        let response = self
            .authorized(request)?
            .send()
            .await
            .map_err(request_error)?;
        let response = successful(response).await?;
        let result = response
            .json::<TranscriptionResult>()
            .await
            .map_err(|error| VoiceError::InvalidResponse(error.to_string()))?;
        if result.text.trim().is_empty() {
            return Err(VoiceError::InvalidResponse(
                "transcription text is empty".to_string(),
            ));
        }
        Ok(result)
    }
}

#[async_trait]
impl TextToSpeechProvider for VoiceRuntimeClient {
    async fn synthesize(&self, request: SynthesisRequest) -> VoiceResult<SynthesizedAudio> {
        if request.text.trim().is_empty() {
            return Err(VoiceError::InvalidInput(
                "speech text must not be empty".to_string(),
            ));
        }
        if request.text.chars().count() > self.config.max_speech_chars {
            return Err(VoiceError::InvalidInput(format!(
                "speech text exceeds the {}-character limit",
                self.config.max_speech_chars
            )));
        }
        if request.format != "wav" {
            return Err(VoiceError::InvalidInput(
                "the MVP voice runtime supports WAV output only".to_string(),
            ));
        }
        let response = self
            .authorized(
                self.client
                    .post(endpoint(&self.config.tts_base_url, "/v1/synthesize")?)
                    .json(&request),
            )?
            .send()
            .await
            .map_err(request_error)?;
        let response = successful(response).await?;
        if let Some(length) = response.content_length()
            && length > self.config.max_output_bytes as u64
        {
            return Err(VoiceError::InvalidResponse(
                "synthesized WAV exceeds the configured output limit".to_string(),
            ));
        }
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("audio/wav")
            .to_string();
        let bytes = response
            .bytes()
            .await
            .map_err(|error| VoiceError::Request(error.to_string()))?
            .to_vec();
        validate_wav(&bytes, self.config.max_output_bytes)?;
        Ok(SynthesizedAudio {
            bytes,
            content_type,
        })
    }
}

pub fn render_spoken_text(input: &str, maximum_chars: usize) -> VoiceResult<SpeechRender> {
    if maximum_chars == 0 {
        return Err(VoiceError::InvalidConfiguration(
            "maximum speech text length must be positive".to_string(),
        ));
    }
    let mut rendered = String::new();
    let mut in_code = false;
    let mut omitted_code = false;
    let mut omitted_links = false;
    for line in input.lines() {
        let mut line = line.trim();
        if line.starts_with("```") || line.starts_with("~~~") {
            in_code = !in_code;
            omitted_code = true;
            continue;
        }
        if in_code {
            omitted_code = true;
            continue;
        }
        line = strip_line_prefix(line);
        if line.is_empty() {
            continue;
        }
        let (line, had_link) = strip_markdown_links(line);
        omitted_links |= had_link;
        let line = line
            .replace(['*', '`'], "")
            .replace("__", "")
            .trim()
            .to_string();
        if line.is_empty() {
            continue;
        }
        if !rendered.is_empty() {
            rendered.push(' ');
        }
        rendered.push_str(&line);
    }
    if omitted_code || omitted_links {
        if !rendered.is_empty() {
            rendered.push(' ');
        }
        rendered.push_str("具体代码和链接我放在文字回复里。");
    }
    let rendered = normalize_brand_pronunciation(&collapse_whitespace(&rendered));
    if rendered.is_empty() {
        return Err(VoiceError::InvalidInput(
            "assistant response has no speakable text".to_string(),
        ));
    }
    let (text, truncated) = truncate_for_speech(&rendered, maximum_chars);
    Ok(SpeechRender {
        text,
        truncated,
        omitted_code,
        omitted_links,
    })
}

pub fn render_spoken_chunks(
    input: &str,
    maximum_chars: usize,
    maximum_chunk_chars: usize,
) -> VoiceResult<SpeechChunkRender> {
    if maximum_chunk_chars == 0 {
        return Err(VoiceError::InvalidConfiguration(
            "maximum speech chunk length must be positive".to_string(),
        ));
    }
    let rendered = render_spoken_text(input, maximum_chars)?;
    let chunks = split_spoken_text(&rendered.text, maximum_chunk_chars);
    Ok(SpeechChunkRender {
        chunks,
        truncated: rendered.truncated,
        omitted_code: rendered.omitted_code,
        omitted_links: rendered.omitted_links,
    })
}

fn split_spoken_text(input: &str, maximum_chunk_chars: usize) -> Vec<String> {
    let sentence_minimum = maximum_chunk_chars.min(24).max(8) / 2;
    let clause_minimum = maximum_chunk_chars
        .saturating_mul(2)
        .saturating_div(3)
        .max(8);
    let mut chunks = Vec::new();
    let mut current = String::new();
    let mut current_chars = 0usize;

    for character in input.chars() {
        current.push(character);
        current_chars += 1;
        let sentence_boundary = matches!(character, '。' | '！' | '？' | '!' | '?')
            && current_chars >= sentence_minimum;
        let clause_boundary =
            matches!(character, '；' | ';' | '，' | ',') && current_chars >= clause_minimum;
        if current_chars >= maximum_chunk_chars || sentence_boundary || clause_boundary {
            let chunk = current.trim().to_string();
            if !chunk.is_empty() {
                chunks.push(chunk);
            }
            current.clear();
            current_chars = 0;
        }
    }
    let tail = current.trim().to_string();
    if !tail.is_empty() {
        chunks.push(tail);
    }

    if chunks.len() > 1 && chunks.last().is_some_and(|chunk| chunk.chars().count() < 8) {
        let tail = chunks.pop().expect("tail exists after length check");
        let previous = chunks.last_mut().expect("previous chunk exists");
        if previous.chars().count() + tail.chars().count() <= maximum_chunk_chars {
            previous.push_str(&tail);
        } else {
            chunks.push(tail);
        }
    }
    chunks
}

fn normalize_brand_pronunciation(input: &str) -> String {
    const BRAND: &str = "yunxi";
    const PRODUCT: &str = "agent";

    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    while index < input.len() {
        let remainder = &input[index..];
        let previous_is_word = input[..index]
            .chars()
            .next_back()
            .is_some_and(is_ascii_word_character);
        if !previous_is_word && starts_with_ascii_case_insensitive(remainder, BRAND) {
            let brand_end = index + BRAND.len();
            let mut product_start = brand_end;
            while product_start < input.len() {
                let character = input[product_start..]
                    .chars()
                    .next()
                    .expect("product start is within input");
                if !character.is_ascii_whitespace() {
                    break;
                }
                product_start += character.len_utf8();
            }
            let has_separator = product_start > brand_end;
            let product_remainder = &input[product_start..];
            if has_separator
                && starts_with_ascii_case_insensitive(product_remainder, PRODUCT)
                && has_ascii_word_boundary_after(input, product_start + PRODUCT.len())
            {
                output.push_str("云熙");
                index = product_start + PRODUCT.len();
                continue;
            }
            if has_ascii_word_boundary_after(input, brand_end) {
                output.push_str("云熙");
                index = brand_end;
                continue;
            }
        }

        let character = remainder.chars().next().expect("index is within input");
        output.push(character);
        index += character.len_utf8();
    }
    output
}

fn starts_with_ascii_case_insensitive(value: &str, pattern: &str) -> bool {
    value
        .as_bytes()
        .get(..pattern.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(pattern.as_bytes()))
}

fn has_ascii_word_boundary_after(value: &str, index: usize) -> bool {
    value
        .get(index..)
        .and_then(|remainder| remainder.chars().next())
        .is_none_or(|character| !is_ascii_word_character(character))
}

fn is_ascii_word_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

pub fn write_wav_atomic(
    output: impl AsRef<Path>,
    bytes: &[u8],
    maximum_bytes: usize,
    overwrite: bool,
) -> VoiceResult<PathBuf> {
    validate_wav(bytes, maximum_bytes)?;
    let output = output.as_ref();
    let label = file_label(output);
    if output.extension().and_then(|value| value.to_str()) != Some("wav") {
        return Err(VoiceError::InvalidInput(
            "voice output path must use the .wav extension".to_string(),
        ));
    }
    if output.exists() && !overwrite {
        return Err(VoiceError::FileOperation(format!(
            "output file {label} already exists; pass --force to replace it"
        )));
    }
    let parent = output.parent().filter(|path| !path.as_os_str().is_empty());
    if let Some(parent) = parent
        && !parent.is_dir()
    {
        return Err(VoiceError::FileOperation(
            "output directory does not exist".to_string(),
        ));
    }
    let temporary = temporary_output_path(output);
    let result = (|| -> VoiceResult<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| {
                VoiceError::FileOperation(format!(
                    "failed to create temporary output for {label}: {error}"
                ))
            })?;
        file.write_all(bytes).map_err(|error| {
            VoiceError::FileOperation(format!("failed to write output {label}: {error}"))
        })?;
        file.sync_all().map_err(|error| {
            VoiceError::FileOperation(format!("failed to flush output {label}: {error}"))
        })?;
        finalize_output(&temporary, output, overwrite, &label)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result?;
    Ok(output.to_path_buf())
}

pub fn validate_wav(bytes: &[u8], maximum_bytes: usize) -> VoiceResult<()> {
    if bytes.len() < 44 {
        return Err(VoiceError::InvalidInput(
            "WAV input is shorter than the minimum header".to_string(),
        ));
    }
    if bytes.len() > maximum_bytes {
        return Err(VoiceError::InvalidInput(format!(
            "WAV input exceeds the {maximum_bytes}-byte limit"
        )));
    }
    if &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(VoiceError::InvalidInput(
            "audio input must be a RIFF/WAVE file".to_string(),
        ));
    }
    Ok(())
}

fn validate_base_url(value: &str, allow_remote: bool) -> VoiceResult<()> {
    let url = Url::parse(value)
        .map_err(|error| VoiceError::InvalidConfiguration(format!("invalid URL: {error}")))?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return Err(VoiceError::InvalidConfiguration(
            "voice runtime URL must use http or https".to_string(),
        ));
    }
    if !allow_remote {
        let local = matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "::1"));
        if !local {
            return Err(VoiceError::InvalidConfiguration(
                "remote voice runtime URLs require YUNXI_VOICE_ALLOW_REMOTE=1".to_string(),
            ));
        }
    }
    Ok(())
}

fn endpoint(base_url: &str, path: &str) -> VoiceResult<Url> {
    let base = base_url.trim_end_matches('/');
    Url::parse(&format!("{base}{path}"))
        .map_err(|error| VoiceError::InvalidConfiguration(format!("invalid endpoint: {error}")))
}

async fn successful(response: reqwest::Response) -> VoiceResult<reqwest::Response> {
    if response.status().is_success() {
        return Ok(response);
    }
    let status = response.status();
    let message = response
        .json::<VoiceErrorResponse>()
        .await
        .ok()
        .map(|value| value.error)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "voice runtime rejected the request".to_string());
    Err(VoiceError::Request(format!("HTTP {status}: {message}")))
}

#[derive(Deserialize)]
struct VoiceErrorResponse {
    error: String,
}

fn request_error(error: reqwest::Error) -> VoiceError {
    if error.is_timeout() {
        VoiceError::RuntimeUnavailable("request timed out".to_string())
    } else if error.is_connect() {
        VoiceError::RuntimeUnavailable("connection failed".to_string())
    } else {
        VoiceError::Request(error.to_string())
    }
}

fn strip_line_prefix(mut value: &str) -> &str {
    value = value.trim_start_matches('#').trim_start();
    for prefix in ["> ", "- ", "* ", "+ "] {
        if let Some(stripped) = value.strip_prefix(prefix) {
            return stripped.trim_start();
        }
    }
    let Some((prefix, rest)) = value.split_once(". ") else {
        return value;
    };
    if !prefix.is_empty() && prefix.chars().all(|character| character.is_ascii_digit()) {
        rest.trim_start()
    } else {
        value
    }
}

fn strip_markdown_links(value: &str) -> (String, bool) {
    let mut output = String::new();
    let characters = value.chars().collect::<Vec<_>>();
    let mut index = 0;
    let mut omitted = false;
    while index < characters.len() {
        if starts_url(&characters, index) {
            omitted = true;
            while index < characters.len() && !characters[index].is_whitespace() {
                index += 1;
            }
            continue;
        }
        let image = characters[index] == '!' && characters.get(index + 1) == Some(&'[');
        let label_start = if image {
            index + 2
        } else if characters[index] == '[' {
            index + 1
        } else {
            output.push(characters[index]);
            index += 1;
            continue;
        };
        let Some(label_end) = characters[label_start..]
            .iter()
            .position(|character| *character == ']')
            .map(|offset| label_start + offset)
        else {
            output.push(characters[index]);
            index += 1;
            continue;
        };
        if characters.get(label_end + 1) != Some(&'(') {
            output.push(characters[index]);
            index += 1;
            continue;
        }
        let Some(link_end) = characters[label_end + 2..]
            .iter()
            .position(|character| *character == ')')
            .map(|offset| label_end + 2 + offset)
        else {
            output.push(characters[index]);
            index += 1;
            continue;
        };
        omitted = true;
        if !image {
            output.extend(characters[label_start..label_end].iter());
        }
        index = link_end + 1;
    }
    (output, omitted)
}

fn starts_url(characters: &[char], index: usize) -> bool {
    for prefix in ["https://", "http://"] {
        let prefix = prefix.chars().collect::<Vec<_>>();
        if characters.get(index..index + prefix.len()) == Some(prefix.as_slice()) {
            return true;
        }
    }
    false
}

fn collapse_whitespace(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn truncate_for_speech(value: &str, maximum_chars: usize) -> (String, bool) {
    if value.chars().count() <= maximum_chars {
        return (value.to_string(), false);
    }
    let reserve = " 后面的内容请查看文字回复。";
    let keep = maximum_chars.saturating_sub(reserve.chars().count()).max(1);
    let prefix = value.chars().take(keep).collect::<String>();
    let boundary = prefix
        .char_indices()
        .rev()
        .find(|(_, character)| matches!(character, '。' | '！' | '？' | '.' | '!' | '?'))
        .map(|(index, character)| index + character.len_utf8())
        .unwrap_or(prefix.len());
    let mut output = prefix[..boundary].trim().to_string();
    output.push_str(reserve);
    (output, true)
}

fn file_label(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("<voice-file>")
        .to_string()
}

fn temporary_output_path(output: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("voice.wav");
    output.with_file_name(format!(".{file_name}.tmp-{}-{nonce}", std::process::id()))
}

fn backup_output_path(output: &Path) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let file_name = output
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("voice.wav");
    output.with_file_name(format!(
        ".{file_name}.backup-{}-{nonce}",
        std::process::id()
    ))
}

fn finalize_output(
    temporary: &Path,
    output: &Path,
    overwrite: bool,
    label: &str,
) -> VoiceResult<()> {
    let backup = if output.exists() {
        if !overwrite {
            return Err(VoiceError::FileOperation(format!(
                "output file {label} already exists; pass --force to replace it"
            )));
        }
        let backup = backup_output_path(output);
        fs::rename(output, &backup).map_err(|error| {
            VoiceError::FileOperation(format!(
                "failed to preserve existing output {label}: {error}"
            ))
        })?;
        Some(backup)
    } else {
        None
    };

    if let Err(error) = fs::rename(temporary, output) {
        if let Some(backup) = backup {
            if let Err(restore_error) = fs::rename(&backup, output) {
                return Err(VoiceError::FileOperation(format!(
                    "failed to finalize output {label}: {error}; the previous output also could not be restored: {restore_error}"
                )));
            }
        }
        return Err(VoiceError::FileOperation(format!(
            "failed to finalize output {label}: {error}"
        )));
    }

    if let Some(backup) = backup {
        fs::remove_file(&backup).map_err(|error| {
            VoiceError::FileOperation(format!(
                "output {label} was replaced but its temporary backup could not be removed: {error}"
            ))
        })?;
    }
    Ok(())
}

fn non_empty_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn parsed_size_env(name: &str, default_value: usize) -> usize {
    non_empty_env(name)
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(default_value)
}

fn env_enabled(name: &str) -> bool {
    non_empty_env(name)
        .map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use wiremock::matchers::{body_bytes, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn wav_bytes() -> Vec<u8> {
        let mut bytes = vec![0_u8; 44];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes
    }

    #[test]
    fn validates_riff_wave_and_rejects_other_bytes() {
        assert!(validate_wav(&wav_bytes(), 1024).is_ok());
        assert!(validate_wav(&[0_u8; 44], 1024).is_err());
        assert!(validate_wav(&wav_bytes(), 43).is_err());
    }

    #[test]
    fn spoken_renderer_omits_code_and_links_without_changing_source() {
        let source = "## 回答\n先看[说明](https://example.com)。\n```rust\nlet secret = 1;\n```";
        let rendered = render_spoken_text(source, 200).expect("render");
        assert_eq!(
            rendered.text,
            "回答 先看说明。 具体代码和链接我放在文字回复里。"
        );
        assert!(rendered.omitted_code);
        assert!(rendered.omitted_links);
        assert_eq!(
            source,
            "## 回答\n先看[说明](https://example.com)。\n```rust\nlet secret = 1;\n```"
        );
    }

    #[test]
    fn spoken_renderer_truncates_with_visible_fallback() {
        let rendered = render_spoken_text(&"内容".repeat(100), 40).expect("render");
        assert!(rendered.truncated);
        assert!(rendered.text.contains("查看文字回复"));
    }

    #[test]
    fn spoken_renderer_normalizes_yunxi_brand_without_changing_source() {
        let source = "YunXi Agent、yunxi agent 和 YUNXI 会回复；myYunXiAgent 保持原样。";
        let rendered = render_spoken_text(source, 200).expect("render");

        assert_eq!(
            rendered.text,
            "云熙、云熙 和 云熙 会回复；myYunXiAgent 保持原样。"
        );
        assert_eq!(
            source,
            "YunXi Agent、yunxi agent 和 YUNXI 会回复；myYunXiAgent 保持原样。"
        );
    }

    #[test]
    fn spoken_chunk_renderer_prefers_natural_boundaries_and_bounds_every_chunk() {
        let source = "第一句话用于立即开始播放。第二句话稍微长一些，用来验证自然分段不会一直等到整段合成结束！最后给出一个简短收尾。";

        let rendered = render_spoken_chunks(source, 200, 28).expect("render chunks");

        assert!(rendered.chunks.len() >= 3);
        assert!(
            rendered
                .chunks
                .iter()
                .all(|chunk| chunk.chars().count() <= 28)
        );
        assert_eq!(rendered.chunks.concat(), source);
        assert!(rendered.chunks[0].ends_with('。'));
    }

    #[test]
    fn spoken_chunk_renderer_rejects_zero_chunk_limit() {
        assert!(render_spoken_chunks("你好。", 100, 0).is_err());
    }

    #[test]
    fn atomic_writer_refuses_existing_output_without_force() {
        let temp = TempDir::new().expect("temp");
        let output = temp.path().join("reply.wav");
        fs::write(&output, wav_bytes()).expect("seed");
        let error = write_wav_atomic(&output, &wav_bytes(), 1024, false).expect_err("refuse");
        assert!(error.to_string().contains("already exists"));
    }

    #[test]
    fn atomic_writer_replaces_existing_output_without_leaving_backup() {
        let temp = TempDir::new().expect("temp");
        let output = temp.path().join("reply.wav");
        fs::write(&output, wav_bytes()).expect("seed");
        let mut replacement = wav_bytes();
        replacement.extend_from_slice(b"replacement");

        write_wav_atomic(&output, &replacement, 1024, true).expect("replace");

        assert_eq!(fs::read(&output).expect("read"), replacement);
        let leftovers = fs::read_dir(temp.path())
            .expect("list")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.contains(".backup-") || name.contains(".tmp-"))
            .collect::<Vec<_>>();
        assert!(leftovers.is_empty(), "leftover files: {leftovers:?}");
    }

    #[test]
    fn remote_runtime_requires_explicit_opt_in() {
        let config = VoiceClientConfig {
            stt_base_url: "https://voice.example.com".to_string(),
            tts_base_url: "https://voice.example.com".to_string(),
            ..VoiceClientConfig::default()
        };
        assert!(VoiceRuntimeClient::new(config).is_err());
    }

    #[tokio::test]
    async fn http_client_completes_health_transcription_and_synthesis() {
        let server = MockServer::start().await;
        let health = VoiceRuntimeHealth {
            schema_version: VOICE_API_SCHEMA_VERSION,
            status: "ok".to_string(),
            stt: VoiceComponentHealth {
                provider: "mock".to_string(),
                model: "mock-stt".to_string(),
                device: "cpu".to_string(),
                ready: true,
            },
            tts: VoiceComponentHealth {
                provider: "mock".to_string(),
                model: "mock-tts".to_string(),
                device: "cpu".to_string(),
                ready: true,
            },
            preset_voices: vec![DEFAULT_PRESET_VOICE.to_string()],
        };
        Mock::given(method("GET"))
            .and(path("/health"))
            .respond_with(ResponseTemplate::new(200).set_body_json(&health))
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/transcribe"))
            .and(header("content-type", "audio/wav"))
            .and(body_bytes(wav_bytes()))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(TranscriptionResult {
                    text: "你好，云希。".to_string(),
                    language: Some("zh".to_string()),
                    emotion: None,
                    audio_events: Vec::new(),
                }),
            )
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/v1/synthesize"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "audio/wav")
                    .set_body_bytes(wav_bytes()),
            )
            .mount(&server)
            .await;

        let client =
            VoiceRuntimeClient::new(VoiceClientConfig::default().with_runtime_url(server.uri()))
                .expect("client");
        assert!(client.health().await.expect("health").ready());
        let transcript = client
            .transcribe(AudioInput::wav(wav_bytes(), 1024).expect("wav"), Some("zh"))
            .await
            .expect("transcribe");
        assert_eq!(transcript.text, "你好，云希。");
        let audio = client
            .synthesize(SynthesisRequest {
                text: "你好。".to_string(),
                voice: DEFAULT_PRESET_VOICE.to_string(),
                format: "wav".to_string(),
            })
            .await
            .expect("synthesize");
        assert_eq!(audio.bytes, wav_bytes());
    }
}
