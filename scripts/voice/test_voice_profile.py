import json
import tempfile
import unittest
from pathlib import Path

from runtime_server import mock_wav
from voice_profile import VoiceProfile, VoiceProfileError


class VoiceProfileTests(unittest.TestCase):
    def test_rejects_missing_corrupt_and_legacy_profiles(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            with self.assertRaises(VoiceProfileError):
                VoiceProfile.load(root / "missing.json")
            corrupt = root / "corrupt.json"
            corrupt.write_text("{", encoding="utf-8")
            with self.assertRaises(VoiceProfileError):
                VoiceProfile.load(corrupt)
            reference = root / "neutral.wav"
            reference.write_bytes(mock_wav("reference"))
            legacy = root / "legacy.json"
            legacy.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "id": "yunxi-primary",
                        "backend": "indextts2",
                        "reference_audio": "neutral.wav",
                        "reference_transcript": "准确的参考文字。",
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )
            with self.assertRaises(VoiceProfileError):
                VoiceProfile.load(legacy)

    def test_loads_relative_cosyvoice3_reference(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            references = root / "references"
            references.mkdir()
            neutral = references / "neutral.wav"
            neutral.write_bytes(mock_wav("neutral"))
            profile_path = root / "profile.json"
            profile_path.write_text(
                json.dumps(
                    {
                        "schema_version": 1,
                        "id": "yunxi-primary",
                        "backend": "cosyvoice3",
                        "reference_audio": "references/neutral.wav",
                        "reference_transcript": "这是一段准确的参考文字。",
                        "speed": 1.0,
                    },
                    ensure_ascii=False,
                ),
                encoding="utf-8",
            )
            profile = VoiceProfile.load(profile_path)
            self.assertEqual(profile.profile_id, "yunxi-primary")
            self.assertEqual(profile.reference_audio, neutral.resolve())
            self.assertNotIn("reference_audio", profile.public_health())


if __name__ == "__main__":
    unittest.main()
