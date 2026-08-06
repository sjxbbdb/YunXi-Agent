use crate::provider_mode::ProviderMode;
use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde::Serialize;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use yunxi_agent_core::{AgentConfig, AgentRunStatus, BackendKind};
use yunxi_agent_voice::{
    AudioInput, DEFAULT_PRESET_VOICE, LiveRecording, SpeechToTextProvider, SynthesisRequest,
    SynthesizedAudio, TextToSpeechProvider, TranscriptionResult, VoiceClientConfig,
    VoiceRuntimeClient, VoiceRuntimeHealth, audio_devices, play_wav, render_spoken_text,
    write_wav_atomic,
};

#[derive(Debug, Subcommand)]
pub(crate) enum VoiceCommand {
    #[command(about = "Check the local STT/TTS runtime without sending audio")]
    Doctor,
    #[command(about = "List microphones and the default playback device")]
    Devices,
    #[command(about = "Transcribe one local WAV file")]
    Transcribe {
        #[arg(long, value_name = "WAV")]
        input: PathBuf,
        #[arg(long, default_value = "auto")]
        language: String,
    },
    #[command(about = "Synthesize text with the configured preset voice")]
    Speak {
        #[arg(value_name = "TEXT", num_args = 1..)]
        text: Vec<String>,
        #[arg(long, value_name = "WAV")]
        output: PathBuf,
        #[arg(long, default_value = DEFAULT_PRESET_VOICE)]
        voice: String,
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Run WAV -> STT -> YunXi Runtime -> TTS -> WAV")]
    Chat {
        #[arg(long, value_name = "WAV")]
        input: PathBuf,
        #[arg(long, value_name = "WAV")]
        output: PathBuf,
        #[arg(long, default_value = "auto")]
        language: String,
        #[arg(long, default_value = DEFAULT_PRESET_VOICE)]
        voice: String,
        #[arg(long)]
        force: bool,
    },
    #[command(about = "Start a live push-to-talk voice conversation")]
    Talk {
        #[arg(long, default_value = "auto")]
        language: String,
        #[arg(long, default_value = DEFAULT_PRESET_VOICE)]
        voice: String,
        #[arg(long, value_name = "NAME")]
        input_device: Option<String>,
        #[arg(long)]
        no_playback: bool,
        #[arg(long, default_value_t = 60)]
        max_recording_seconds: u32,
    },
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceDoctorOutput {
    status: &'static str,
    runtime_url: String,
    health: VoiceRuntimeHealth,
    secrets_included: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceDevicesOutput {
    default_input: Option<String>,
    default_output: Option<String>,
    input_devices: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceTranscriptionOutput {
    status: &'static str,
    transcript: TranscriptionResult,
    stt_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceSynthesisOutput {
    status: &'static str,
    output: String,
    voice: String,
    speech_chars: usize,
    speech_truncated: bool,
    omitted_code: bool,
    omitted_links: bool,
    tts_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VoiceChatOutput {
    status: &'static str,
    transcript: String,
    response: String,
    output: Option<String>,
    voice: String,
    provider: String,
    model: String,
    offline: bool,
    speech_truncated: bool,
    omitted_code: bool,
    omitted_links: bool,
    stt_ms: u128,
    runtime_ms: u128,
    tts_ms: u128,
    total_ms: u128,
    audio_error: Option<String>,
}

#[derive(Debug)]
struct VoiceTurn {
    transcript: String,
    response: String,
    audio: Option<SynthesizedAudio>,
    audio_error: Option<String>,
    provider: String,
    model: String,
    offline: bool,
    speech_truncated: bool,
    omitted_code: bool,
    omitted_links: bool,
    stt_ms: u128,
    runtime_ms: u128,
    tts_ms: u128,
    total_ms: u128,
}

pub(crate) async fn run(
    command: VoiceCommand,
    runtime_url: Option<String>,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    json: bool,
) -> Result<()> {
    let mut voice_config = VoiceClientConfig::from_env();
    if let Some(runtime_url) = runtime_url {
        voice_config = voice_config.with_runtime_url(runtime_url);
    }
    let client = VoiceRuntimeClient::new(voice_config).context("invalid voice runtime config")?;
    match command {
        VoiceCommand::Doctor => run_doctor(&client, json).await,
        VoiceCommand::Devices => run_devices(json),
        VoiceCommand::Transcribe { input, language } => {
            run_transcribe(&client, &input, &language, json).await
        }
        VoiceCommand::Speak {
            text,
            output,
            voice,
            force,
        } => run_speak(&client, text.join(" "), &output, &voice, force, json).await,
        VoiceCommand::Chat {
            input,
            output,
            language,
            voice,
            force,
        } => {
            run_chat(
                &client,
                &input,
                &output,
                &language,
                &voice,
                force,
                config,
                backend,
                provider_mode,
                json,
            )
            .await
        }
        VoiceCommand::Talk {
            language,
            voice,
            input_device,
            no_playback,
            max_recording_seconds,
        } => {
            run_talk(
                &client,
                &language,
                &voice,
                input_device.as_deref(),
                no_playback,
                max_recording_seconds,
                config,
                backend,
                provider_mode,
                json,
            )
            .await
        }
    }
}

async fn run_doctor(client: &VoiceRuntimeClient, json: bool) -> Result<()> {
    let health = client
        .health()
        .await
        .context("voice runtime health check failed")?;
    if !health.ready() {
        bail!("voice runtime reported that STT or TTS is not ready");
    }
    let output = VoiceDoctorOutput {
        status: "ready",
        runtime_url: client.config().stt_base_url.clone(),
        health,
        secrets_included: false,
    };
    print_output(&output, json, || {
        println!("voice runtime: ready");
        println!("runtime_url: {}", output.runtime_url);
        println!(
            "stt: {} / {} / {}",
            output.health.stt.provider, output.health.stt.model, output.health.stt.device
        );
        println!(
            "tts: {} / {} / {}",
            output.health.tts.provider, output.health.tts.model, output.health.tts.device
        );
        if let Some(mode) = output.health.mode.as_deref() {
            println!("mode: {mode}");
        }
        if let Some(active) = output.health.active.as_ref() {
            println!("active: stt={} / tts={}", active.stt, active.tts);
        }
        if let Some(fallback) = output.health.fallback.as_ref() {
            println!("fallback_count: {}", fallback.count);
        }
        println!("preset_voices: {}", output.health.preset_voices.join(", "));
    })
}

fn run_devices(json: bool) -> Result<()> {
    let devices = audio_devices().context("failed to enumerate live audio devices")?;
    let output = VoiceDevicesOutput {
        default_input: devices.default_input,
        default_output: devices.default_output,
        input_devices: devices.input_devices,
    };
    print_output(&output, json, || {
        println!(
            "default input: {}",
            output.default_input.as_deref().unwrap_or("unavailable")
        );
        println!(
            "default output: {}",
            output.default_output.as_deref().unwrap_or("unavailable")
        );
        println!("microphones:");
        for device in &output.input_devices {
            println!("- {device}");
        }
    })
}

async fn run_transcribe(
    client: &VoiceRuntimeClient,
    input: &Path,
    language: &str,
    json: bool,
) -> Result<()> {
    let audio = AudioInput::from_wav_file(input, client.config().max_input_bytes)
        .context("failed to load voice input")?;
    let started = Instant::now();
    let transcript = client
        .transcribe(audio, normalized_language(language))
        .await
        .context("voice transcription failed")?;
    let output = VoiceTranscriptionOutput {
        status: "completed",
        transcript,
        stt_ms: started.elapsed().as_millis(),
    };
    print_output(&output, json, || {
        println!("{}", output.transcript.text);
        println!("stt_ms: {}", output.stt_ms);
    })
}

async fn run_speak(
    client: &VoiceRuntimeClient,
    text: String,
    output_path: &Path,
    voice: &str,
    force: bool,
    json: bool,
) -> Result<()> {
    let speech = render_spoken_text(&text, client.config().max_speech_chars)
        .context("failed to prepare speech text")?;
    let started = Instant::now();
    let audio = client
        .synthesize(SynthesisRequest {
            text: speech.text.clone(),
            voice: voice.to_string(),
            format: "wav".to_string(),
        })
        .await
        .context("voice synthesis failed")?;
    let output_path = write_wav_atomic(
        output_path,
        &audio.bytes,
        client.config().max_output_bytes,
        force,
    )
    .context("failed to save synthesized voice")?;
    let output = VoiceSynthesisOutput {
        status: "completed",
        output: output_path.display().to_string(),
        voice: voice.to_string(),
        speech_chars: speech.text.chars().count(),
        speech_truncated: speech.truncated,
        omitted_code: speech.omitted_code,
        omitted_links: speech.omitted_links,
        tts_ms: started.elapsed().as_millis(),
    };
    print_output(&output, json, || {
        println!("voice output: {}", output.output);
        println!("voice: {}", output.voice);
        println!("tts_ms: {}", output.tts_ms);
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_chat(
    client: &VoiceRuntimeClient,
    input_path: &Path,
    output_path: &Path,
    language: &str,
    voice: &str,
    force: bool,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    json: bool,
) -> Result<()> {
    let audio = AudioInput::from_wav_file(input_path, client.config().max_input_bytes)
        .context("failed to load voice input")?;
    let turn = execute_voice_turn(
        client,
        audio,
        language,
        voice,
        config,
        backend,
        provider_mode,
        json,
    )
    .await?;
    let saved_output = if let Some(audio) = &turn.audio {
        Some(
            write_wav_atomic(
                output_path,
                &audio.bytes,
                client.config().max_output_bytes,
                force,
            )
            .context("failed to save voice reply")?
            .display()
            .to_string(),
        )
    } else {
        None
    };
    let output = VoiceChatOutput {
        status: if turn.audio_error.is_some() {
            "text_completed_audio_failed"
        } else {
            "completed"
        },
        transcript: turn.transcript,
        response: turn.response,
        output: saved_output,
        voice: voice.to_string(),
        provider: turn.provider,
        model: turn.model,
        offline: turn.offline,
        speech_truncated: turn.speech_truncated,
        omitted_code: turn.omitted_code,
        omitted_links: turn.omitted_links,
        stt_ms: turn.stt_ms,
        runtime_ms: turn.runtime_ms,
        tts_ms: turn.tts_ms,
        total_ms: turn.total_ms,
        audio_error: turn.audio_error.clone(),
    };
    print_chat_output(&output, json)?;
    if let Some(error) = turn.audio_error {
        bail!("voice synthesis failed after the text reply completed: {error}");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn execute_voice_turn(
    client: &VoiceRuntimeClient,
    audio: AudioInput,
    language: &str,
    voice: &str,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    json: bool,
) -> Result<VoiceTurn> {
    let total_started = Instant::now();
    let stt_started = Instant::now();
    let transcript = client
        .transcribe(audio, normalized_language(language))
        .await
        .context("voice transcription failed")?;
    let stt_ms = stt_started.elapsed().as_millis();

    let invocation = crate::prepare_runtime_invocation(config, backend, provider_mode)?;
    crate::print_provider_selection_warning(&invocation.selection, json, false)?;
    let provider = invocation.selection.provider.clone();
    let model = invocation.selection.model.clone();
    let offline = invocation.selection.is_offline_runtime();
    let runtime_started = Instant::now();
    let result = crate::run_agent_backend(
        backend,
        invocation.config,
        transcript.text.trim().to_string(),
        invocation.selection.live,
    )
    .await?;
    let runtime_ms = runtime_started.elapsed().as_millis();
    if result.status != AgentRunStatus::Completed {
        bail!("voice agent turn did not complete: {:?}", result.status);
    }
    let response = result
        .final_response
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow::anyhow!("voice agent turn completed without a final response"))?;
    let speech = render_spoken_text(&response, client.config().max_speech_chars)
        .context("failed to prepare assistant speech")?;

    let tts_started = Instant::now();
    let (audio, audio_error) = match client
        .synthesize(SynthesisRequest {
            text: speech.text.clone(),
            voice: voice.to_string(),
            format: "wav".to_string(),
        })
        .await
    {
        Ok(audio) => (Some(audio), None),
        Err(error) => (None, Some(error.to_string())),
    };
    let tts_ms = tts_started.elapsed().as_millis();

    Ok(VoiceTurn {
        transcript: transcript.text,
        response,
        audio,
        audio_error,
        provider,
        model,
        offline,
        speech_truncated: speech.truncated,
        omitted_code: speech.omitted_code,
        omitted_links: speech.omitted_links,
        stt_ms,
        runtime_ms,
        tts_ms,
        total_ms: total_started.elapsed().as_millis(),
    })
}

#[allow(clippy::too_many_arguments)]
async fn run_talk(
    client: &VoiceRuntimeClient,
    language: &str,
    voice: &str,
    input_device: Option<&str>,
    no_playback: bool,
    max_recording_seconds: u32,
    config: AgentConfig,
    backend: BackendKind,
    provider_mode: ProviderMode,
    json: bool,
) -> Result<()> {
    if json {
        bail!("voice talk is interactive and does not support --json");
    }
    if !io::stdin().is_terminal() || !io::stdout().is_terminal() {
        bail!("voice talk requires an interactive terminal");
    }
    if !(1..=300).contains(&max_recording_seconds) {
        bail!("--max-recording-seconds must be between 1 and 300");
    }
    let health = client
        .health()
        .await
        .context("voice runtime health check failed")?;
    if !health.ready() {
        bail!("voice runtime reported that STT or TTS is not ready");
    }
    let devices = audio_devices().context("failed to enumerate live audio devices")?;
    let selected_input = input_device
        .map(str::to_string)
        .or_else(|| devices.default_input.clone())
        .unwrap_or_else(|| "unavailable".to_string());
    println!("实时语音已就绪");
    println!("麦克风: {selected_input}");
    println!("音色: {voice}");
    if no_playback {
        println!("播放: 已关闭");
    } else {
        println!(
            "播放: {}",
            devices.default_output.as_deref().unwrap_or("unavailable")
        );
    }

    let session_prefix = talk_session_prefix();
    let mut parent_session_id: Option<String> = None;
    let mut turn_number = 0_u64;
    loop {
        let Some(command) = read_console_command("\n按 Enter 开始说话，输入 q 退出: ")?
        else {
            break;
        };
        if is_quit_command(&command) {
            break;
        }
        let recording = match LiveRecording::start(input_device, max_recording_seconds) {
            Ok(recording) => recording,
            Err(error) => {
                eprintln!("无法启动麦克风: {error}");
                continue;
            }
        };
        let Some(stop_command) = read_console_command("正在听，再按 Enter 结束: ")? else {
            drop(recording);
            break;
        };
        if is_quit_command(&stop_command) {
            drop(recording);
            break;
        }
        let captured = match recording.finish(client.config().max_input_bytes) {
            Ok(captured) => captured,
            Err(error) => {
                eprintln!("这段录音没有送出: {error}");
                continue;
            }
        };
        if captured.reached_limit {
            eprintln!(
                "录音达到 {} 秒上限，已使用上限前的内容。",
                max_recording_seconds
            );
        }
        println!(
            "已录制 {:.1} 秒，正在识别...",
            captured.duration_ms as f64 / 1_000.0
        );

        turn_number = turn_number.saturating_add(1);
        let session_id = format!("{session_prefix}-{turn_number:04}");
        let mut turn_config = config
            .clone()
            .with_session_id(session_id.clone())
            .with_session_title("云熙实时语音");
        if let Some(parent) = &parent_session_id {
            turn_config = turn_config.with_parent_session_id(parent.clone());
        }
        let turn = match execute_voice_turn(
            client,
            captured.input,
            language,
            voice,
            turn_config,
            backend,
            provider_mode,
            false,
        )
        .await
        {
            Ok(turn) => turn,
            Err(error) => {
                eprintln!("这一轮没有完成: {error:#}");
                continue;
            }
        };
        parent_session_id = Some(session_id);
        println!("你: {}", turn.transcript);
        println!("云熙: {}", turn.response);
        println!(
            "耗时: 识别 {}ms · 回复 {}ms · 合成 {}ms · 总计 {}ms",
            turn.stt_ms, turn.runtime_ms, turn.tts_ms, turn.total_ms
        );
        if let Some(error) = &turn.audio_error {
            eprintln!("语音播放不可用，文字回复已保留: {error}");
            continue;
        }
        if !no_playback
            && let Some(audio) = &turn.audio
            && let Err(error) = play_wav(&audio.bytes)
        {
            eprintln!("播放失败，文字回复已保留: {error}");
        }
    }
    println!("实时语音已结束");
    Ok(())
}

fn read_console_command(prompt: &str) -> Result<Option<String>> {
    print!("{prompt}");
    io::stdout()
        .flush()
        .context("failed to flush voice prompt")?;
    let mut input = String::new();
    let read = io::stdin()
        .read_line(&mut input)
        .context("failed to read voice command")?;
    Ok((read != 0).then(|| input.trim().to_string()))
}

fn is_quit_command(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "q" | "quit" | "exit" | "退出"
    )
}

fn talk_session_prefix() -> String {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("voice-talk-{}-{timestamp}", std::process::id())
}

fn print_chat_output(output: &VoiceChatOutput, json: bool) -> Result<()> {
    print_output(output, json, || {
        println!("transcript: {}", output.transcript);
        println!("response: {}", output.response);
        if let Some(path) = &output.output {
            println!("voice output: {path}");
        } else {
            println!("voice output: unavailable");
        }
        println!(
            "timings_ms: stt={} runtime={} tts={} total={}",
            output.stt_ms, output.runtime_ms, output.tts_ms, output.total_ms
        );
    })
}

fn print_output<T, F>(output: &T, json: bool, plain: F) -> Result<()>
where
    T: Serialize,
    F: FnOnce(),
{
    if json {
        println!("{}", serde_json::to_string_pretty(output)?);
    } else {
        plain();
    }
    Ok(())
}

fn normalized_language(language: &str) -> Option<&str> {
    let language = language.trim();
    (!language.is_empty() && !language.eq_ignore_ascii_case("auto")).then_some(language)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_language_omits_explicit_hint() {
        assert_eq!(normalized_language("auto"), None);
        assert_eq!(normalized_language(" AUTO "), None);
        assert_eq!(normalized_language("zh"), Some("zh"));
    }

    #[test]
    fn quit_commands_accept_supported_console_forms() {
        for value in ["q", " Q ", "quit", "EXIT", "退出"] {
            assert!(is_quit_command(value), "expected quit command: {value:?}");
        }
        for value in ["", "continue", "退出一下"] {
            assert!(
                !is_quit_command(value),
                "unexpected quit command: {value:?}"
            );
        }
    }

    #[test]
    fn talk_session_prefix_is_namespaced_for_voice() {
        let prefix = talk_session_prefix();
        assert!(prefix.starts_with(&format!("voice-talk-{}-", std::process::id())));
        assert!(
            prefix
                .rsplit_once('-')
                .is_some_and(|(_, timestamp)| timestamp.parse::<u128>().is_ok())
        );
    }
}
