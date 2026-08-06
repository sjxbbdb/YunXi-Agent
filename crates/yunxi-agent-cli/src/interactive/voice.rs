use super::InteractiveSession;
use crate::input::InteractiveInput;
use crate::render::InteractiveRenderer;
use anyhow::Result;
use std::time::{Duration, Instant};
use yunxi_agent_core::AgentInputModality;
use yunxi_agent_voice::{
    CapturedAudio, DEFAULT_PRESET_VOICE, LiveRecording, PlaybackCancellationToken,
    SpeechToTextProvider, SynthesizedAudio, SynthesisRequest, TextToSpeechProvider,
    VoiceActivityConfig, VoiceClientConfig, VoiceRuntimeClient, audio_devices,
    play_wav_cancellable, render_spoken_chunks,
};

const MAX_RECORDING_SECONDS: u32 = 60;
const MAX_SPEECH_CHUNK_CHARS: usize = 90;
const PLAYBACK_CONTROL_POLL: Duration = Duration::from_millis(40);

pub(super) async fn handle_voice_command(
    session: &mut InteractiveSession,
    action: Option<String>,
    input: &mut dyn InteractiveInput,
    renderer: &mut dyn InteractiveRenderer,
) -> Result<()> {
    let action = action
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase);
    match action.as_deref() {
        None | Some("") | Some("on") | Some("start") => {
            run_voice_mode(session, input, renderer).await
        }
        Some("status") | Some("realtime") | Some("realtime status") => {
            show_voice_status(session, renderer).await
        }
        Some("devices") => show_audio_devices(renderer),
        Some("realtime on") => run_realtime_voice_mode(session, input, renderer).await,
        Some("off") | Some("realtime off") => {
            set_realtime_voice_state(session, renderer, false)?;
            renderer.notice("voice", "实时语音已关闭")?;
            Ok(())
        }
        Some(other) => {
            renderer.notice(
                "voice",
                &format!(
                    "未知语音命令: {other}；可用 /voice、/voice status、/voice devices、/voice realtime on|off|status"
                ),
            )?;
            Ok(())
        }
    }
}

async fn show_voice_status(
    session: &InteractiveSession,
    renderer: &mut dyn InteractiveRenderer,
) -> Result<()> {
    let client = match voice_client(renderer)? {
        Some(client) => client,
        None => return Ok(()),
    };
    match client.health().await {
        Ok(health) if health.ready() => renderer.notice(
            "voice",
            &format!(
                "语音运行时已就绪\nSTT: {} / {} / {}\nTTS: {} / {} / {}",
                health.stt.provider,
                health.stt.model,
                health.stt.device,
                health.tts.provider,
                health.tts.model,
                health.tts.device,
            ),
        )?,
        Ok(health) => renderer.warning(&format!(
            "语音运行时未就绪: status={} stt_ready={} tts_ready={}",
            health.status, health.stt.ready, health.tts.ready
        ))?,
        Err(error) => renderer.error(&format!("语音运行时检查失败: {error}"))?,
    }
    renderer.notice(
        "voice",
        if session.realtime_voice_enabled {
            "实时语音: 开启"
        } else {
            "实时语音: 关闭"
        },
    )?;
    Ok(())
}

