#!/usr/bin/env python3
"""Local-only STT/TTS sidecar for the YunXi voice MVP."""

from __future__ import annotations

import argparse
import contextlib
import hmac
import io
import json
import logging
import math
import os
import re
import struct
import sys
import tempfile
import threading
import wave
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from typing import Any

from voice_router import QualityVoiceHttpClient, VoiceBackendRouter


SCHEMA_VERSION = 1
DEFAULT_BIND = "127.0.0.1"
DEFAULT_PORT = 17862
DEFAULT_PRESET = "中文女"
DEFAULT_MAX_AUDIO_BYTES = 25 * 1024 * 1024
DEFAULT_MAX_JSON_BYTES = 512 * 1024
DEFAULT_MAX_TEXT_CHARS = 2_000
DEFAULT_VOICE_LANGUAGE = "zh"
YUNXI_BRAND_ALIASES = (
    "云希",
    "云溪",
    "云系",
    "云戏",
    "云汐",
    "云熙",
    "云惜",
    "云西",
    "云曦",
    "云夕",
)
YUNXI_BRAND_SUFFIXES = ("智能体", "助手")
YUNXI_SELF_REFERENCE_PREFIXES = ("我是", "我叫", "叫我", "这里是")


class VoiceRequestError(Exception):
    def __init__(self, status: HTTPStatus, message: str) -> None:
        super().__init__(message)
        self.status = status


class VoiceModelPrivacyFilter(logging.Filter):
    def filter(self, record: logging.LogRecord) -> bool:
        message = record.getMessage().lower()
        return not message.startswith("synthesis text ")


def env_enabled(name: str) -> bool:
    return os.environ.get(name, "").strip().lower() in {"1", "true", "yes", "on"}


def safe_int_env(name: str, default: int, minimum: int, maximum: int) -> int:
    try:
        value = int(os.environ.get(name, str(default)))
    except ValueError:
        return default
    return min(maximum, max(minimum, value))


def configured_voice_language() -> str:
    """Use Chinese for short clips unless multilingual detection is explicit."""
    value = os.environ.get("YUNXI_VOICE_DEFAULT_LANGUAGE", DEFAULT_VOICE_LANGUAGE).strip().lower()
    if not value:
        return DEFAULT_VOICE_LANGUAGE
    if value == "auto" or re.fullmatch(r"[a-z]{2,3}(?:[-_][a-z0-9]{2,8})?", value):
        return value
    return DEFAULT_VOICE_LANGUAGE


def resolve_voice_language(language: str | None, default: str | None = None) -> str:
    value = (language or "").strip().lower()
    if value in {"", "auto"}:
        return (default or configured_voice_language()).strip().lower()
    return value


def normalize_yunxi_brand_transcript(text: str) -> str:
    normalized = re.sub(r"\byunxi\s+agent\b", "云熙", text, flags=re.IGNORECASE)
    normalized = re.sub(r"\byunxi\b", "云熙", normalized, flags=re.IGNORECASE)
    for alias in YUNXI_BRAND_ALIASES:
        for suffix in YUNXI_BRAND_SUFFIXES:
            normalized = normalized.replace(f"{alias}{suffix}", "云熙")
            normalized = normalized.replace(f"{alias} {suffix}", "云熙")
        for prefix in YUNXI_SELF_REFERENCE_PREFIXES:
            normalized = normalized.replace(f"{prefix}{alias}", f"{prefix}云熙")
            normalized = normalized.replace(f"{prefix} {alias}", f"{prefix}云熙")
    return normalized


def mock_wav(text: str) -> bytes:
    sample_rate = 16_000
    duration_seconds = min(1.5, max(0.35, len(text) / 60.0))
    frame_count = int(sample_rate * duration_seconds)
    amplitude = 3_000
    buffer = io.BytesIO()
    with wave.open(buffer, "wb") as output:
        output.setnchannels(1)
        output.setsampwidth(2)
        output.setframerate(sample_rate)
        frames = bytearray()
        for index in range(frame_count):
            sample = int(amplitude * math.sin(2 * math.pi * 440 * index / sample_rate))
            frames.extend(struct.pack("<h", sample))
        output.writeframes(bytes(frames))
    return buffer.getvalue()


