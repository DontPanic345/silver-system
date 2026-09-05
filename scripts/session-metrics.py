#!/usr/bin/env python3
"""Extracts timing/cost/token metrics from a Claude Code session transcript
(JSONL), for comparing effort across experiments.

The harness already computes exactly this data per session — a `cost-state`
event, updated periodically, carrying cumulative wall-clock duration, model
inference time, tool time, per-model token usage, cost, and lines
added/removed. This script just finds the latest such event in a transcript
and formats it, rather than re-deriving any of it (re-deriving from raw
`usage` blocks on every assistant message would double-count retries and
drift from whatever the harness itself bills against).

Usage:
    python3 scripts/session-metrics.py [path/to/session.jsonl]

With no path, finds the most recently modified .jsonl under this project's
`~/.claude/projects/<project-slug>/` directory. That's a best-effort guess —
reliable when one session at a time has been working this repo, wrong if
several ran concurrently (e.g. background/cloud agents). Pass the path
explicitly if that guess is wrong; the transcript path for the current
session is generally visible in-session (e.g. quoted at the top of a
`/compact` summary, or under `~/.claude/projects/<slug>/<session-id>.jsonl`).
"""
import json
import sys
from pathlib import Path


def find_latest_transcript() -> Path:
    home = Path.home()
    projects_dir = home / ".claude" / "projects"
    cwd_slug = "-" + "-".join(Path.cwd().parts[1:])
    candidates = sorted(
        (projects_dir / cwd_slug).glob("*.jsonl"),
        key=lambda p: p.stat().st_mtime,
    )
    if not candidates:
        sys.exit(f"no .jsonl transcripts found under {projects_dir / cwd_slug}")
    return candidates[-1]


def human_ms(ms: float) -> str:
    secs = ms / 1000
    h, secs = divmod(secs, 3600)
    m, secs = divmod(secs, 60)
    parts = []
    if h:
        parts.append(f"{int(h)}h")
    if m:
        parts.append(f"{int(m)}m")
    parts.append(f"{secs:.0f}s")
    return " ".join(parts)


def main() -> None:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else find_latest_transcript()

    last_cost_state = None
    first_ts = last_ts = None
    with path.open() as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            obj = json.loads(line)
            if obj.get("type") == "cost-state":
                last_cost_state = obj
            ts = obj.get("timestamp")
            if ts:
                first_ts = first_ts or ts
                last_ts = ts

    if last_cost_state is None:
        sys.exit(f"no cost-state events found in {path}")

    cs = last_cost_state
    print(f"## Session metrics ({path.name})")
    print()
    print(f"- Wall-clock span (first→last transcript event): {first_ts} → {last_ts}")
    print(f"- Total duration (harness-tracked, includes idle gaps): {human_ms(cs['totalDuration'])}")
    print(f"- Model inference time (sum, all calls): {human_ms(cs['totalAPIDuration'])}")
    print(f"- Tool execution time (sum, all calls): {human_ms(cs['totalToolDuration'])}")
    print(f"- Lines added / removed: {cs['totalLinesAdded']} / {cs['totalLinesRemoved']}")
    print(f"- Total cost: ${cs['totalCostUSD']:.4f}")
    print("- Token usage by model:")
    for model, u in cs["modelUsage"].items():
        print(
            f"  - `{model}`: {u['inputTokens']:,} in, {u['outputTokens']:,} out, "
            f"{u.get('thinkingTokens', 0):,} thinking, "
            f"{u['cacheReadInputTokens']:,} cache-read, "
            f"{u['cacheCreationInputTokens']:,} cache-created "
            f"(${u['costUSD']:.4f})"
        )


if __name__ == "__main__":
    main()
