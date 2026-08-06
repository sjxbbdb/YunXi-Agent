#!/usr/bin/env python3
"""Isolated quality voice worker: faster-whisper large-v3 and IndexTTS2.

This process intentionally has a separate Python environment. The stable sidecar
can therefore stay warm and usable when a quality dependency or model fails.
"""

from __future__ import annotations

import argparse
import contextlib
import io
import logging
import os
import sys
import tempfile
import threading
from pathlib import Path
from typing import Any

from runtime_server import (
    VoiceHttpServer,
    env_enabled,
    configured_voice_language,
    normalize_yunxi_brand_transcript,
    resolve_voice_language,
    safe_int_env,
)
from voice_profile import VoiceProfile, VoiceProfileError


def _path_env(name: str, default: str) -> Path:
    return Path(os.environ.get(name, default).strip() or default).expanduser().resolve()


class QualityVoiceModels:
    def __init__(self) -> None:
        self.device = os.environ.get("YUNXI_VOICE_QUALITY_DEVICE", "cuda:0").strip() or "cuda:0"
        self.default_language = configured_voice_language()
        self.stt_model_dir = _path_env(
            "YUNXI_VOICE_QUALITY_STT_MODEL_DIR", "D:\\YunXi Voice Runtime\\models\\faster-whisper-large-v3"
        )
        self.index_source = _path_env(
            "YUNXI_INDEXTTS_SOURCE", "D:\\YunXi Voice Runtime\\sources\\index-tts"
        )
        self.index_model_dir = _path_env(
            "YUNXI_INDEXTTS_MODEL_DIR", "D:\\YunXi Voice Runtime\\models\\IndexTTS-2"
        )
        self.index_config = _path_env(
            "YUNXI_INDEXTTS_CONFIG", str(self.index_model_dir / "config.yaml")
        )
        profile_path = os.environ.get("YUNXI_VOICE_PROFILE", "").strip()
        self.profile: VoiceProfile | None = None
        self.profile_error: str | None = None
        if profile_path:
            try:
                self.profile = VoiceProfile.load(profile_path)
            except VoiceProfileError:
                self.profile_error = "voice profile is unavailable"
        self._stt: Any = None
        self._tts: Any = None
        self._stt_error: str | None = None
        self._tts_error: str | None = None
        self._stt_lock = threading.Lock()
        self._tts_lock = threading.Lock()
        self._model_io_lock = threading.Lock()

    def _stt_configured(self) -> bool:
        try:
            import importlib.util

            return (
                self._stt_error is None
                and self.stt_model_dir.is_dir()
                and importlib.util.find_spec("faster_whisper") is not None
            )
        except (ImportError, ValueError):
            return False

    def _tts_configured(self) -> bool:
        try:
            import importlib.util

            return (
                self._tts_error is None
                and self.profile is not None
                and self.index_source.is_dir()
                and self.index_config.is_file()
                and self.index_model_dir.is_dir()
                and importlib.util.find_spec("torch") is not None
            )
        except (ImportError, ValueError):
            return False

    def _load_stt(self) -> Any:
        if self._stt is not None:
            return self._stt
        with self._stt_lock:
            if self._stt is not None:
                return self._stt
            if not self._stt_configured():
                raise RuntimeError("faster-whisper model or package is unavailable")
            try:
                from faster_whisper import WhisperModel

                compute_type = os.environ.get(
                    "YUNXI_VOICE_WHISPER_COMPUTE_TYPE", "int8_float16"
                ).strip()
                whisper_device = "cuda" if self.device.startswith("cuda") else self.device
                device_index = 0
                if self.device.startswith("cuda:"):
                    device_index = int(self.device.split(":", 1)[1])
                logging.info("loading quality STT model=%s", self.stt_model_dir)
                self._stt = WhisperModel(
                    str(self.stt_model_dir),
                    device=whisper_device,
                    device_index=device_index,
                    compute_type=compute_type,
                )
                return self._stt
            except Exception:
                self._stt_error = "faster-whisper failed to load"
                raise

    def _load_tts(self) -> Any:
        if self._tts is not None:
            return self._tts
        with self._tts_lock:
            if self._tts is not None:
                return self._tts
            if not self._tts_configured() or self.profile is None:
                raise RuntimeError("IndexTTS2 or voice profile is unavailable")
            try:
                sys.path.insert(0, str(self.index_source))
                from indextts.infer_v2 import IndexTTS2

                logging.info("loading quality TTS model=%s", self.index_model_dir)
                self._tts = IndexTTS2(
                    cfg_path=str(self.index_config),
                    model_dir=str(self.index_model_dir),
                    use_fp16=self.device.startswith("cuda"),
                    device=self.device,
                    use_cuda_kernel=False,
                    use_deepspeed=False,
                    use_accel=False,
                    use_torch_compile=False,
                )
                return self._tts
            except Exception:
                self._tts_error = "IndexTTS2 failed to load"
                raise

    def health(self) -> dict[str, Any]:
        stt_ready = self._stt_configured()
        tts_ready = self._tts_configured()
        warmup_complete = (not stt_ready or self._stt is not None) and (
            not tts_ready or self._tts is not None
        )
        return {
            "schema_version": 1,
            "status": "ok" if stt_ready or tts_ready else "unavailable",
            "stt": {
                "provider": "faster-whisper",
                "model": self.stt_model_dir.name,
                "device": self.device,
                "ready": stt_ready,
                "loaded": self._stt is not None,
                "language": self.default_language,
            },
            "tts": {
                "provider": "IndexTTS2",
                "model": self.index_model_dir.name,
                "device": self.device,
                "ready": tts_ready,
                "loaded": self._tts is not None,
                "profile": self.profile.public_health() if self.profile else {"ready": False},
            },
            "preset_voices": [
                "中文女",
                *([self.profile.profile_id] if self.profile else []),
            ],
            "capabilities": {
                "streaming": False,
                "voice_clone": tts_ready,
                "emotion_control": bool(self.profile and self.profile.emotion_references),
            },
            "errors": {
                "stt": self._stt_error,
                "tts": self._tts_error or self.profile_error,
            },
            "warmup": {
                "enabled": env_enabled("YUNXI_VOICE_QUALITY_WARMUP"),
                "complete": warmup_complete,
                "stt_loaded": self._stt is not None,
                "tts_loaded": self._tts is not None,
            },
        }

    def warmup(self) -> None:
        """Load configured models before the first user turn pays the cold-start cost."""
        if self._stt_configured():
            try:
                self._load_stt()
            except Exception:
                logging.warning("quality STT warmup failed; stable fallback remains available")
        if self._tts_configured():
            try:
                self._load_tts()
            except Exception:
                logging.warning("quality TTS warmup failed; stable fallback remains available")

    def transcribe(self, audio: bytes, language: str | None) -> dict[str, Any]:
        model = self._load_stt()
        temporary_path: str | None = None
        try:
            with tempfile.NamedTemporaryFile(prefix="yunxi-quality-", suffix=".wav", delete=False) as temporary:
                temporary.write(audio)
                temporary_path = temporary.name
            with self._model_io_lock, self._stt_lock:
                effective_language = resolve_voice_language(language, self.default_language)
                segments, info = model.transcribe(
                    temporary_path,
                    language=None if effective_language == "auto" else effective_language,
                    beam_size=5,
                    vad_filter=True,
                    condition_on_previous_text=False,
                    initial_prompt="这是中文对话。助手名为云熙。" if effective_language == "zh" else None,
                )
                text = normalize_yunxi_brand_transcript(
                    "".join(segment.text for segment in segments).strip()
                )
            if not text:
                raise RuntimeError("faster-whisper returned an empty transcription")
            return {
                "text": text,
                "language": getattr(info, "language", None) or effective_language,
                "emotion": None,
                "audio_events": [],
            }
        finally:
            if temporary_path:
                try:
                    os.remove(temporary_path)
                except FileNotFoundError:
                    pass

    def synthesize(self, text: str, voice_id: str, emotion: str | None = None) -> bytes:
        if self.profile is None:
            raise RuntimeError("voice profile is unavailable")
        if voice_id not in {"中文女", self.profile.profile_id, self.profile.fallback_voice}:
            raise ValueError("unknown quality voice profile")
        tts = self._load_tts()
        output_path: str | None = None
        try:
            with tempfile.NamedTemporaryFile(prefix="yunxi-quality-", suffix=".wav", delete=False) as output:
                output_path = output.name
            emotion_audio = self.profile.emotion_audio(emotion)
            with self._model_io_lock, self._tts_lock:
                with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(
                    io.StringIO()
                ):
                    tts.infer(
                        spk_audio_prompt=str(self.profile.reference_audio),
                        text=text,
                        output_path=output_path,
                        emo_audio_prompt=str(emotion_audio) if emotion_audio else None,
                        verbose=False,
                    )
            data = Path(output_path).read_bytes()
            if len(data) < 44 or data[0:4] != b"RIFF" or data[8:12] != b"WAVE":
                raise RuntimeError("IndexTTS2 returned invalid WAV")
            return data
        finally:
            if output_path:
                try:
                    os.remove(output_path)
                except FileNotFoundError:
                    pass


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="YunXi isolated quality voice runtime")
    parser.add_argument("--bind", default=os.environ.get("YUNXI_VOICE_QUALITY_BIND", "127.0.0.1"))
    parser.add_argument(
        "--port",
        type=int,
        default=safe_int_env("YUNXI_VOICE_QUALITY_PORT", 17864, 1, 65_535),
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.bind not in {"127.0.0.1", "localhost", "::1"}:
        raise SystemExit("quality voice runtime refuses non-loopback bind addresses")
    logging.basicConfig(level=logging.INFO, format="%(asctime)s %(levelname)s %(message)s")
    models = QualityVoiceModels()
    if env_enabled("YUNXI_VOICE_QUALITY_WARMUP"):
        models.warmup()
    server = VoiceHttpServer((args.bind, args.port), models)
    logging.info("YunXi quality voice runtime listening on http://%s:%s", args.bind, args.port)
    try:
        server.serve_forever(poll_interval=0.25)
    except KeyboardInterrupt:
        logging.info("quality voice runtime interrupted")
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
