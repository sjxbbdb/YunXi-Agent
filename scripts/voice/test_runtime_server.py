import unittest

from runtime_server import normalize_yunxi_brand_transcript


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


if __name__ == "__main__":
    unittest.main()
