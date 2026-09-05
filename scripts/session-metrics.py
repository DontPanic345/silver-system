#!/usr/bin/env python3
"""Extracts timing/cost/token metrics from a Claude Code session transcript
(JSONL), for comparing effort across experiments.

The harness already computes exactly this data per session — a `cost-state`
event, updated periodically, carrying cumulative wall-clock duration, model
inference time, tool time, per-model token usage, cost, and lines
added/removed. This script's primary path just finds the latest such event
in a transcript and formats it, rather than re-deriving any of it
(re-deriving from raw `usage` blocks on every assistant message would
double-count retries and drift from whatever the harness itself bills
against).

## The fallback, and why it exists

`cost-state` is not guaranteed to appear at all: it showed up zero times in
an otherwise-ordinary session transcript this script was run against
mid-session, per this repo's own "when you stop, run this" convention —
exactly the moment this script most needs to work. A second run, from a
subagent spawned inside that same session, hit the identical failure — and
turned up an even sharper problem: a subagent spawned via the `Agent` tool
does not get its own transcript file at all. It appends straight into the
orchestrating session's own `.jsonl`, un-flagged as a sidechain. So a
"fresh session" run from inside another session is never actually isolated
at the transcript level, and this script's job — comparing one run's effort
against another's — cannot lean on `cost-state` alone if any run in the
comparison was launched that way.

Erroring out in either case defeats the instrument's whole point
(comparability across runs), so when no `cost-state` event is found, this
script falls back to token totals derived directly from the transcript's own
assistant-message `usage` blocks, deduplicated by message `id` (a single
logical assistant turn is split across multiple `assistant`-type lines, one
per content block — thinking, then each tool call — and every one of those
lines repeats that turn's *whole-message* `usage`, not an incremental
delta; summing them all would overcount by however many blocks the turn
had). The fallback is clearly labeled as such and does not attempt to
reconstruct total cost in USD or the model/tool duration split — both need
a live pricing table and per-call timing this script has no independent
source for, and guessing either would silently drift from whatever the
harness itself would have billed, the exact failure the module doc comment
above already rejects for the primary path. It also does not attempt lines
added/removed — `git diff`/`git log` are the honest source for that when
this fallback fires.

**Known sharp edge:** if this script is run from inside a subagent spawned
by another still-running session, the fallback's token totals are only
correct if nothing else has written `assistant`-type messages into that
same transcript file since the subagent started — a real risk given
subagents share the parent's file (see above). No general fix for this is
attempted here; the fallback's numbers should be treated as an upper bound
in that situation, not an exact per-agent figure.

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


def empty_usage_bucket() -> dict:
    return {
        "inputTokens": 0,
        "outputTokens": 0,
        "thinkingTokens": 0,
        "cacheReadInputTokens": 0,
        "cacheCreationInputTokens": 0,
    }


def main() -> None:
    path = Path(sys.argv[1]) if len(sys.argv) > 1 else find_latest_transcript()

    last_cost_state = None
    first_ts = last_ts = None
    # Fallback-only accumulation: per-model usage totals derived straight
    # from assistant-message `usage` blocks, deduplicated by message id (see
    # module doc comment for why dedup is required). Built unconditionally
    # (it's cheap) so falling back never needs a second pass over the file.
    seen_message_ids: set[str] = set()
    fallback_usage: dict[str, dict] = {}

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

            if obj.get("type") == "assistant":
                msg = obj.get("message", {})
                msg_id = msg.get("id")
                usage = msg.get("usage")
                model = msg.get("model")
                if not (msg_id and usage and model) or msg_id in seen_message_ids:
                    continue
                seen_message_ids.add(msg_id)
                bucket = fallback_usage.setdefault(model, empty_usage_bucket())
                bucket["inputTokens"] += usage.get("input_tokens", 0)
                bucket["outputTokens"] += usage.get("output_tokens", 0)
                bucket["thinkingTokens"] += (usage.get("output_tokens_details") or {}).get(
                    "thinking_tokens", 0
                )
                bucket["cacheReadInputTokens"] += usage.get("cache_read_input_tokens", 0)
                bucket["cacheCreationInputTokens"] += usage.get(
                    "cache_creation_input_tokens", 0
                )

    print(f"## Session metrics ({path.name})")
    print()
    print(f"- Wall-clock span (first→last transcript event): {first_ts} → {last_ts}")

    if last_cost_state is not None:
        cs = last_cost_state
        print(f"- Total duration (harness-tracked, includes idle gaps): {human_ms(cs['totalDuration'])}")
        print(f"- Model inference time (sum, all calls): {human_ms(cs['totalAPIDuration'])}")
        print(f"- Tool execution time (sum, all calls): {human_ms(cs['totalToolDuration'])}")
        print(f"- Lines added / removed: {cs['totalLinesAdded']} / {cs['totalLinesRemoved']}")
        print(f"- Total cost: ${cs['totalCostUSD']:.4f}")
        print("- Token usage by model (harness-tracked):")
        for model, u in cs["modelUsage"].items():
            print(
                f"  - `{model}`: {u['inputTokens']:,} in, {u['outputTokens']:,} out, "
                f"{u.get('thinkingTokens', 0):,} thinking, "
                f"{u['cacheReadInputTokens']:,} cache-read, "
                f"{u['cacheCreationInputTokens']:,} cache-created "
                f"(${u['costUSD']:.4f})"
            )
        return

    print(
        "- No `cost-state` event found in this transcript — falling back to "
        "totals derived directly from this transcript's own assistant-message "
        "`usage` blocks (see this script's module doc comment for why, and "
        "what the fallback can't reconstruct):"
    )
    print("- Total duration / model / tool time split: not available (no cost-state event)")
    print("- Lines added / removed: not available (no cost-state event — see `git diff --stat`)")
    print("- Total cost: not available (no cost-state event, and no pricing table here to derive it from)")
    if not fallback_usage:
        print("- Token usage by model: none found (no assistant messages with usage in this transcript)")
    else:
        print("- Token usage by model (derived, deduplicated by message id, cost not computed):")
        for model, u in fallback_usage.items():
            print(
                f"  - `{model}`: {u['inputTokens']:,} in, {u['outputTokens']:,} out, "
                f"{u['thinkingTokens']:,} thinking, "
                f"{u['cacheReadInputTokens']:,} cache-read, "
                f"{u['cacheCreationInputTokens']:,} cache-created"
            )


if __name__ == "__main__":
    main()