class MockVoiceModels:
    def __init__(self) -> None:
        self.device = "cpu"
        self.preset_voices = [DEFAULT_PRESET]

    def health(self) -> dict[str, Any]:
        return {
            "schema_version": SCHEMA_VERSION,
            "status": "ok",
            "stt": {
                "provider": "mock",
                "model": "mock-sensevoice",
                "device": self.device,
                "ready": True,
            },
            "tts": {
                "provider": "mock",
                "model": "mock-cosyvoice-sft",
                "device": self.device,
                "ready": True,
            },
            "preset_voices": self.preset_voices,
        }

    def transcribe(self, _audio: bytes, language: str | None) -> dict[str, Any]:
        text = os.environ.get("YUNXI_VOICE_MOCK_TRANSCRIPT", "你好，云希。").strip()
        if not text:
            raise VoiceRequestError(HTTPStatus.INTERNAL_SERVER_ERROR, "mock transcript is empty")
        return {
            "text": normalize_yunxi_brand_transcript(text),
            "language": resolve_voice_language(language),
            "emotion": None,
            "audio_events": [],
        }

    def synthesize(self, text: str, voice_id: str, _emotion: str | None = None) -> bytes:
        if voice_id not in self.preset_voices:
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "unknown preset voice")
        return mock_wav(text)


class LocalVoiceModels:
    def __init__(self) -> None:
        self.device = os.environ.get("YUNXI_VOICE_DEVICE", "cuda:0").strip() or "cuda:0"
        self.default_language = configured_voice_language()
        self.stt_model_dir = os.environ.get(
            "YUNXI_VOICE_STT_MODEL_DIR", "iic/SenseVoiceSmall"
        ).strip()
        self.tts_model_dir = os.environ.get(
            "YUNXI_VOICE_TTS_MODEL_DIR", "iic/CosyVoice-300M-SFT"
        ).strip()
        cosyvoice_repo = os.environ.get("YUNXI_COSYVOICE_REPO", "").strip()
        if cosyvoice_repo:
            cosyvoice_root = Path(cosyvoice_repo).resolve()
            matcha_root = cosyvoice_root / "third_party" / "Matcha-TTS"
            if matcha_root.is_dir():
                sys.path.insert(0, str(matcha_root))
            sys.path.insert(0, str(cosyvoice_root))

        import torch
        from funasr import AutoModel
        from funasr.utils.postprocess_utils import rich_transcription_postprocess
        from cosyvoice.cli.cosyvoice import CosyVoice

        if self.device.startswith("cuda") and not torch.cuda.is_available():
            raise RuntimeError("CUDA was requested but PyTorch cannot access the NVIDIA GPU")
        self._torch = torch
        self._postprocess = rich_transcription_postprocess
        self._stt_lock = threading.Lock()
        self._tts_lock = threading.Lock()
        self._model_io_lock = threading.Lock()
        logging.getLogger().addFilter(VoiceModelPrivacyFilter())
        logging.info("loading SenseVoice model=%s device=%s", self.stt_model_dir, self.device)
        self._stt = AutoModel(
            model=self.stt_model_dir,
            trust_remote_code=True,
            device=self.device,
            disable_update=True,
        )
        logging.info("loading CosyVoice model=%s", self.tts_model_dir)
        self._tts = CosyVoice(
            self.tts_model_dir,
            load_jit=False,
            load_trt=False,
            fp16=self.device.startswith("cuda"),
        )
        self.preset_voices = list(self._tts.list_available_spks())
        if not self.preset_voices:
            raise RuntimeError("CosyVoice SFT model did not expose any preset voices")

    def health(self) -> dict[str, Any]:
        return {
            "schema_version": SCHEMA_VERSION,
            "status": "ok",
            "stt": {
                "provider": "local",
                "model": Path(self.stt_model_dir).name or self.stt_model_dir,
                "device": self.device,
                "ready": True,
            },
            "tts": {
                "provider": "local",
                "model": Path(self.tts_model_dir).name or self.tts_model_dir,
                "device": self.device,
                "ready": True,
            },
            "preset_voices": self.preset_voices,
        }

    def transcribe(self, audio: bytes, language: str | None) -> dict[str, Any]:
        temporary_path: str | None = None
        try:
            with tempfile.NamedTemporaryFile(
                prefix="yunxi-voice-", suffix=".wav", delete=False
            ) as temporary:
                temporary.write(audio)
                temporary_path = temporary.name
            with self._model_io_lock, self._stt_lock:
                result = self._stt.generate(
                    input=temporary_path,
                    cache={},
                    language=resolve_voice_language(language, self.default_language),
                    use_itn=True,
                    batch_size_s=60,
                )
            if not result or not isinstance(result[0], dict):
                raise RuntimeError("SenseVoice returned no transcription result")
            raw_text = str(result[0].get("text", "")).strip()
            text = normalize_yunxi_brand_transcript(self._postprocess(raw_text).strip())
            if not text:
                raise RuntimeError("SenseVoice returned an empty transcription")
            detected_language = result[0].get("language")
            return {
                "text": text,
                "language": str(detected_language) if detected_language else resolve_voice_language(language, self.default_language),
                "emotion": None,
                "audio_events": [],
            }
        finally:
            if temporary_path:
                try:
                    os.remove(temporary_path)
                except FileNotFoundError:
                    pass

    def synthesize(self, text: str, voice_id: str, _emotion: str | None = None) -> bytes:
        if voice_id not in self.preset_voices:
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "unknown preset voice")
        chunks = []
        with self._model_io_lock, self._tts_lock:
            # CosyVoice prints the input sentence and progress directly. Keep
            # those model-internal diagnostics out of local request logs.
            with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
                for result in self._tts.inference_sft(
                    text, voice_id, stream=False, speed=1.0
                ):
                    speech = result.get("tts_speech")
                    if speech is not None:
                        chunks.append(speech.detach().cpu())
        if not chunks:
            raise RuntimeError("CosyVoice returned no audio")
        speech = self._torch.cat(chunks, dim=1).squeeze(0).numpy()
        import soundfile

        buffer = io.BytesIO()
        soundfile.write(
            buffer,
            speech,
            self._tts.sample_rate,
            format="WAV",
            subtype="PCM_16",
        )
        return buffer.getvalue()


