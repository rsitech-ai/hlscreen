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
MAX_COLS = 144
CHAR_WIDTH = 8.6
LINE_HEIGHT = 20
PADDING_X = 24
PADDING_TOP = 68
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
                title="Live market board",
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
                filename="confidence-degraded.svg",
                title="Data confidence pane",
                commands=[
                    [
                        str(HLS),
                        "live",
                        "--symbols",
                        "@107",
                        "--fixture-file",
                        "tests/fixtures/microstructure/sparse_trades.ndjson",
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
                        "--verify-parity",
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
                filename="health-panel.svg",
                title="Operations health panel",
                commands=[
                    [
                        str(HLS),
                        "doctor",
                        "--live",
                        "--simulate-health",
                        "writer-lag",
                        "--data-dir",
                        str(temp_dir / "health"),
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

        redactions = [(str(temp_dir), "<tmp>")]

        for screenshot in screenshots:
            lines = run_screenshot(screenshot, redactions)
            svg = render_svg(screenshot.title, lines)
            (OUT_DIR / screenshot.filename).write_text(svg, encoding="utf-8")
            print(f"wrote {OUT_DIR / screenshot.filename}")


def ensure_binary() -> None:
    subprocess.run(["cargo", "build", "-p", "hls-cli"], cwd=ROOT, check=True)


def run_screenshot(screenshot: Screenshot, redactions: list[tuple[str, str]]) -> list[str]:
    lines: list[str] = []
    for command in screenshot.commands:
        display = printable_command(command, redactions)
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
            lines.extend(redact(line, redactions) for line in output.splitlines())
        if completed.returncode != 0:
            raise SystemExit(
                f"command failed ({completed.returncode}): {display}\n{completed.stdout}"
            )
        lines.append("")
    while lines and lines[-1] == "":
        lines.pop()
    return wrap_lines(lines)


def printable_command(command: list[str], redactions: list[tuple[str, str]]) -> str:
    display = []
    for part in command:
        if part == str(HLS):
            display.append("./target/debug/hls")
        elif part.startswith(str(ROOT)):
            display.append(part.replace(str(ROOT) + "/", ""))
        else:
            display.append(redact(part, redactions))
    return " ".join(shlex.quote(part) for part in display)


def redact(value: str, redactions: list[tuple[str, str]]) -> str:
    redacted = value
    for before, after in redactions:
        redacted = redacted.replace(before, after)
    return redacted


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
        '<svg xmlns="http://www.w3.org/2000/svg" role="img" xml:space="preserve" '
        f'aria-label="{html.escape(title)} terminal screenshot" '
        f'viewBox="0 0 {width} {height}" width="{width}" height="{height}">',
        "<defs>",
        "<linearGradient id=\"bg\" x1=\"0\" x2=\"1\" y1=\"0\" y2=\"1\">",
        "<stop offset=\"0\" stop-color=\"#0d1117\"/>",
        "<stop offset=\"1\" stop-color=\"#18212b\"/>",
        "</linearGradient>",
        "</defs>",
        f'<rect width="{width}" height="{height}" rx="8" fill="url(#bg)"/>',
        f'<rect x="0" y="0" width="{width}" height="50" rx="8" fill="#141b24"/>',
        f'<rect x="0" y="42" width="{width}" height="8" fill="#141b24"/>',
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
        fill, weight = line_style(line)
        parts.append(
            f'<text x="{PADDING_X}" y="{y}" fill="{fill}" '
            'font-family="ui-monospace, SFMono-Regular, Menlo, Consolas, monospace" '
            f'font-size="14" font-weight="{weight}">{html.escape(line)}</text>'
        )
        y += LINE_HEIGHT
    parts.append("</svg>")
    return "\n".join(parts) + "\n"


def line_style(line: str) -> tuple[str, str]:
    if line.startswith("$ "):
        return "#7ee787", "700"
    if "HLSCREEN" in line:
        return "#79c0ff", "700"
    if line.startswith(("╭", "├", "╰", "─")):
        return "#53616f", "400"
    if line.startswith(("#   SYMBOL", "#  SYMBOL")):
        return "#ffa657", "700"
    if line.startswith(("│ SAFETY", "│ UNIVERSE", "│ SESSION")):
        return "#7ee787", "700"
    if line.startswith(
        (
            "│ INGEST",
            "│ STORAGE",
            "│ QUALITY",
            "│ LATENCY",
            "│ CONFIDENCE",
            "│ CONNECTION",
            "│ RECORDER",
            "│ RUNBOOK",
        )
    ):
        return "#f2cc60", "700"
    if "SELECTED SYMBOL" in line:
        return "#79c0ff", "700"
    if "● fresh" in line or "PASS" in line:
        return "#7ee787", "600"
    if "● FRESH" in line:
        return "#7ee787", "600"
    if "DEGRADED" in line or "WATCH" in line or line.startswith("- "):
        return "#f2cc60", "600"
    if line.startswith("  • "):
        return "#f2cc60", "600"
    if line.startswith("No wallet") or line.startswith("Read-only screen"):
        return "#a7b3c2", "400"
    if "READ-ONLY" in line:
        return "#ffa657", "700"
    return "#d8dee9", "400"


if __name__ == "__main__":
    main()
