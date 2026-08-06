#!/usr/bin/env python3
"""Process-level mock smoke for the YunXi voice MVP."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
import threading
from pathlib import Path

from runtime_server import MockVoiceModels, VoiceHttpServer


def resolve_yunxi() -> str:
    configured = os.environ.get("YUNXI_EXE", "").strip()
    if configured:
        executable = Path(configured).expanduser().resolve()
        if not executable.is_file():
            raise RuntimeError(f"YUNXI_EXE does not exist: {executable}")
        return str(executable)
    executable = shutil.which("yunxi") or shutil.which("yunxi.exe")
    if not executable:
        raise RuntimeError("yunxi is not on PATH; set YUNXI_EXE to yunxi.exe")
    return executable


def run(yunxi: str, env: dict[str, str], *arguments: str) -> dict:
    command = [yunxi, *arguments]
    completed = subprocess.run(
        command,
        env=env,
        capture_output=True,
        text=True,
        encoding="utf-8",
        timeout=180,
        check=False,
    )
    if completed.returncode != 0:
        raise RuntimeError(
            "voice smoke command failed "
            f"exit={completed.returncode} stderr={completed.stderr.strip()}"
        )
    try:
        return json.loads(completed.stdout)
    except json.JSONDecodeError as error:
        raise RuntimeError(
            f"voice smoke returned invalid JSON: {completed.stdout.strip()}"
        ) from error


def is_wav(path: Path) -> bool:
    data = path.read_bytes()
    return len(data) >= 44 and data[0:4] == b"RIFF" and data[8:12] == b"WAVE"


def main() -> int:
    yunxi = resolve_yunxi()
    server = VoiceHttpServer(("127.0.0.1", 0), MockVoiceModels())
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    port = server.server_address[1]
    try:
        with tempfile.TemporaryDirectory(prefix="yunxi-voice-mvp-") as directory:
            root = Path(directory)
            workspace = root / "workspace"
            home = root / "home"
            workspace.mkdir()
            home.mkdir()
            question = root / "question.wav"
            reply = root / "reply.wav"
            env = os.environ.copy()
            env["YUNXI_HOME"] = str(home)
            env["YUNXI_VOICE_RUNTIME_URL"] = f"http://127.0.0.1:{port}"
            env["YUNXI_VOICE_MOCK_TRANSCRIPT"] = "请用一句温暖的话回应我。"

            doctor = run(yunxi, env, "--json", "voice", "doctor")
            speak = run(
                yunxi,
                env,
                "--json",
                "voice",
                "speak",
                "生成一段测试音频",
                "--output",
                str(question),
            )
            chat = run(
                yunxi,
                env,
                "--offline",
                "--json",
                "--cwd",
                str(workspace),
                "voice",
                "chat",
                "--input",
                str(question),
                "--output",
                str(reply),
            )
            if doctor.get("status") != "ready":
                raise RuntimeError("voice doctor did not report ready")
            if speak.get("status") != "completed" or not is_wav(question):
                raise RuntimeError("voice speak did not create a WAV")
            if chat.get("status") != "completed" or not is_wav(reply):
                raise RuntimeError("voice chat did not create a WAV")
            print(
                json.dumps(
                    {
                        "status": "pass",
                        "runtime": doctor["health"],
                        "transcript": chat["transcript"],
                        "response": chat["response"],
                        "timingsMs": {
                            "stt": chat["sttMs"],
                            "runtime": chat["runtimeMs"],
                            "tts": chat["ttsMs"],
                            "total": chat["totalMs"],
                        },
                        "questionBytes": question.stat().st_size,
                        "replyBytes": reply.stat().st_size,
                        "isolatedWorkspaceState": (workspace / ".yunxi").exists(),
                    },
                    ensure_ascii=False,
                    indent=2,
                )
            )
    finally:
        server.shutdown()
        server.server_close()
        thread.join(timeout=5)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
