#!/usr/bin/env python3
"""Validate a versioned hlscreen soak evidence report."""

from __future__ import annotations

import argparse
import json
import math
import re
import sys
from datetime import datetime
from pathlib import Path
from typing import Any


COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")


def integer(value: Any, path: str, errors: list[str], *, minimum: int = 0) -> int | None:
    if isinstance(value, bool) or not isinstance(value, int):
        errors.append(f"{path} must be an integer")
        return None
    if value < minimum:
        errors.append(f"{path} must be >= {minimum}")
    return value


def number(value: Any, path: str, errors: list[str], *, minimum: float = 0.0) -> float | None:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        errors.append(f"{path} must be a number")
        return None
    parsed = float(value)
    if not math.isfinite(parsed):
        errors.append(f"{path} must be finite")
        return None
    if parsed < minimum:
        errors.append(f"{path} must be >= {minimum}")
    return parsed


def object_field(report: dict[str, Any], name: str, errors: list[str]) -> dict[str, Any]:
    value = report.get(name)
    if not isinstance(value, dict):
        errors.append(f"{name} must be an object")
        return {}
    return value


def parse_timestamp(value: Any, path: str, errors: list[str]) -> datetime | None:
    if not isinstance(value, str) or not value:
        errors.append(f"{path} must be a non-empty RFC3339 timestamp")
        return None
    try:
        parsed = datetime.fromisoformat(value.replace("Z", "+00:00"))
        if parsed.tzinfo is None:
            errors.append(f"{path} must include a timezone")
            return None
        return parsed
    except ValueError:
        errors.append(f"{path} must be a valid RFC3339 timestamp")
        return None


def validate(report: Any, minimum_duration_secs: int) -> list[str]:
    errors: list[str] = []
    if not isinstance(report, dict):
        return ["report root must be an object"]

    if report.get("schema_version") != 1:
        errors.append("schema_version must equal 1")
    if not isinstance(report.get("run_id"), str) or not report.get("run_id", "").strip():
        errors.append("run_id must be a non-empty string")
    if not isinstance(report.get("commit"), str) or not COMMIT_RE.fullmatch(report["commit"]):
        errors.append("commit must be a lowercase 40-character Git object ID")
    command = report.get("command")
    if not isinstance(command, list) or not command or not all(isinstance(item, str) and item for item in command):
        errors.append("command must be a non-empty array of non-empty strings")

    started = parse_timestamp(report.get("started_at"), "started_at", errors)
    ended = parse_timestamp(report.get("ended_at"), "ended_at", errors)
    duration = integer(report.get("duration_secs"), "duration_secs", errors)
    if duration is not None and duration < minimum_duration_secs:
        errors.append(f"duration_secs must be >= {minimum_duration_secs}")
    if started is not None and ended is not None:
        wall_duration = (ended - started).total_seconds()
        if wall_duration < 0:
            errors.append("ended_at must not precede started_at")
        elif duration is not None and abs(wall_duration - duration) > 2:
            errors.append("timestamp span and duration_secs differ by more than 2 seconds")

    if integer(report.get("exit_code"), "exit_code", errors) not in (None, 0):
        errors.append("exit_code must equal 0")
    if report.get("clean_shutdown") is not True:
        errors.append("clean_shutdown must equal true")

    samples = report.get("samples")
    if not isinstance(samples, list) or len(samples) < 2:
        errors.append("samples must contain at least two resource samples")
        samples = []
    previous_elapsed = -1
    previous_data = -1
    rss_values: list[int] = []
    for index, sample in enumerate(samples):
        if not isinstance(sample, dict):
            errors.append(f"samples[{index}] must be an object")
            continue
        elapsed = integer(sample.get("elapsed_secs"), f"samples[{index}].elapsed_secs", errors)
        number(sample.get("cpu_percent"), f"samples[{index}].cpu_percent", errors)
        rss = integer(sample.get("rss_bytes"), f"samples[{index}].rss_bytes", errors)
        data = integer(sample.get("data_bytes"), f"samples[{index}].data_bytes", errors)
        if elapsed is not None:
            if elapsed <= previous_elapsed:
                errors.append("sample elapsed_secs values must be strictly increasing")
            previous_elapsed = elapsed
        if data is not None:
            if data < previous_data:
                errors.append("sample data_bytes values must be monotonic")
            previous_data = data
        if rss is not None:
            rss_values.append(rss)
    if duration is not None and previous_elapsed >= 0 and previous_elapsed < duration - 2:
        errors.append("last resource sample does not cover the reported duration")

    summary = object_field(report, "summary", errors)
    for field in (
        "symbols", "subscriptions", "ws_messages", "market_events", "reconnects",
        "data_gaps", "unrepaired_gaps", "parser_drops", "backfill_requests_failed",
    ):
        integer(summary.get(field), f"summary.{field}", errors)
    for field in ("symbols", "subscriptions", "ws_messages", "market_events"):
        if summary.get(field) == 0:
            errors.append(f"summary.{field} must be greater than zero")
    for field in ("unrepaired_gaps", "parser_drops", "backfill_requests_failed"):
        if isinstance(summary.get(field), int) and summary[field] > 0:
            errors.append(f"summary.{field} must equal zero")
    if (
        isinstance(summary.get("unrepaired_gaps"), int)
        and isinstance(summary.get("data_gaps"), int)
        and summary["unrepaired_gaps"] > summary["data_gaps"]
    ):
        errors.append("summary.unrepaired_gaps cannot exceed summary.data_gaps")

    replay = object_field(report, "replay", errors)
    if replay.get("first_status") not in ("baseline_written", "passed"):
        errors.append("replay.first_status must be baseline_written or passed")
    if replay.get("second_status") != "passed":
        errors.append("replay.second_status must equal passed")
    for field in ("drift", "missing", "extra"):
        value = integer(replay.get(field), f"replay.{field}", errors)
        if value not in (None, 0):
            errors.append(f"replay.{field} must equal zero")

    limits = object_field(report, "limits", errors)
    max_rss_growth = integer(
        limits.get("max_rss_growth_bytes"), "limits.max_rss_growth_bytes", errors
    )
    if max_rss_growth is not None and len(rss_values) >= 2:
        rss_growth = max(rss_values) - rss_values[0]
        if rss_growth > max_rss_growth:
            errors.append(
                f"RSS growth {rss_growth} exceeds limit {max_rss_growth} bytes"
            )

    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("report", type=Path)
    parser.add_argument("--minimum-duration-secs", type=int, default=900)
    args = parser.parse_args()
    if args.minimum_duration_secs < 1:
        parser.error("--minimum-duration-secs must be positive")

    try:
        report = json.loads(args.report.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        print(f"soak_report=invalid: {error}", file=sys.stderr)
        return 1

    errors = validate(report, args.minimum_duration_secs)
    if errors:
        print("soak_report=invalid", file=sys.stderr)
        for error in errors:
            print(f"- {error}", file=sys.stderr)
        return 1

    print(f"soak_report=passed run_id={report['run_id']} duration_secs={report['duration_secs']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
