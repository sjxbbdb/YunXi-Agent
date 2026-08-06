"""Deterministic speech style selection for the single CosyVoice3 chain."""

from __future__ import annotations


VOICE_IDENTITY_INSTRUCTION = (
    "保持参考音色和说话人身份。声线像十九到二十一岁的年轻女性，"
    "音高略低、靠近自然胸声，音量稳定，避免尖亮、喊叫或突然拔高。"
)

VOICE_INSTRUCTIONS: dict[str, str] = {
    "gentle": "请自然温柔、亲近地说这句话。语速自然，停顿柔和，句尾自然下落，不要过度气声或刻意变细。",
    "happy": "请轻快、自然开心地说这句话。声音带浅笑，音高只小幅上扬，语速略快，保持普通说话的音量，不要提高响度或突然拔高。",
    "concerned": "请用关切、心疼但克制的声音说这句话。语速稍慢，停顿自然，像在认真安慰对方，保持稳定音量。",
    "serious": "请用严肃、坚定但年轻的声音说这句话。音高略低，语速平稳，关键词重音清楚，句尾自然下落。",
    "sad": "请用轻微低落、克制的声音说这句话。语速稍慢，尾音自然下沉，只带少量气声，不要显得苍老。",
    "surprised": "请用短暂、自然的惊讶说这句话。只在关键词上轻微抬高音调，保持音量稳定，不要突然尖叫。",
    "angry": "请用克制、不满但不要喊叫的声音说这句话。语速稍快，咬字清楚，重音适度，不提高整体音量。",
}


def infer_reply_emotion(text: str, explicit: str | None = None) -> str:
    requested = (explicit or "").strip().lower()
    if requested in VOICE_INSTRUCTIONS:
        return requested
    rules = (
        ("angry", ("气死", "生气", "愤怒", "恼火", "火大", "讨厌", "烦死", "太过分")),
        ("sad", ("悲伤", "难过", "难受", "伤心", "哭", "失落", "低落", "沮丧", "遗憾", "心疼")),
        ("concerned", ("担心", "关心", "在意", "不舒服", "还好吗", "没事吧", "注意休息", "照顾好", "抱抱")),
        ("happy", ("开心", "高兴", "快乐", "兴奋", "太好了", "好耶", "哈哈", "真棒", "太棒", "喜欢")),
        ("surprised", ("惊讶", "震惊", "没想到", "竟然", "真的吗", "真的假的", "居然", "天哪")),
        ("serious", ("严肃", "郑重", "必须", "认真", "重要", "风险", "警告", "不能")),
    )
    for emotion, markers in rules:
        if any(marker in text for marker in markers):
            return emotion
    return "gentle"


def instruction_for_reply(text: str, explicit: str | None = None) -> tuple[str, str]:
    emotion = infer_reply_emotion(text, explicit)
    instruction = (
        f"You are a helpful assistant. {VOICE_IDENTITY_INSTRUCTION}"
        f"{VOICE_INSTRUCTIONS[emotion]}<|endofprompt|>"
    )
    return emotion, instruction