async fn run_voice_mode(
    session: &mut InteractiveSession,
    input: &mut dyn InteractiveInput,
    renderer: &mut dyn InteractiveRenderer,
) -> Result<()> {
        let client = match voice_client(renderer)? {
            Some(client) => client,
            None => return Ok(()),
        };
        match client.health().await {
            Ok(health) if health.ready() => {},
            Ok(health) => {
                renderer.error(&format!(
                    "语音运行时未就绪: status={} stt_ready={} tts_ready={}",
                    health.status, health.stt.ready, health.tts.ready
                ))?;
                return Ok(());
            }
            Err(error) => {
                renderer.error(&format!("语音运行时检查失败: {error}"))?;
                return Ok(());
            }
        }
        let devices = match audio_devices() {
            Ok(devices) => devices,
            Err(error) => {
                renderer.error(&format!("音频设备枚举失败: {error}"))?;
                return Ok(());
            }
        };
        renderer.notice(
            "voice",
            &format!(
                "语音模式已启动\n麦克风: {}\n音色: {}\n扬声器: {}\n输入 q 返回文字模式",
                devices.default_input.as_deref().unwrap_or("不可用"),
                DEFAULT_PRESET_VOICE,
                devices.default_output.as_deref().unwrap_or("不可用"),
            ),
        )?;

        loop {
            let Some(start_command) = input.read_response("按 Enter 开始说话，输入 q 返回文字模式: ")?
            else {
                return Ok(());
            };
            if voice_quit_command(&start_command) {
                break;
            }

            let recording = match LiveRecording::start(None, MAX_RECORDING_SECONDS) {
                Ok(recording) => recording,
                Err(error) => {
                    renderer.error(&format!("无法启动麦克风: {error}"))?;
                    continue;
                }
            };
            renderer.notice("voice", "正在听，再按 Enter 结束；输入 q 取消本轮并返回文字模式")?;
            let Some(stop_command) = input.read_response("正在听，再按 Enter 结束: ")? else {
                drop(recording);
                return Ok(());
            };
            if voice_quit_command(&stop_command) {
                drop(recording);
                break;
            }
            let captured = match recording.finish(client.config().max_input_bytes) {
                Ok(captured) => captured,
                Err(error) => {
                    renderer.error(&format!("这段录音没有送出: {error}"))?;
                    continue;
                }
            };
            let _ = process_captured_audio(session, input, renderer, &client, captured, false).await?;
        }
        renderer.notice("voice", "语音模式已结束，已回到文字模式")?;
    Ok(())
}

