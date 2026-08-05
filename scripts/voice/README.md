# YunXi Voice Runtime

> Independent private deployment repository: [sjxbbdb/YunXi-Voice-Runtime](https://github.com/sjxbbdb/YunXi-Voice-Runtime). This directory remains the monorepo integration snapshot; use the private repository README for standalone installation.

This directory contains the optional local STT/TTS sidecar used by the voice MVP.

The Rust CLI never imports Python or model code. It calls a loopback-only HTTP
contract:

- GET /health
- POST /v1/transcribe with a raw audio/wav body
- POST /v1/synthesize with JSON and a raw WAV response

## Mock smoke

The mock runtime uses only the Python standard library:

~~~powershell
.\scripts\voice\start-voice-runtime.ps1 -Mock
cargo run -q -p yunxi-agent-cli --bin yunxi -- voice doctor
~~~

Set YUNXI_VOICE_MOCK_TRANSCRIPT to choose the mock transcription.

Run the complete isolated process-level smoke:

~~~powershell
python .\scripts\voice\smoke_mock.py
~~~

## Real models

The MVP model pairing is:

- STT: SenseVoiceSmall
- TTS: CosyVoice-300M-SFT
- Preset voice: 中文女

The real runtime expects these environment variables:

~~~text
YUNXI_VOICE_DEVICE=cuda:0
YUNXI_VOICE_STT_MODEL_DIR=<SenseVoice model directory or model id>
YUNXI_VOICE_TTS_MODEL_DIR=<CosyVoice SFT model directory or model id>
YUNXI_COSYVOICE_REPO=<CosyVoice source checkout>
~~~

Model installation is intentionally separate from the Rust workspace so Python
packages, model weights, and caches do not enter target/ or release binaries.

On the validated Windows setup, install the optional runtime with:

~~~powershell
.\scripts\voice\install-voice-runtime.ps1 -RuntimeRoot "D:\YunXi Voice Runtime"
~~~

The installer creates an isolated Python 3.10 environment, installs
`torch 2.8.0+cu129` and `torchaudio 2.8.0+cu129`, downloads both models, and
keeps all caches under the runtime root. The CUDA 12.9 wheels are required for
the validated RTX 5060 Ti; do not replace them with CosyVoice's old Torch 2.3.1
pin on RTX 50-series hardware.

Start and verify the real runtime:

~~~powershell
.\scripts\voice\start-voice-runtime.ps1 -RuntimeRoot "D:\YunXi Voice Runtime"
yunxi voice doctor
yunxi voice devices
yunxi voice talk --companion
yunxi voice transcribe --input .\question.wav
yunxi voice speak "你好，我是 YunXi。" --output .\reply.wav
yunxi voice chat --input .\question.wav --output .\reply.wav --companion
~~~

From an already-running interactive CLI or TUI, use the integrated mode:

~~~text
yunxi> /voice status
yunxi> /voice devices
yunxi> /voice
yunxi> /voice realtime on
yunxi> /voice realtime status
yunxi> /voice realtime off
~~~

`/voice` keeps text and speech inside the same `InteractiveSession`. Press Enter
to start recording, press Enter again to stop, and enter `q` to return to the
normal composer. Transcribed speech uses the existing streaming runtime, tool
approval UI, active session id, persona, memory, and companion configuration.

`/voice realtime on` explicitly enters hands-free half-duplex mode. Local VAD
detects speech and trailing silence without writing microphone audio to disk.
Replies are split at natural sentence boundaries: the first synthesized chunk
starts playing while the next chunk is prefetched. After playback, listening resumes. Say `关闭实时语音` to return
to the text composer. In the TUI, entering `q` or `/voice realtime off` and
pressing Enter, or pressing Ctrl+C, also stops capture immediately. The switch
defaults to off, is not persisted between CLI sessions, and is visible as
`voice=live` in the TUI while active.

`voice talk` provides Windows push-to-talk capture and playback. Press Enter to
start recording, press Enter again to stop, and enter `q` to exit. Captured
audio is encoded as an in-memory WAV and is never written to disk. Each turn is
linked to the previous voice session so follow-up questions retain conversation
context. Use `--input-device <NAME>` to select a microphone shown by
`voice devices`, or `--no-playback` when text-only replies are preferred.

This is not a full-duplex implementation. Keyboard control can cancel current
playback, but realtime mode does not implement spoken barge-in, streaming STT,
server-streamed TTS, voice cloning, JSONL, or spoken tool approvals. The
single-file transcribe, speak, and chat commands continue to use WAV files.

## Security

- The server refuses non-loopback bind addresses.
- WAV input has a bounded request size and is deleted after transcription.
- Request bodies and transcriptions are not logged.
- Set the same YUNXI_VOICE_AUTH_TOKEN in the server and CLI environment when
  local bearer authentication is required.
- Remote voice URLs are rejected by the Rust client unless
  YUNXI_VOICE_ALLOW_REMOTE=1 is explicit.
