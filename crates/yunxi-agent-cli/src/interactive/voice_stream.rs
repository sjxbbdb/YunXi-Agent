use std::collections::{HashMap, HashSet};
use std::time::Instant;

use tokio::sync::mpsc;
use yunxi_agent_core::{AgentEvent, AgentMessageStreamPhase};
use yunxi_agent_voice::{
    DEFAULT_PRESET_VOICE, PlaybackCancellationToken, SynthesisRequest, TextToSpeechProvider,
    VoiceRuntimeClient, play_wav_sequence_cancellable, render_spoken_text,
};

const FIRST_SPEECH_CHUNK_CHARS: usize = 48;
const FOLLOWING_SPEECH_CHUNK_CHARS: usize = 112;
const MINIMUM_SENTENCE_CHARS: usize = 4;
const FIRST_CLAUSE_CHARS: usize = 36;
const FOLLOWING_SENTENCE_CHARS: usize = 32;
const FOLLOWING_SEMICOLON_CHARS: usize = 64;
const SEGMENT_QUEUE_CAPACITY: usize = 2;
const AUDIO_QUEUE_CAPACITY: usize = 1;
const PLAYBACK_QUEUE_CHUNKS: usize = 2;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct StreamingSpeechReport {
    pub chunks: usize,
    pub first_sentence_ms: Option<u128>,
    pub first_tts_ms: Option<u128>,
    pub first_audio_ms: Option<u128>,
    pub error: Option<String>,
}

