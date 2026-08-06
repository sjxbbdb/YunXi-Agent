import unittest
import logging

from runtime_server import (
    VoiceModelPrivacyFilter,
    normalize_yunxi_brand_transcript,
    resolve_voice_language,
)
from quality_runtime_server import infer_reply_emotion


class YunXiBrandTranscriptTests(unittest.TestCase):
    def test_normalizes_brand_homophones_only_with_brand_suffix(self) -> None:
        self.assertEqual(
            normalize_yunxi_brand_transcript(
                "云系智能体、云戏助手和云汐 智能体都指向同一品牌。"
            ),
            "云熙、云熙和云熙都指向同一品牌。",
        )

    def test_normalizes_spoken_self_reference_to_yunxi_name(self) -> None:
        self.assertEqual(
            normalize_yunxi_brand_transcript("我是云溪，也可以叫我云西。"),
            "我是云熙，也可以叫我云熙。",
        )

    def test_keeps_unrelated_homophones_unchanged(self) -> None:
        self.assertEqual(
            normalize_yunxi_brand_transcript("这是云系架构，旁边有一条云溪。"),
            "这是云系架构，旁边有一条云溪。",
        )

    def test_model_privacy_filter_drops_cosyvoice_input_text(self) -> None:
        privacy_filter = VoiceModelPrivacyFilter()
        sensitive = logging.LogRecord(
            "root", logging.INFO, __file__, 1, "synthesis text private sentence", (), None
        )
        diagnostic = logging.LogRecord(
            "root", logging.INFO, __file__, 1, "loading model", (), None
        )
        self.assertFalse(privacy_filter.filter(sensitive))
        self.assertTrue(privacy_filter.filter(diagnostic))

    def test_auto_language_defaults_to_chinese_for_short_clips(self) -> None:
        self.assertEqual(resolve_voice_language("auto", "zh"), "zh")
        self.assertEqual(resolve_voice_language(None, "zh"), "zh")
        self.assertEqual(resolve_voice_language("en", "zh"), "en")

    def test_reply_emotion_is_deterministic_without_an_extra_model(self) -> None:
        self.assertEqual(infer_reply_emotion("太好了，我真的很开心。"), "happy")
        self.assertEqual(infer_reply_emotion("听起来很难过，我有点心疼你。"), "sad")
        self.assertEqual(infer_reply_emotion("我在这里，慢慢说。"), "gentle")
        self.assertEqual(infer_reply_emotion("普通内容", "serious"), "serious")


if __name__ == "__main__":
    unittest.main()