class VoiceRequestHandler(BaseHTTPRequestHandler):
    server_version = "YunXiVoice/1"
    protocol_version = "HTTP/1.0"

    @property
    def voice_server(self) -> "VoiceHttpServer":
        return self.server  # type: ignore[return-value]

    def log_message(self, format_string: str, *args: object) -> None:
        logging.info("%s - %s", self.client_address[0], format_string % args)

    def do_GET(self) -> None:
        try:
            self._authorize()
            if self.path != "/health":
                raise VoiceRequestError(HTTPStatus.NOT_FOUND, "endpoint not found")
            self._json(HTTPStatus.OK, self.voice_server.models.health())
        except VoiceRequestError as error:
            self._json(error.status, {"error": str(error)})
        except Exception:
            logging.exception("voice health request failed")
            self._json(HTTPStatus.INTERNAL_SERVER_ERROR, {"error": "voice runtime failed"})

    def do_POST(self) -> None:
        try:
            self._authorize()
            if self.path == "/v1/transcribe":
                self._transcribe()
            elif self.path == "/v1/synthesize":
                self._synthesize()
            else:
                raise VoiceRequestError(HTTPStatus.NOT_FOUND, "endpoint not found")
        except VoiceRequestError as error:
            self._json(error.status, {"error": str(error)})
        except Exception:
            logging.exception("voice request failed path=%s", self.path)
            self._json(HTTPStatus.INTERNAL_SERVER_ERROR, {"error": "voice runtime failed"})

    def _authorize(self) -> None:
        expected = self.voice_server.auth_token
        if not expected:
            return
        supplied = self.headers.get("Authorization", "")
        expected_header = f"Bearer {expected}"
        if not hmac.compare_digest(supplied, expected_header):
            raise VoiceRequestError(HTTPStatus.UNAUTHORIZED, "voice runtime authorization failed")

    def _content_length(self, maximum: int) -> int:
        raw = self.headers.get("Content-Length", "")
        try:
            length = int(raw)
        except ValueError as error:
            raise VoiceRequestError(
                HTTPStatus.LENGTH_REQUIRED, "valid Content-Length is required"
            ) from error
        if length <= 0:
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "request body is empty")
        if length > maximum:
            raise VoiceRequestError(
                HTTPStatus.REQUEST_ENTITY_TOO_LARGE, "request body exceeds the configured limit"
            )
        return length

    def _transcribe(self) -> None:
        content_type = self.headers.get("Content-Type", "").split(";", 1)[0].strip()
        if content_type not in {"audio/wav", "audio/x-wav"}:
            raise VoiceRequestError(
                HTTPStatus.UNSUPPORTED_MEDIA_TYPE, "the MVP accepts WAV audio only"
            )
        length = self._content_length(self.voice_server.max_audio_bytes)
        audio = self.rfile.read(length)
        if len(audio) < 44 or audio[0:4] != b"RIFF" or audio[8:12] != b"WAVE":
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "audio is not RIFF/WAVE")
        language = self.headers.get("X-Yunxi-Language")
        result = self.voice_server.models.transcribe(audio, language)
        self._json(HTTPStatus.OK, result)

    def _synthesize(self) -> None:
        length = self._content_length(self.voice_server.max_json_bytes)
        try:
            payload = json.loads(self.rfile.read(length))
        except (UnicodeDecodeError, json.JSONDecodeError) as error:
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "invalid JSON request") from error
        text = str(payload.get("text", "")).strip()
        voice_id = str(payload.get("voice", "")).strip()
        output_format = str(payload.get("format", "")).strip().lower()
        if not text:
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "speech text is empty")
        if len(text) > self.voice_server.max_text_chars:
            raise VoiceRequestError(
                HTTPStatus.REQUEST_ENTITY_TOO_LARGE, "speech text exceeds the configured limit"
            )
        if not voice_id:
            raise VoiceRequestError(HTTPStatus.BAD_REQUEST, "preset voice is required")
        if output_format != "wav":
            raise VoiceRequestError(
                HTTPStatus.BAD_REQUEST, "the MVP supports WAV output only"
            )
        emotion = str(payload.get("emotion", "")).strip() or None
        audio = self.voice_server.models.synthesize(text, voice_id, emotion)
        self._bytes(HTTPStatus.OK, "audio/wav", audio)

    def _json(self, status: HTTPStatus, payload: dict[str, Any]) -> None:
        body = json.dumps(payload, ensure_ascii=False).encode("utf-8")
        self._bytes(status, "application/json; charset=utf-8", body)

    def _bytes(self, status: HTTPStatus, content_type: str, body: bytes) -> None:
        self.send_response(status.value)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(body)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(body)