#[derive(Debug)]
struct QueuedSpeech {
    text: String,
    ready_ms: u128,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct SynthesisReport {
    chunks: usize,
    first_sentence_ms: Option<u128>,
    first_tts_ms: Option<u128>,
    first_audio_ms: Option<u128>,
    error: Option<String>,
}

pub(super) async fn run_streaming_speech_pipeline(
    client: VoiceRuntimeClient,
    mut events: mpsc::UnboundedReceiver<AgentEvent>,
    realtime: bool,
    cancellation: PlaybackCancellationToken,
) -> StreamingSpeechReport {
    let started = Instant::now();
    let maximum_speech_chars = client.config().max_speech_chars;
    let (segment_tx, segment_rx) = mpsc::channel(SEGMENT_QUEUE_CAPACITY);
    let (audio_tx, audio_rx) = mpsc::channel(AUDIO_QUEUE_CAPACITY);

    let playback_cancellation = cancellation.clone();
    let playback = tokio::task::spawn_blocking(move || {
        play_wav_sequence_cancellable(
            audio_rx,
            &playback_cancellation,
            PLAYBACK_QUEUE_CHUNKS,
        )
    });
    let synthesis_cancellation = cancellation.clone();
    let synthesis = tokio::spawn(synthesize_segments(
        client,
        segment_rx,
        audio_tx,
        realtime,
        started,
        synthesis_cancellation,
    ));

    let mut collector = StreamingSpeechCollector::new(maximum_speech_chars);
    while let Some(event) = events.recv().await {
        if cancellation.is_cancelled() {
            break;
        }
        for text in collector.push_event(&event) {
            let queued = QueuedSpeech {
                text,
                ready_ms: started.elapsed().as_millis(),
            };
            if segment_tx.send(queued).await.is_err() {
                break;
            }
        }
    }
    if !cancellation.is_cancelled() {
        for text in collector.finish() {
            let queued = QueuedSpeech {
                text,
                ready_ms: started.elapsed().as_millis(),
            };
            if segment_tx.send(queued).await.is_err() {
                break;
            }
        }
    }
    drop(segment_tx);

    let synthesis_report = match synthesis.await {
        Ok(report) => report,
        Err(error) => SynthesisReport {
            error: Some(format!("语音预合成任务失败: {error}")),
            ..SynthesisReport::default()
        },
    };
    let playback_error = match playback.await {
        Ok(Ok(_)) => None,
        Ok(Err(error)) => Some(format!("连续播放失败: {error}")),
        Err(error) => Some(format!("连续播放任务失败: {error}")),
    };

    StreamingSpeechReport {
        chunks: synthesis_report.chunks,
        first_sentence_ms: synthesis_report.first_sentence_ms,
        first_tts_ms: synthesis_report.first_tts_ms,
        first_audio_ms: synthesis_report.first_audio_ms,
        error: synthesis_report.error.or(playback_error),
    }
}

async fn synthesize_segments(
    client: VoiceRuntimeClient,
    mut segments: mpsc::Receiver<QueuedSpeech>,
    audio_tx: mpsc::Sender<Vec<u8>>,
    realtime: bool,
    started: Instant,
    cancellation: PlaybackCancellationToken,
) -> SynthesisReport {
    let mut report = SynthesisReport::default();
    while let Some(segment) = segments.recv().await {
        if cancellation.is_cancelled() {
            break;
        }
        let tts_started = Instant::now();
        let audio = match client
            .synthesize(SynthesisRequest {
                text: segment.text,
                voice: DEFAULT_PRESET_VOICE.to_string(),
                format: "wav".to_string(),
                realtime,
            })
            .await
        {
            Ok(audio) => audio,
            Err(error) => {
                report.error = Some(format!("语音预合成失败: {error}"));
                break;
            }
        };
        if cancellation.is_cancelled() {
            break;
        }
        let tts_ms = tts_started.elapsed().as_millis();
        if report.first_sentence_ms.is_none() {
            report.first_sentence_ms = Some(segment.ready_ms);
            report.first_tts_ms = Some(tts_ms);
            report.first_audio_ms = Some(started.elapsed().as_millis());
        }
        if audio_tx.send(audio.bytes).await.is_err() {
            report.error = Some("连续播放队列提前关闭".to_string());
            break;
        }
        report.chunks += 1;
    }
    report
}

#[derive(Clone, Debug)]
struct StreamingSpeechCollector {
    segmenter: StreamingSpeechSegmenter,
    event_ids: HashSet<String>,
    delta_streams: HashSet<String>,
    stream_texts: HashMap<String, String>,
    completed_messages: HashSet<String>,
}

impl StreamingSpeechCollector {
    fn new(maximum_chars: usize) -> Self {
        Self {
            segmenter: StreamingSpeechSegmenter::new(maximum_chars),
            event_ids: HashSet::new(),
            delta_streams: HashSet::new(),
            stream_texts: HashMap::new(),
            completed_messages: HashSet::new(),
        }
    }

    fn push_event(&mut self, event: &AgentEvent) -> Vec<String> {
        let AgentEvent::Message { content, stream } = event else {
            return Vec::new();
        };
        if content.trim().is_empty() {
            return Vec::new();
        }

        let Some(stream) = stream else {
            if self.message_was_seen(content) {
                return Vec::new();
            }
            self.completed_messages.insert(content.clone());
            return self.segmenter.push(content);
        };
        if !self.event_ids.insert(stream.event_id.clone()) {
            return Vec::new();
        }

        match stream.phase {
            AgentMessageStreamPhase::Started => Vec::new(),
            AgentMessageStreamPhase::Delta => {
                self.delta_streams.insert(stream.stream_id.clone());
                self.stream_texts
                    .entry(stream.stream_id.clone())
                    .or_default()
                    .push_str(content);
                self.segmenter.push(content)
            }
            AgentMessageStreamPhase::Final => {
                self.completed_messages.insert(content.clone());
                if self.delta_streams.contains(&stream.stream_id) {
                    self.stream_texts
                        .entry(stream.stream_id.clone())
                        .or_insert_with(|| content.clone());
                    Vec::new()
                } else {
                    self.stream_texts
                        .insert(stream.stream_id.clone(), content.clone());
                    self.segmenter.push(content)
                }
            }
        }
    }

    fn finish(&mut self) -> Vec<String> {
        self.segmenter.finish()
    }

