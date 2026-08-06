"""Stable/quality voice routing with per-endpoint fallback and circuit breakers."""

from __future__ import annotations

import json
import os
import threading
import time
import urllib.error
import urllib.request
from dataclasses import dataclass
from queue import Empty, Queue
from typing import Any, Protocol


class VoiceBackend(Protocol):
    def health(self) -> dict[str, Any]: ...

    def transcribe(self, audio: bytes, language: str | None) -> dict[str, Any]: ...

    def synthesize(self, text: str, voice_id: str, emotion: str | None = None) -> bytes: ...


def env_float(name: str, default: float, minimum: float, maximum: float) -> float:
    try:
        value = float(os.environ.get(name, str(default)))
    except ValueError:
        return default
    return min(maximum, max(minimum, value))


def run_with_timeout(function: Any, timeout: float) -> Any:
    """Run a model call without allowing a stuck quality worker to block fallback."""
    result: Queue[tuple[bool, Any]] = Queue(maxsize=1)

    def invoke() -> None:
        try:
            result.put((True, function()))
        except Exception as error:  # propagate the original model error to the router
            result.put((False, error))

    thread = threading.Thread(target=invoke, name="yunxi-voice-call", daemon=True)
    thread.start()
    try:
        succeeded, value = result.get(timeout=timeout)
    except Empty as error:
        raise TimeoutError("quality voice backend timed out") from error
    if not succeeded:
        raise value
    return value


@dataclass
class CircuitBreaker:
    threshold: int = 3
    cooldown_seconds: float = 120.0
    failures: int = 0
    opened_at: float | None = None

    def allow(self, now: float | None = None) -> bool:
        now = time.monotonic() if now is None else now
        if self.opened_at is None:
            return True
        if now - self.opened_at >= self.cooldown_seconds:
            return True
        return False

    def success(self) -> None:
        self.failures = 0
        self.opened_at = None

    def failure(self, now: float | None = None) -> None:
        self.failures += 1
        if self.failures >= self.threshold:
            self.opened_at = time.monotonic() if now is None else now

    def health(self) -> dict[str, Any]:
        return {
            "failures": self.failures,
            "open": self.opened_at is not None and not self.allow(),
            "cooldown_seconds": self.cooldown_seconds,
        }


class QualityVoiceHttpClient:
    """HTTP client for the isolated quality worker process."""

    def __init__(self, base_url: str, auth_token: str = "") -> None:
        self.base_url = base_url.rstrip("/")
        self.auth_token = auth_token
        self.timeout = env_float("YUNXI_VOICE_QUALITY_TIMEOUT_SECONDS", 45.0, 1.0, 180.0)
        self.health_timeout = env_float(
            "YUNXI_VOICE_QUALITY_HEALTH_TIMEOUT_SECONDS", 2.0, 0.2, 15.0
        )
        self._health_cache: tuple[float, dict[str, Any]] | None = None
        self._health_lock = threading.Lock()

    def _request(
        self,
        path: str,
        body: bytes | None = None,
        content_type: str | None = None,
        timeout: float | None = None,
    ) -> tuple[str, bytes]:
        request = urllib.request.Request(f"{self.base_url}{path}", data=body, method="POST" if body else "GET")
        if content_type:
            request.add_header("Content-Type", content_type)
        if self.auth_token:
            request.add_header("Authorization", f"Bearer {self.auth_token}")
        try:
            with urllib.request.urlopen(request, timeout=timeout or self.timeout) as response:
                return response.headers.get("Content-Type", ""), response.read()
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise RuntimeError("quality voice sidecar unavailable") from error

    def health(self) -> dict[str, Any]:
        now = time.monotonic()
        with self._health_lock:
            if self._health_cache and now - self._health_cache[0] < 5.0:
                return self._health_cache[1]
        try:
            _content_type, body = self._request("/health", timeout=self.health_timeout)
            payload = json.loads(body)
            if not isinstance(payload, dict):
                raise ValueError("invalid health payload")
        except (RuntimeError, UnicodeDecodeError, json.JSONDecodeError, ValueError):
            payload = {
                "schema_version": 1,
                "status": "unavailable",
                "quality": {"ready": False},
                "error": "quality voice sidecar unavailable",
            }
        with self._health_lock:
            self._health_cache = (now, payload)
        return payload

    def transcribe(self, audio: bytes, language: str | None) -> dict[str, Any]:
        request = urllib.request.Request(
            f"{self.base_url}/v1/transcribe", data=audio, method="POST"
        )
        request.add_header("Content-Type", "audio/wav")
        request.add_header("X-Yunxi-Language", language or "auto")
        if self.auth_token:
            request.add_header("Authorization", f"Bearer {self.auth_token}")
        try:
            with urllib.request.urlopen(request, timeout=self.timeout) as response:
                payload = json.loads(response.read())
        except (urllib.error.URLError, TimeoutError, OSError) as error:
            raise RuntimeError("quality voice sidecar unavailable") from error
        if not isinstance(payload, dict) or not str(payload.get("text", "")).strip():
            raise RuntimeError("quality voice returned an empty transcription")
        return payload

    def synthesize(self, text: str, voice_id: str, emotion: str | None = None) -> bytes:
        payload = json.dumps(
            {"text": text, "voice": voice_id, "format": "wav", "emotion": emotion},
            ensure_ascii=False,
        ).encode("utf-8")
        _content_type, body = self._request(
            "/v1/synthesize", payload, "application/json", timeout=self.timeout
        )
        if len(body) < 44 or body[0:4] != b"RIFF" or body[8:12] != b"WAVE":
            raise RuntimeError("quality voice returned invalid WAV")
        return body


