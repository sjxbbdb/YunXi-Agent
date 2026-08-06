"""Local-only voice profile validation for the optional quality TTS backend."""

from __future__ import annotations

import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any


class VoiceProfileError(ValueError):
    """Raised when a local voice profile is missing or unsafe to use."""


@dataclass(frozen=True)
class VoiceProfile:
    schema_version: int
    profile_id: str
    backend: str
    reference_audio: Path
    reference_transcript: str
    emotion_references: dict[str, Path] = field(default_factory=dict)
    speed: float = 1.0
    fallback_voice: str = "中文女"

    @classmethod
    def load(cls, path: str | Path) -> "VoiceProfile":
        profile_path = Path(path).expanduser().resolve()
        if not profile_path.is_file():
            raise VoiceProfileError("voice profile does not exist")
        try:
            payload = json.loads(profile_path.read_text(encoding="utf-8"))
        except (OSError, UnicodeDecodeError, json.JSONDecodeError) as error:
            raise VoiceProfileError("voice profile is not valid JSON") from error
        if not isinstance(payload, dict):
            raise VoiceProfileError("voice profile must be a JSON object")

        schema_version = payload.get("schema_version")
        profile_id = str(payload.get("id", "")).strip()
        backend = str(payload.get("backend", "")).strip().lower()
        transcript = str(payload.get("reference_transcript", "")).strip()
        fallback_voice = str(payload.get("fallback_voice", "中文女")).strip() or "中文女"
        if schema_version != 1 or not profile_id or backend != "indextts2":
            raise VoiceProfileError("unsupported voice profile schema or backend")
        if not transcript:
            raise VoiceProfileError("reference_transcript is required")

        try:
            speed = float(payload.get("speed", 1.0))
        except (TypeError, ValueError) as error:
            raise VoiceProfileError("voice profile speed is invalid") from error
        # IndexTTS2 currently has no stable speed parameter in its public API.
        if not 0.5 <= speed <= 2.0:
            raise VoiceProfileError("voice profile speed must be between 0.5 and 2.0")

        reference_audio = cls._resolve_audio(
            profile_path.parent, payload.get("reference_audio"), "reference_audio"
        )
        raw_emotions = payload.get("emotion_references", {})
        if raw_emotions is None:
            raw_emotions = {}
        if not isinstance(raw_emotions, dict):
            raise VoiceProfileError("emotion_references must be an object")
        emotion_references = {
            str(name).strip(): cls._resolve_audio(profile_path.parent, value, "emotion reference")
            for name, value in raw_emotions.items()
            if str(name).strip()
        }
        return cls(
            schema_version=1,
            profile_id=profile_id,
            backend=backend,
            reference_audio=reference_audio,
            reference_transcript=transcript,
            emotion_references=emotion_references,
            speed=speed,
            fallback_voice=fallback_voice,
        )

    @staticmethod
    def _resolve_audio(root: Path, value: Any, label: str) -> Path:
        raw = str(value or "").strip()
        if not raw:
            raise VoiceProfileError(f"{label} is required")
        path = Path(raw).expanduser()
        if not path.is_absolute():
            path = root / path
        path = path.resolve()
        if not path.is_file():
            raise VoiceProfileError(f"{label} does not exist")
        if path.suffix.lower() not in {".wav", ".flac", ".mp3", ".m4a", ".ogg"}:
            raise VoiceProfileError(f"{label} must be an audio file")
        return path

    def emotion_audio(self, emotion: str | None) -> Path | None:
        if not emotion:
            return None
        return self.emotion_references.get(emotion.strip().lower())

    def public_health(self) -> dict[str, Any]:
        return {
            "id": self.profile_id,
            "backend": self.backend,
            "ready": self.reference_audio.is_file(),
            "emotion_controls": sorted(self.emotion_references),
            "fallback_voice": self.fallback_voice,
        }
