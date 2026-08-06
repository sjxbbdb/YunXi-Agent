import json
import tempfile
import time
import unittest
from pathlib import Path
from typing import Any

from runtime_server import mock_wav
from voice_profile import VoiceProfile, VoiceProfileError
from voice_router import CircuitBreaker, VoiceBackendRouter


class FakeBackend:
    def __init__(self, name: str, ready: bool = True) -> None:
        self.name = name
        self.ready = ready
        self.transcript: dict[str, Any] = {
            "text": f"{name} transcript",
            "language": "zh",
            "emotion": None,
            "audio_events": [],
        }
        self.audio = mock_wav(name)
        self.transcribe_error: BaseException | None = None
        self.synthesize_error: BaseException | None = None
        self.transcribe_delay = 0.0
        self.synthesize_delay = 0.0
        self.transcribe_inputs: list[bytes] = []
        self.synthesize_inputs: list[tuple[str, str, str | None]] = []

    def health(self) -> dict[str, Any]:
        return {
            "schema_version": 1,
            "status": "ok" if self.ready else "unavailable",
            "stt": {"provider": self.name, "ready": self.ready},
            "tts": {"provider": self.name, "ready": self.ready},
            "preset_voices": ["中文女"],
            "capabilities": {
                "voice_clone": self.name == "quality" and self.ready,
                "emotion_control": self.name == "quality" and self.ready,
            },
        }

    def transcribe(self, audio: bytes, _language: str | None) -> dict[str, Any]:
        self.transcribe_inputs.append(audio)
        if self.transcribe_delay:
            time.sleep(self.transcribe_delay)
        if self.transcribe_error:
            raise self.transcribe_error
        return self.transcript

    def synthesize(self, text: str, voice: str, emotion: str | None = None) -> bytes:
        self.synthesize_inputs.append((text, voice, emotion))
        if self.synthesize_delay:
            time.sleep(self.synthesize_delay)
        if self.synthesize_error:
            raise self.synthesize_error
        return self.audio


class VoiceBackendRouterTests(unittest.TestCase):
    def setUp(self) -> None:
        self.stable = FakeBackend("stable")
        self.quality = FakeBackend("quality")

    def test_stable_mode_never_calls_quality(self) -> None:
        router = VoiceBackendRouter(self.stable, self.quality, "stable")
        audio = mock_wav("input")
        self.assertEqual(router.transcribe(audio, "zh")["text"], "stable transcript")
        self.assertIs(router.synthesize("reply", "中文女"), self.stable.audio)
        self.assertEqual(self.quality.transcribe_inputs, [])
        self.assertEqual(self.quality.synthesize_inputs, [])

    def test_quality_mode_routes_each_end_to_quality(self) -> None:
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        self.assertEqual(router.transcribe(mock_wav("input"), None)["text"], "quality transcript")
        self.assertIs(router.synthesize("reply", "中文女", "gentle"), self.quality.audio)
        health = router.health()
        self.assertEqual(health["active"], {"stt": "quality", "tts": "quality"})
        self.assertTrue(health["capabilities"]["voice_clone"])
        self.assertEqual(health["preset_voices"], ["中文女"])

    def test_auto_mode_skips_unready_quality(self) -> None:
        self.quality.ready = False
        router = VoiceBackendRouter(self.stable, self.quality, "auto")
        self.assertEqual(router.transcribe(mock_wav("input"), None)["text"], "stable transcript")
        self.assertEqual(self.quality.transcribe_inputs, [])

    def test_stt_failure_reuses_same_audio_for_stable_fallback(self) -> None:
        self.quality.transcribe_error = ImportError("injected")
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        audio = mock_wav("same object")
        result = router.transcribe(audio, "zh")
        self.assertEqual(result["text"], "stable transcript")
        self.assertIs(self.quality.transcribe_inputs[0], audio)
        self.assertIs(self.stable.transcribe_inputs[0], audio)
        self.assertEqual(router.health()["fallback"]["count"], 1)

    def test_empty_quality_stt_falls_back(self) -> None:
        self.quality.transcript = {"text": ""}
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        self.assertEqual(router.transcribe(mock_wav("input"), None)["text"], "stable transcript")

    def test_tts_failure_reuses_same_text_for_stable_fallback(self) -> None:
        self.quality.synthesize_error = RuntimeError("injected")
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        text = "the same reply object"
        result = router.synthesize(text, "中文女", "gentle")
        self.assertIs(result, self.stable.audio)
        self.assertIs(self.quality.synthesize_inputs[0][0], text)
        self.assertIs(self.stable.synthesize_inputs[0][0], text)
        self.assertEqual(self.stable.synthesize_inputs[0][2], "gentle")

    def test_invalid_quality_wav_falls_back(self) -> None:
        self.quality.audio = b"not a wav"
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        self.assertIs(router.synthesize("reply", "中文女"), self.stable.audio)

    def test_quality_timeout_falls_back_without_waiting_for_worker(self) -> None:
        self.quality.transcribe_delay = 0.2
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        router.timeout = 0.01
        started = time.monotonic()
        self.assertEqual(router.transcribe(mock_wav("input"), None)["text"], "stable transcript")
        self.assertLess(time.monotonic() - started, 0.15)

    def test_circuit_breaker_stops_repeated_quality_calls(self) -> None:
        self.quality.transcribe_error = RuntimeError("injected")
        router = VoiceBackendRouter(self.stable, self.quality, "quality")
        router._circuits["stt"] = CircuitBreaker(threshold=2, cooldown_seconds=60)
        audio = mock_wav("input")
        router.transcribe(audio, None)
        router.transcribe(audio, None)
        router.transcribe(audio, None)
        self.assertEqual(len(self.quality.transcribe_inputs), 2)
        self.assertTrue(router.health()["circuit_breaker"]["stt"]["open"])


class VoiceProfileTests(unittest.TestCase):
    def test_rejects_missing_and_corrupt_profiles(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(VoiceProfileError):
                VoiceProfile.load(root / "missing.json")
            corrupt = root / "corrupt.json"
            corrupt.write_text("{", encoding="utf-8")
            with self.assertRaises(VoiceProfileError):
                VoiceProfile.load(corrupt)

    def test_loads_relative_private_audio_references(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "references").mkdir()
            neutral = root / "references" / "neutral.wav"
            gentle = root / "references" / "gentle.wav"
            neutral.write_bytes(mock_wav("neutral"))
            gentle.write_bytes(mock_wav("gentle"))
            profile_path = root / "profile.json"
            profile_path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "id": "yunxi-primary",
                        "backend": "indextts2",
                        "reference_audio": "references/neutral.wav",
                        "reference_transcript": "这是一段准确的参考文字。",
                        "emotion_references": {"gentle": "references/gentle.wav"},
                        "speed": 1.0,
                        "fallback_voice": "中文女",
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )
            profile = VoiceProfile.load(profile_path)
            self.assertEqual(profile.profile_id, "yunxi-primary")
            self.assertEqual(profile.reference_audio, neutral.resolve())
            self.assertEqual(profile.emotion_audio("gentle"), gentle.resolve())
            self.assertNotIn("reference_audio", profile.public_health())


if __name__ == "__main__":
    unittest.main()
