#!/usr/bin/env python3
"""Generate deterministic SVG terminal screenshots for README/docs."""

from __future__ import annotations

import html
import shlex
import subprocess
import tempfile
import textwrap
from dataclasses import dataclass
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
OUT_DIR = ROOT / "docs" / "assets" / "screenshots"
HLS = ROOT / "target" / "debug" / "hls"
FIXTURE = "tests/fixtures/hyperliquid/ws_mock_live.ndjson"
MAX_COLS = 108
CHAR_WIDTH = 8.6
LINE_HEIGHT = 20
PADDING_X = 24
PADDING_TOP = 60
PADDING_BOTTOM = 28


@dataclass(frozen=True)
class Screenshot:
    filename: str
    title: str
    commands: list[list[str]]


def main() -> None:
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    ensure_binary()

    with tempfile.TemporaryDirectory(prefix="hlscreen-screenshots.") as temp:
        temp_dir = Path(temp)
        screenshots = [
            Screenshot(
                filename="live-screen.svg",
                title="Terminal live screen test capture",
                commands=[
                    [
                        str(HLS),
                        "live",
                        "--symbols",
                        "@107",
                        "--fixture-file",
                        FIXTURE,
                        "--preset",
                        "thin_books",
                        "--once",
                    ]
                ],
            ),
            Screenshot(
                filename="record-replay.svg",
                title="Record and replay",
                commands=[
                    [
                        str(HLS),
                        "record",
                        "--symbols",
                        "@107",
                        "--fixture-file",
                        FIXTURE,
                        "--raw",
                        "--normalized",
                        "--run-id",
                        "screenshot",
                        "--data-dir",
                        str(temp_dir),
                    ],
                    [
                        str(HLS),
                        "replay",
                        "--data-dir",
                        str(temp_dir),
                        "--run-id",
                        "screenshot",
                    ],
                ],
            ),
            Screenshot(
                filename="health-json.svg",
                title="Read-only health JSON",
                commands=[
                    [
                        str(HLS),
                        "doctor",
                        "--live",
                        "--json",
                        "--simulate-health",
                        "healthy",
                    ]
                ],
            ),
            Screenshot(
                filename="symbols.svg",
                title="Spot symbol metadata",
                commands=[
                    [
                        str(HLS),
                        "symbols",
                        "--top",
                        "2",
                        "--asset-contexts-file",
                        "tests/fixtures/hyperliquid/spot_meta_and_asset_ctxs.json",
                    ]
                ],
            ),
        ]

        for screenshot in screenshots:
            lines = run_screenshot(screenshot)
            svg = render_svg(screenshot.title, lines)
            (OUT_DIR / screenshot.filename).write_text(svg, encoding="utf-8")
            print(f"wrote {OUT_DIR / screenshot.filename}")


def ensure_binary() -> None:
    if HLS.exists():
        return
    subprocess.run(["cargo", "build", "-p", "hls-cli"], cwd=ROOT, check=True)


def run_screenshot(screenshot: Screenshot) -> list[str]:
    lines: list[str] = []
    for command in screenshot.commands:
        display = printable_command(command)
        lines.append(f"$ {display}")
        completed = subprocess.run(
            command,
            cwd=ROOT,
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
        )
        output = completed.stdout.rstrip("\n")
        if output:
            lines.extend(output.splitlines())
        if completed.returncode != 0:
            raise SystemExit(
                f"command failed ({completed.returncode}): {display}\n{completed.stdout}"
            )
        lines.append("")
    while lines and lines[-1] == "":
        lines.pop()
    return wrap_lines(lines)


def printable_command(command: list[str]) -> str:
    display = []
    for part in command:
        if part == str(HLS):
            display.append("./target/debug/hls")
        elif part.startswith(str(ROOT)):
            display.append(part.replace(str(ROOT) + "/", ""))
        else:
            display.append(part)
    return " ".join(shlex.quote(part) for part in display)


def wrap_lines(lines: list[str]) -> list[str]:
    wrapped: list[str] = []
    for line in lines:
        if len(line) <= MAX_COLS:
            wrapped.append(line)
            continue
        wrapped.extend(
            textwrap.wrap(
                line,
                width=MAX_COLS,
                subsequent_indent="  ",
                break_long_words=False,
                break_on_hyphens=False,
            )
        )
    return wrapped


def render_svg(title: str, lines: list[str]) -> str:
    width = int(MAX_COLS * CHAR_WIDTH + PADDING_X * 2)
    height = PADDING_TOP + PADDING_BOTTOM + len(lines) * LINE_HEIGHT
    parts = [
        '<svg xmlns="http://www.w3.org/2000/svg" role="img" '
        f'aria-label="{html.escape(title)} terminal screenshot" '
        f'viewBox="0 0 {width} {height}" width="{width}" height="{height}">',
        "<defs>",
        "<linearGradient id=\"bg\" x1=\"0\" x2=\"1\" y1=\"0\" y2=\"1\">",
        "<stop offset=\"0\" stop-color=\"#101418\"/>",
        "<stop offset=\"1\" stop-color=\"#1d242c\"/>",
        "</linearGradient>",
        "</defs>",
        f'<rect width="{width}" height="{height}" rx="10" fill="url(#bg)"/>',
        '<circle cx="24" cy="24" r="6" fill="#ff5f57"/>',
        '<circle cx="44" cy="24" r="6" fill="#ffbd2e"/>',
        '<circle cx="64" cy="24" r="6" fill="#28c840"/>',
        f'<text x="{PADDING_X}" y="40" fill="#d8dee9" '
        'font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" '
        'font-size="14" font-weight="700">'
        f"{html.escape(title)}</text>",
    ]
    y = PADDING_TOP
    for line in lines:
        fill = "#98c379" if line.startswith("$ ") else "#d8dee9"
        parts.append(
            f'<text x="{PADDING_X}" y="{y}" fill="{fill}" '
            'font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" '
            f'font-size="14">{html.escape(line)}</text>'
        )
        y += LINE_HEIGHT
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


if __name__ == "__main__":
    main()