class VoiceBackendRouter:
    def __init__(
        self,
        stable: VoiceBackend,
        quality: VoiceBackend | None = None,
        mode: str | None = None,
    ) -> None:
        self.stable = stable
        self.quality = quality
        requested = (mode or os.environ.get("YUNXI_VOICE_MODE", "stable")).strip().lower()
        self.mode = requested if requested in {"stable", "quality", "auto"} else "stable"
        self.timeout = env_float("YUNXI_VOICE_QUALITY_TIMEOUT_SECONDS", 45.0, 1.0, 180.0)
        try:
            threshold = max(1, int(os.environ.get("YUNXI_VOICE_QUALITY_FAILURE_THRESHOLD", "3")))
        except ValueError:
            threshold = 3
        cooldown = env_float("YUNXI_VOICE_QUALITY_CIRCUIT_COOLDOWN_SECONDS", 120.0, 1.0, 3600.0)
        self._circuits = {
            "stt": CircuitBreaker(threshold, cooldown),
            "tts": CircuitBreaker(threshold, cooldown),
        }
        self._fallback_count = 0
        self._last_fallback: dict[str, str] | None = None
        self._lock = threading.Lock()

    def _quality_allowed(self, component: str) -> bool:
        if self.quality is None or self.mode == "stable":
            return False
        circuit = self._circuits[component]
        if not circuit.allow():
            return False
        try:
            health = self.quality.health()
            component_health = health.get(component, {})
            return bool(component_health.get("ready")) and health.get("status") in {"ok", "ready"}
        except Exception:
            return False

    def _record_failure(self, component: str, error: BaseException) -> None:
        self._circuits[component].failure()
        with self._lock:
            self._fallback_count += 1
            self._last_fallback = {"component": component, "reason": type(error).__name__}

    def _record_success(self, component: str) -> None:
        self._circuits[component].success()

    def transcribe(self, audio: bytes, language: str | None) -> dict[str, Any]:
        if self._quality_allowed("stt"):
            try:
                result = run_with_timeout(lambda: self.quality.transcribe(audio, language), self.timeout)  # type: ignore[union-attr]
                if not str(result.get("text", "")).strip():
                    raise RuntimeError("quality voice returned an empty transcription")
                self._record_success("stt")
                return result
            except Exception as error:
                self._record_failure("stt", error)
        return self.stable.transcribe(audio, language)

    def synthesize(self, text: str, voice_id: str, emotion: str | None = None) -> bytes:
        if self._quality_allowed("tts"):
            try:
                result = run_with_timeout(
                    lambda: self.quality.synthesize(text, voice_id, emotion),  # type: ignore[union-attr]
                    self.timeout,
                )
                if len(result) < 44 or result[0:4] != b"RIFF" or result[8:12] != b"WAVE":
                    raise RuntimeError("quality voice returned invalid WAV")
                self._record_success("tts")
                return result
            except Exception as error:
                self._record_failure("tts", error)
        return self.stable.synthesize(text, voice_id, emotion)

    @staticmethod
    def _safe_health(backend: VoiceBackend | None) -> dict[str, Any]:
        if backend is None:
            return {"status": "unavailable", "ready": False}
        try:
            value = backend.health()
            return value if isinstance(value, dict) else {"status": "unavailable", "ready": False}
        except Exception:
            return {"status": "unavailable", "ready": False}

    def health(self) -> dict[str, Any]:
        stable = self._safe_health(self.stable)
        quality = self._safe_health(self.quality)
        stt_quality = self._quality_allowed("stt")
        tts_quality = self._quality_allowed("tts")
        active_stt = quality.get("stt") if stt_quality else stable.get("stt")
        active_tts = quality.get("tts") if tts_quality else stable.get("tts")
        preset_voices = list(stable.get("preset_voices", []))
        for voice in quality.get("preset_voices", []):
            if voice not in preset_voices:
                preset_voices.append(voice)
        result = {
            "schema_version": 1,
            "status": "ok" if stable.get("status") in {"ok", "ready"} else "degraded",
            "mode": self.mode,
            "stt": active_stt or {"ready": False},
            "tts": active_tts or {"ready": False},
            "preset_voices": preset_voices,
            "backends": {"stable": stable, "quality": quality},
            "active": {
                "stt": "quality" if stt_quality else "stable",
                "tts": "quality" if tts_quality else "stable",
            },
            "fallback": {"count": self._fallback_count, "last": self._last_fallback},
            "circuit_breaker": {name: circuit.health() for name, circuit in self._circuits.items()},
            "capabilities": {
                "streaming": False,
                "voice_clone": bool(quality.get("capabilities", {}).get("voice_clone")),
                "emotion_control": bool(quality.get("capabilities", {}).get("emotion_control")),
            },
        }
        return result