async fn run_realtime_voice_mode(
    session: &mut InteractiveSession,
    input: &mut dyn InteractiveInput,
    renderer: &mut dyn InteractiveRenderer,
) -> Result<()> {
    if session.realtime_voice_enabled {
        renderer.notice("voice", "实时语音已经开启")?;
        return Ok(());
    }
    let client = match voice_client(renderer)? {
        Some(client) => client,
        None => return Ok(()),
    };
    match client.health().await {
        Ok(health) if health.ready() => {}
        Ok(health) => {
            renderer.error(&format!(
                "语音运行时未就绪: status={} stt_ready={} tts_ready={}",
                health.status, health.stt.ready, health.tts.ready
            ))?;
            return Ok(());
        }
        Err(error) => {
            renderer.error(&format!("语音运行时检查失败: {error}"))?;
            return Ok(());
        }
    }
    let devices = match audio_devices() {
        Ok(devices) => devices,
        Err(error) => {
            renderer.error(&format!("音频设备枚举失败: {error}"))?;
            return Ok(());
        }
    };

    set_realtime_voice_state(session, renderer, true)?;
    renderer.notice(
        "voice",
        &format!(
            "实时语音已开启\n麦克风: {}\n音色: {}\n扬声器: {}\n直接说话即可；说“关闭实时语音”返回文字模式",
            devices.default_input.as_deref().unwrap_or("不可用"),
            DEFAULT_PRESET_VOICE,
            devices.default_output.as_deref().unwrap_or("不可用"),
        ),
    )?;
    renderer.flush()?;

    while session.realtime_voice_enabled {
        renderer.notice("voice", "正在等待你说话...")?;
        renderer.flush()?;
        let recording = match LiveRecording::start(None, MAX_RECORDING_SECONDS) {
            Ok(recording) => recording,
            Err(error) => {
                renderer.error(&format!("无法启动麦克风: {error}"))?;
                break;
            }
        };
        let mut stop_requested = false;
        let mut control_error = None;
        let captured = match recording.capture_until_silence_with_control(
            client.config().max_input_bytes,
            VoiceActivityConfig::default(),
            || match renderer.poll_realtime_voice_stop() {
                Ok(true) => {
                    stop_requested = true;
                    true
                }
                Ok(false) => false,
                Err(error) => {
                    control_error = Some(error);
                    true
                }
            },
        ) {
            Ok(Some(captured)) => captured,
            Ok(None) if stop_requested => break,
            Ok(None) => {
                if let Some(error) = control_error {
                    renderer.error(&format!("实时语音控制失败: {error:#}"))?;
                    break;
                }
                continue;
            }
            Err(error) => {
                renderer.error(&format!("自动收音失败: {error}"))?;
                continue;
            }
        };
        if process_captured_audio(session, input, renderer, &client, captured, true).await?
            == VoiceTurnOutcome::StopRealtime
        {
            break;
        }
    }

    set_realtime_voice_state(session, renderer, false)?;
    renderer.notice("voice", "实时语音已关闭，已回到文字模式")?;
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VoiceTurnOutcome {
    Continue,
    StopRealtime,
}

async fn process_captured_audio(
    session: &mut InteractiveSession,
    input: &mut dyn InteractiveInput,
    renderer: &mut dyn InteractiveRenderer,
    client: &VoiceRuntimeClient,
    captured: CapturedAudio,
    realtime: bool,
) -> Result<VoiceTurnOutcome> {
    if captured.reached_limit {
        renderer.warning(&format!(
            "录音达到 {MAX_RECORDING_SECONDS} 秒上限，已使用上限前的内容"
        ))?;
    }
    renderer.notice(
        "voice",
        &format!(
            "已录制 {:.1} 秒，正在识别...",
            captured.duration_ms as f64 / 1_000.0
        ),
    )?;

    let stt_started = Instant::now();
    let transcript = match client.transcribe(captured.input, None).await {
        Ok(transcript) => transcript,
        Err(error) => {
            renderer.error(&format!("语音识别失败: {error}"))?;
            return Ok(VoiceTurnOutcome::Continue);
        }
    };
    let stt_ms = stt_started.elapsed().as_millis();
    let transcript = transcript.text.trim();
    if transcript.is_empty() {
        renderer.warning("没有识别到有效语音内容")?;
        return Ok(VoiceTurnOutcome::Continue);
    }
    renderer.user_message(transcript)?;
    if realtime && voice_realtime_stop_command(transcript) {
        session.realtime_voice_enabled = false;
        return Ok(VoiceTurnOutcome::StopRealtime);
    }

    let runtime_started = Instant::now();
    let response = match session
        .run_turn_with_modality(
            transcript.to_string(),
            AgentInputModality::Voice,
            input,
            renderer,
        )
        .await
    {
        Ok(response) => response,
        Err(error) => {
            renderer.error(&format!("语音这一轮没有完成: {error:#}"))?;
            return Ok(VoiceTurnOutcome::Continue);
        }
    };
    let runtime_ms = runtime_started.elapsed().as_millis();
    let Some(response) = response else {
        renderer.notice("voice", &format!("本轮没有最终回复（识别 {stt_ms}ms）"))?;
        return Ok(VoiceTurnOutcome::Continue);
    };

    let outcome = match synthesize_and_play_response(
        renderer,
        client,
        &response,
        realtime,
        stt_ms,
        runtime_ms,
    )
    .await
    {
        Ok(outcome) => outcome,
        Err(error) => {
            renderer.warning(&format!("语音播放失败，文字回复已保留: {error:#}"))?;
            return Ok(VoiceTurnOutcome::Continue);
        }
    };
    if outcome == VoiceTurnOutcome::StopRealtime {
        session.realtime_voice_enabled = false;
    }
    Ok(outcome)
}

async fn synthesize_and_play_response(
    renderer: &mut dyn InteractiveRenderer,
    client: &VoiceRuntimeClient,
    response: &str,
    realtime: bool,
    stt_ms: u128,
    runtime_ms: u128,
) -> Result<VoiceTurnOutcome> {
    let speech = render_spoken_chunks(
        response,
        client.config().max_speech_chars,
        MAX_SPEECH_CHUNK_CHARS,
    )?;
    let total_chunks = speech.chunks.len();
    let first_chunk = speech
        .chunks
        .first()
        .expect("speakable text always produces at least one chunk");
    let first_tts_started = Instant::now();
    let mut current_audio = synthesize_chunk(client, first_chunk.clone(), realtime).await?;
    let first_tts_ms = first_tts_started.elapsed().as_millis();
    let mut current_tts_ms = first_tts_ms;

    for index in 0..total_chunks {
        renderer.notice("voice", &playback_status(
            index,
            total_chunks,
            stt_ms,
            runtime_ms,
            current_tts_ms,
        ))?;

        let cancellation = PlaybackCancellationToken::new();
        let playback_cancellation = cancellation.clone();
        let audio_bytes = current_audio.bytes;
        let mut playback = Box::pin(tokio::task::spawn_blocking(move || {
            play_wav_cancellable(&audio_bytes, &playback_cancellation)
        }));
        let next_chunk = speech.chunks.get(index + 1).cloned();
        let next_client = client.clone();
        let next_realtime = realtime;
        let mut next_synthesis = Box::pin(async move {
            match next_chunk {
                Some(chunk) => {
                    let started = Instant::now();
                    let result = synthesize_chunk(&next_client, chunk, next_realtime).await;
                    Some((result, started.elapsed().as_millis()))
                }
                None => None,
            }
        });
        let mut playback_result = None;
        let mut next_result = None;
        let mut stop_requested = false;
        let mut control_error = None;

        loop {
            tokio::select! {
                result = &mut playback, if playback_result.is_none() => {
                    playback_result = Some(result);
                }
                result = &mut next_synthesis, if next_result.is_none() => {
                    next_result = Some(result);
                }
                _ = tokio::time::sleep(PLAYBACK_CONTROL_POLL), if realtime && (!stop_requested) => {
                    match renderer.poll_realtime_voice_stop() {
                        Ok(true) => {
                            stop_requested = true;
                            cancellation.cancel();
                        }
                        Ok(false) => {}
                        Err(error) => {
                            control_error = Some(error);
                            stop_requested = true;
                            cancellation.cancel();
                        }
                    }
                }
            }
            if stop_requested && playback_result.is_some() {
                break;
            }
            if playback_result.is_some() && next_result.is_some() {
                break;
            }
        }

        match playback_result.expect("playback completes before segment advances") {
            Ok(Ok(_)) => {}
            Ok(Err(error)) => renderer.warning(&format!("播放第 {} 段失败: {error}", index + 1))?,
            Err(error) => renderer.warning(&format!("播放第 {} 段任务失败: {error}", index + 1))?,
        }
        if let Some(error) = control_error {
            return Err(error);
        }
        if stop_requested {
            return Ok(VoiceTurnOutcome::StopRealtime);
        }

        match next_result.expect("next synthesis settles before segment advances") {
            Some((Ok(audio), synthesis_ms)) => {
                current_audio = audio;
                current_tts_ms = synthesis_ms;
            }
            Some((Err(error), _)) => {
                renderer.warning(&format!(
                    "第 {} 段语音合成失败，剩余内容保留在文字回复中: {error}",
                    index + 2
                ))?;
                break;
            }
            None => break,
        }
    }
    Ok(VoiceTurnOutcome::Continue)
}

fn playback_status(
    index: usize,
    total_chunks: usize,
    stt_ms: u128,
    runtime_ms: u128,
    synthesis_ms: u128,
) -> String {
    if index == 0 {
        format!(
            "正在播放云熙的回复 1/{total_chunks}（识别 {stt_ms}ms · 回复 {runtime_ms}ms · 首段合成 {synthesis_ms}ms）"
        )
    } else {
        format!(
            "正在播放云熙的回复 {}/{}（本段预取合成 {synthesis_ms}ms）",
            index + 1,
            total_chunks
        )
    }
}

async fn synthesize_chunk(
    client: &VoiceRuntimeClient,
    text: String,
    realtime: bool,
) -> yunxi_agent_voice::VoiceResult<SynthesizedAudio> {
    client
        .synthesize(SynthesisRequest {
            text,
            voice: DEFAULT_PRESET_VOICE.to_string(),
            format: "wav".to_string(),
            realtime,
        })
        .await
}

fn set_realtime_voice_state(
    session: &mut InteractiveSession,
    renderer: &mut dyn InteractiveRenderer,
    enabled: bool,
) -> Result<()> {
    session.realtime_voice_enabled = enabled;
    renderer.realtime_voice_state(enabled)
}

fn voice_client(renderer: &mut dyn InteractiveRenderer) -> Result<Option<VoiceRuntimeClient>> {
    match VoiceRuntimeClient::new(VoiceClientConfig::from_env()) {
        Ok(client) => Ok(Some(client)),
        Err(error) => {
            renderer.error(&format!("语音运行时配置无效: {error}"))?;
            Ok(None)
        }
    }
}

fn show_audio_devices(renderer: &mut dyn InteractiveRenderer) -> Result<()> {
    match audio_devices() {
        Ok(devices) => renderer.notice(
            "voice",
            &format!(
                "麦克风: {}\n扬声器: {}\n可用麦克风: {}",
                devices.default_input.as_deref().unwrap_or("不可用"),
                devices.default_output.as_deref().unwrap_or("不可用"),
                if devices.input_devices.is_empty() {
                    "无".to_string()
                } else {
                    devices.input_devices.join("、")
                },
            ),
        ),
        Err(error) => {
            renderer.error(&format!("音频设备枚举失败: {error}"))?;
            Ok(())
        }
    }
}

fn voice_quit_command(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "q" | "quit" | "exit" | "退出" | "/exit" | "/quit"
    )
}

