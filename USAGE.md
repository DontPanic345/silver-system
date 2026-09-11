# Usage tracking

Raw `/cost` snapshots for the seven-night compounding run (see `NIGHTLY.md`).
Nothing here is read by a nightly agent — this file exists purely for the
human tracking min/maxing a Pro-plan-style weekly budget across the week,
and it's fine to just paste `/cost`'s output verbatim, before and after a
run. Append-only; oldest first.

| Night | Date | Branch/commit | Session % before → after | Week % before → after |
|---|---|---|---|---|
| 1 | 2026-09-07 | `experiment/effort-opus-low` (merged) | — | 18% → 23%* |

\* From the effort-level experiment's own tracking
(`effort-level-experiment/README.md`), not captured specifically for this
run — night 1 predates this file. Nights 2–7 should have real before/after
numbers.

<!-- Paste each night's /cost output below, or just the two percentages if
that's all you want to keep. -->

**Snapshot 2026-09-07T13:xx UTC — before night 2** (session window resets
3:50pm UTC, ~2h25m out; night 2 is queued to launch right at that reset, so
this snapshot is the "before" for a fresh window, not a mid-window reading):

```
Total cost:            $11.94
Total duration (API):  38m 30s
Total duration (wall): 7h 37m 10s
Total code changes:    618 lines added, 217 lines removed
Usage by model:
     claude-sonnet-5:  9.9k input, 114.4k output, 21.5m cache read, 442.1k cache write ($7.23)
       claude-opus-5:  2.2k input, 35.2k output, 2.7m cache read, 262.6k cache write ($4.71)

Current session
█████████▌                                         19% used
Resets 3:50pm (UTC)

Current week (all models)
█████████████▌                                     27% used
Resets Sep 12, 9am (UTC)
+50% weekly limits promo through Sep 13 · clau.de/cc-50-promo
```

**Snapshot 2026-09-07T13:5x UTC — still before night 2** (delta from above
is the journal-trimming work done in the interim, not night 2 itself — that
still hasn't fired):

```
Total cost:            $12.78
Total duration (API):  40m 53s
Total duration (wall): 7h 45m 9s
Total code changes:    664 lines added, 288 lines removed
Usage by model:
     claude-sonnet-5:  11.0k input, 121.8k output, 25.1m cache read, 453.9k cache write ($8.07)
       claude-opus-5:  2.2k input, 35.2k output, 2.7m cache read, 262.6k cache write ($4.71)

Current session
██████████                                         20% used
Resets 3:49pm (UTC)

Current week (all models)
██████████████                                     28% used
```

**Snapshot 2026-09-08T21:39:50+12:00 - before night 3**
```
Session

Total cost:            $0.0000
Total duration (API):  0s
Total duration (wall): 23s
Total code changes:    0 lines added, 0 lines removed
Usage:                 0 input, 0 output, 0 cache read, 0 cache write

Current session
▌                                                  1% used
Resets 11:30pm (Pacific/Auckland)

Current week (all models)
███████████████▌                                   31% used
Resets Sep 12, 9pm (Pacific/Auckland)
```

**Snapshot 2026-09-08T23:53:45+12:00 - after night 3**
```
Session

Total cost:            $21.95
Total duration (API):  42m 20s
Total duration (wall): 2h 14m 11s
Total code changes:    1062 lines added, 11 lines removed
Usage by model:
    claude-haiku-4-5:  3.5k input, 29 output, 0 cache read, 0 cache write ($0.0037)
       claude-opus-5:  324 input, 189.8k output, 27.6m cache read, 341.8k cache write ($21.95)
Prompt cache (main):   115 requests · 99% of input tokens from cache · no misses · warm (1h TTL, last activity 38m 58s ago)

Current session
                                                   0% used

Current week (all models)
██████████████████                                 36% used
```
**Snapshot 2026-09-11T21:50+12:00 - after night 6**
```
Session                                            99% used  (resets Sat 12 Sep 00:40)
Current week (all models)                          70% used  (resets Sat 12 Sep 21:00)
```
Opened at session 70% / weekly 68% — this run was one of several back-to-back
on the same day, so the session window, not the work, is what ended it. Read
with `~/.claude/skills/usage-check/usage.py`; nights 4 and 5 left no snapshot
here, so the weekly figure spans more than this night.