    fn message_was_seen(&self, content: &str) -> bool {
        self.completed_messages.contains(content)
            || self.stream_texts.values().any(|text| text == content)
    }
}

#[derive(Clone, Debug)]
struct StreamingSpeechSegmenter {
    buffer: String,
    buffer_chars: usize,
    spoken_chars: usize,
    maximum_chars: usize,
    emitted_chunks: usize,
    in_code_fence: bool,
    backtick_run: usize,
}

impl StreamingSpeechSegmenter {
    fn new(maximum_chars: usize) -> Self {
        Self {
            buffer: String::new(),
            buffer_chars: 0,
            spoken_chars: 0,
            maximum_chars,
            emitted_chunks: 0,
            in_code_fence: false,
            backtick_run: 0,
        }
    }

    fn push(&mut self, delta: &str) -> Vec<String> {
        let mut chunks = Vec::new();
        for character in delta.chars() {
            if character == '`' {
                self.backtick_run += 1;
                if self.backtick_run == 3 {
                    self.in_code_fence = !self.in_code_fence;
                    self.backtick_run = 0;
                }
                continue;
            }
            self.backtick_run = 0;
            if self.in_code_fence || self.spoken_chars >= self.maximum_chars {
                continue;
            }
            self.buffer.push(character);
            self.buffer_chars += 1;
            if self.should_emit(character) {
                chunks.extend(self.emit_buffer());
            }
        }
        chunks
    }

    fn finish(&mut self) -> Vec<String> {
        self.backtick_run = 0;
        self.emit_buffer()
    }

    fn should_emit(&self, character: char) -> bool {
        let maximum = self.current_chunk_limit();
        let sentence_boundary = matches!(character, '。' | '！' | '？' | '!' | '?');
        let semicolon_boundary = matches!(character, '；' | ';');
        let clause_boundary = matches!(character, '，' | ',');
        if self.emitted_chunks == 0 {
            self.buffer_chars >= maximum
                || (sentence_boundary && self.buffer_chars >= MINIMUM_SENTENCE_CHARS)
                || ((semicolon_boundary || clause_boundary)
                    && self.buffer_chars >= FIRST_CLAUSE_CHARS)
                || (character == '\n' && self.buffer_chars >= MINIMUM_SENTENCE_CHARS * 2)
        } else {
            self.buffer_chars >= maximum
                || (sentence_boundary && self.buffer_chars >= FOLLOWING_SENTENCE_CHARS)
                || (semicolon_boundary && self.buffer_chars >= FOLLOWING_SEMICOLON_CHARS)
                || (character == '\n' && self.buffer_chars >= FOLLOWING_SENTENCE_CHARS)
        }
    }

    fn current_chunk_limit(&self) -> usize {
        if self.emitted_chunks == 0 {
            FIRST_SPEECH_CHUNK_CHARS
        } else {
            FOLLOWING_SPEECH_CHUNK_CHARS
        }
    }