class VoiceHttpServer(ThreadingHTTPServer):
    daemon_threads = True

    def __init__(
        self,
        address: tuple[str, int],
        models: Any,
    ) -> None:
        super().__init__(address, VoiceRequestHandler)
        self.models = models
        self.auth_token = os.environ.get("YUNXI_VOICE_AUTH_TOKEN", "").strip()
        self.max_audio_bytes = safe_int_env(
            "YUNXI_VOICE_MAX_INPUT_BYTES",
            DEFAULT_MAX_AUDIO_BYTES,
            44,
            100 * 1024 * 1024,
        )
        self.max_json_bytes = safe_int_env(
            "YUNXI_VOICE_MAX_JSON_BYTES",
            DEFAULT_MAX_JSON_BYTES,
            1_024,
            4 * 1024 * 1024,
        )
        self.max_text_chars = safe_int_env(
            "YUNXI_VOICE_MAX_SPEECH_CHARS",
            DEFAULT_MAX_TEXT_CHARS,
            1,
            20_000,
        )


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="YunXi local voice runtime")
    parser.add_argument("--bind", default=os.environ.get("YUNXI_VOICE_BIND", DEFAULT_BIND))
    parser.add_argument(
        "--port",
        type=int,
        default=safe_int_env("YUNXI_VOICE_PORT", DEFAULT_PORT, 1, 65_535),
    )
    parser.add_argument("--mock", action="store_true", default=env_enabled("YUNXI_VOICE_MOCK"))
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    if args.bind not in {"127.0.0.1", "localhost", "::1"}:
        raise SystemExit("voice runtime refuses non-loopback bind addresses")
    logging.basicConfig(
        level=logging.INFO,
        format="%(asctime)s %(levelname)s %(message)s",
    )
    models: Any
    if args.mock:
        models = MockVoiceModels()
        logging.info("starting mock voice runtime")
    else:
        stable = LocalVoiceModels()
        quality: QualityVoiceHttpClient | None = None
        mode = os.environ.get("YUNXI_VOICE_MODE", "stable").strip().lower()
        if mode in {"quality", "auto"}:
            quality = QualityVoiceHttpClient(
                os.environ.get("YUNXI_VOICE_QUALITY_URL", "http://127.0.0.1:17864"),
                os.environ.get("YUNXI_VOICE_AUTH_TOKEN", "").strip(),
            )
        models = VoiceBackendRouter(stable, quality, mode)
    server = VoiceHttpServer((args.bind, args.port), models)
    logging.info("YunXi voice runtime listening on http://%s:%s", args.bind, args.port)
    try:
        server.serve_forever(poll_interval=0.25)
    except KeyboardInterrupt:
        logging.info("voice runtime interrupted")
    finally:
        server.server_close()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
