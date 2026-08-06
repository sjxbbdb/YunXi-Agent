"""Deterministic speech style selection for the single CosyVoice3 chain."""

from __future__ import annotations


VOICE_INSTRUCTIONS: dict[str, str] = {
    "gentle": "请用自然、温柔、亲近且清晰的中文语气表达。",
    "happy": "请用开心、明亮但不过度夸张的中文语气表达。",
    "concerned": "请用轻柔、关心、稍慢一些的中文语气表达。",
    "serious": "请用沉稳、认真、清晰且克制的中文语气表达。",
    "sad": "请用低缓、温和、带有共情但不过度悲伤的中文语气表达。",
    "surprised": "请用自然惊讶、稍有起伏但不过度夸张的中文语气表达。",
    "angry": "请用克制、明确、坚定但不具有攻击性的中文语气表达。",
}


def infer_reply_emotion(text: str, explicit: str | None = None) -> str:
    requested = (explicit or "").strip().lower()
    if requested in VOICE_INSTRUCTIONS:
        return requested
    rules = (
        ("angry", ("气死", "生气", "愤怒", "讨厌", "烦死", "太过分")),
        ("sad", ("难过", "伤心", "哭", "失落", "遗憾", "心疼")),
        ("concerned", ("担心", "不舒服", "还好吗", "没事吧", "注意休息", "抱抱")),
        ("happy", ("开心", "高兴", "太好了", "哈哈", "真棒", "喜欢")),
        ("surprised", ("没想到", "竟然", "真的吗", "居然", "天哪")),
        ("serious", ("必须", "认真", "重要", "风险", "警告", "不能")),
    )
    for emotion, markers in rules:
        if any(marker in text for marker in markers):
            return emotion
    return "gentle"


def instruction_for_reply(text: str, explicit: str | None = None) -> tuple[str, str]:
    emotion = infer_reply_emotion(text, explicit)
    instruction = f"You are a helpful assistant. {VOICE_INSTRUCTIONS[emotion]}<|endofprompt|>"
    return emotion, instruction