    fn emit_buffer(&mut self) -> Vec<String> {
        let raw = std::mem::take(&mut self.buffer);
        self.buffer_chars = 0;
        let raw = raw.trim();
        if raw.is_empty() || self.spoken_chars >= self.maximum_chars {
            return Vec::new();
        }
        let remaining = self.maximum_chars - self.spoken_chars;
        let Ok(rendered) = render_spoken_text(raw, remaining) else {
            return Vec::new();
        };
        let chunk_chars = rendered.text.chars().count();
        if chunk_chars == 0 || self.spoken_chars + chunk_chars > self.maximum_chars {
            return Vec::new();
        }
        self.spoken_chars += chunk_chars;
        self.emitted_chunks += 1;
        vec![rendered.text]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use yunxi_agent_core::{AgentMessageSequence, AgentMessageStream};

    fn stream_event(
        stream_id: &str,
        event_id: &str,
        phase: AgentMessageStreamPhase,
        content: &str,
    ) -> AgentEvent {
        AgentEvent::Message {
            content: content.to_string(),
            stream: Some(AgentMessageStream {
                thread_id: "thread".to_string(),
                turn_id: "turn".to_string(),
                stream_id: stream_id.to_string(),
                event_id: event_id.to_string(),
                source_sequence: AgentMessageSequence::ProviderReliable(1),
                phase,
            }),
        }
    }

    #[test]
    fn emits_first_sentence_without_waiting_for_final_response() {
        let mut collector = StreamingSpeechCollector::new(200);
        let first = collector.push_event(&stream_event(
            "message",
            "event-1",
            AgentMessageStreamPhase::Delta,
            "你好，我在。",
        ));
        let second = collector.push_event(&stream_event(
            "message",
            "event-2",
            AgentMessageStreamPhase::Delta,
            "接下来我们慢慢说。",
        ));

        assert_eq!(first, vec!["你好，我在。"]);
        assert!(second.is_empty());
        assert_eq!(collector.finish(), vec!["接下来我们慢慢说。"]);
    }

    #[test]
    fn final_message_and_legacy_fallback_do_not_repeat_streamed_text() {
        let mut collector = StreamingSpeechCollector::new(200);
        assert_eq!(
            collector.push_event(&stream_event(
                "message",
                "event-1",
                AgentMessageStreamPhase::Delta,
                "你好，我在。",
            )),
            vec!["你好，我在。"]
        );
        assert!(
            collector
                .push_event(&stream_event(
                    "message",
                    "event-2",
                    AgentMessageStreamPhase::Final,
                    "你好，我在。",
                ))
                .is_empty()
        );
        assert!(
            collector
                .push_event(&AgentEvent::Message {
                    content: "你好，我在。".to_string(),
                    stream: None,
                })
                .is_empty()
        );
    }

    #[test]
    fn code_fences_are_not_spoken_across_stream_deltas() {
        let mut segmenter = StreamingSpeechSegmenter::new(200);
        assert_eq!(segmenter.push("先说结论。```rust\n"), vec!["先说结论。"]);
        assert!(segmenter.push("println!(\"不要朗读\");\n").is_empty());
        assert!(segmenter.push("```后面继续。").is_empty());
        assert_eq!(segmenter.finish(), vec!["后面继续。"]);
    }

    #[test]
    fn first_unpunctuated_chunk_is_bounded_for_low_latency() {
        let mut segmenter = StreamingSpeechSegmenter::new(200);
        let source = "这是一段没有标点并且会持续输出很长时间的回复用于验证第一段不会无限等待直到整个模型回复全部生成完毕";
        let chunks = segmenter.push(source);

        assert!(!chunks.is_empty());
        assert!(chunks[0].chars().count() <= FIRST_SPEECH_CHUNK_CHARS);
    }

    #[test]
    fn following_short_sentences_are_grouped_for_continuity() {
        let mut segmenter = StreamingSpeechSegmenter::new(300);
        assert_eq!(segmenter.push("你好，我在。"), vec!["你好，我在。"]);
        assert!(segmenter.push("先别着急。").is_empty());

        let chunks = segmenter.push("我们慢慢把这件事理清楚，让前后语气保持在同一个呼吸里。");

        assert_eq!(
            chunks,
            vec!["先别着急。我们慢慢把这件事理清楚，让前后语气保持在同一个呼吸里。"]
        );
    }

    #[test]
    fn following_commas_do_not_fragment_speech() {
        let mut segmenter = StreamingSpeechSegmenter::new(300);
        assert_eq!(segmenter.push("你好，我在。"), vec!["你好，我在。"]);
        assert!(
            segmenter
                .push("接下来我会先听你说完，然后把重点整理清楚，再陪你一步一步往下走")
                .is_empty()
        );
        assert_eq!(
            segmenter.finish(),
            vec!["接下来我会先听你说完，然后把重点整理清楚，再陪你一步一步往下走"]
        );
    }
}