fn voice_realtime_stop_command(value: &str) -> bool {
    let compact = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|character| {
            !character.is_whitespace()
                && !matches!(character, '。' | '，' | ',' | '.' | '！' | '!' | '？' | '?')
        })
        .collect::<String>();
    let compact = compact.replace("实施语音", "实时语音");
    [
        "关闭实时语音",
        "关闭实时语音聊天",
        "关闭实时语音模式",
        "停止实时语音",
        "停止实时语音聊天",
        "停止实时语音模式",
        "退出实时语音",
        "退出实时语音聊天",
        "退出实时语音模式",
        "结束实时语音",
        "结束实时语音聊天",
        "结束实时语音模式",
        "把实时语音关了",
        "把实时语音关掉",
        "关闭语音",
        "关闭语音模式",
        "停止语音",
        "停止语音模式",
        "退出语音",
        "退出语音模式",
        "结束语音",
        "结束语音模式",
        "把语音关了",
        "把语音关掉",
        "回到文字模式",
        "返回文字模式",
        "切回文字模式",
        "stoprealtimevoice",
        "exitrealtimevoice",
        "turnoffrealtimevoice",
        "stopvoicechat",
        "exitvoicemode",
        "switchtotextmode",
    ]
    .iter()
    .any(|command| compact == *command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn playback_status_distinguishes_first_chunk_from_prefetched_chunks() {
        assert_eq!(
            playback_status(0, 3, 62, 1_800, 450),
            "正在播放云熙的回复 1/3（识别 62ms · 回复 1800ms · 首段合成 450ms）"
        );
        assert_eq!(
            playback_status(1, 3, 62, 1_800, 320),
            "正在播放云熙的回复 2/3（本段预取合成 320ms）"
        );
    }

    #[test]
    fn voice_mode_quit_commands_are_explicit() {
        for value in ["q", " Q ", "quit", "EXIT", "退出", "/exit", "/QUIT"] {
            assert!(voice_quit_command(value), "expected voice quit: {value:?}");
        }
        for value in ["", "continue", "退出一下", "/voice"] {
            assert!(
                !voice_quit_command(value),
                "unexpected voice quit: {value:?}"
            );
        }
    }

    #[test]
    fn realtime_voice_stop_commands_require_an_explicit_phrase() {
        for value in [
            "关闭实时语音",
            "关闭实时语音聊天。",
            " 停止 实时语音 ",
            "退出实施语音。",
            "把语音关了",
            "回到文字模式",
            "EXIT REALTIME VOICE",
        ] {
            assert!(
                voice_realtime_stop_command(value),
                "expected realtime stop: {value:?}"
            );
        }
        for value in [
            "实时语音",
            "如何关闭语音",
            "请不要关闭语音",
            "别说话",
            "继续聊天",
        ] {
            assert!(
                !voice_realtime_stop_command(value),
                "unexpected realtime stop: {value:?}"
            );
        }
    }
}
