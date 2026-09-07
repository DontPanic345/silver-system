# Nightly runs

A standing brief for unattended, one-shot build sessions. One run per night,
each starting cold from the repo's committed state, each leaving the next one
something to build on.

The shape of this brief is deliberate, and derived from runs recorded in
[`effort-level-experiment/README.md`](effort-level-experiment/README.md).
Three findings from that experiment shape it:

- **Bounded ambition beats both extremes.** A heavily prescribed process
  (`night-shift`) died of its own ceremony. A ceiling-free prompt — "build
  the vision the best way you see fit" — made the agent *more* conservative,
  not less: it read this repo's history of overreach and used it as a reason
  to stay small, shipping less than the control prompt did. The phrasing that
  worked, twice, on two different models, was ambition bounded by confidence.
- **The path is better chosen by whoever has read the code.** The one run
  that picked its own target picked a better one than any plan written in
  advance had.
- **More effort buys volume, not correctness.** The single confirmed false
  claim in the whole experiment came from one of the *highest* effort
  settings, with a fully green test suite behind it.

So: the stance and the invariants are fixed. The target is not.

## Operating rules

- **One run per night, each in its own fresh 5-hour usage window.** Runs
  packed into a single window starve each other — that is what stalled the
  `high` run and made its numbers unusable.
- **Size the work to finish inside one context window.** If it doesn't fit,
  it's too big for one night: commit a checkpoint and leave the rest for
  tomorrow, rather than letting the session compact itself mid-task and
  continue from a lossy summary.
- **Don't course-correct mid-week.** Read the week's journal entries together
  and steer once. Every experiment in this project's history was reset rather
  than extended; the thing being tested now is whether work compounds when
  it's left alone to.

## The prompt

Swap nothing. Paste as-is.

```
Read JOURNAL.md, NORTH_STARS.md and PRINCIPLES.md first. They carry the
history, the aspiration, and the aphorisms distilled along the way.
JOURNAL.md's most recent entries are the previous night's work — you are
continuing that line, not starting a new one.

Choose tonight's piece yourself. Pick whatever most moves this project
toward NORTH_STARS.md, and open your journal entry by saying what you chose
and why you chose it over the alternatives. You have read the code; you are
better placed to judge what comes next than a plan written in advance.

Build as much of it as you can CONFIDENTLY pull off in this session — don't
default to something small or safe, and don't stop at the first thing that
compiles. If you finish with room to spare, keep going into whatever that
piece unblocks rather than polishing what you already have.

Build on what is in src/ already. Extend it, refactor it where it's in your
way, but don't restart it — the point of tonight is that tomorrow starts
from where you stopped.

Invariants, which are not yours to trade away:

- Conserve by construction. Mass and energy may not appear or vanish except
  through a declared, accounted-for hole — see src/gnome.rs for how magic is
  already ledgered.
- Materials stay data-driven. No per-material special-case code.
- Everything must be verifiable headlessly, in numbers rather than
  screenshots. Anything with a visual surface also needs a real-browser e2e
  check that reads actual canvas pixels after real wall-clock time.
- NORTH_STARS.md is read-only. If you think it's wrong, argue with it in
  your journal entry — do not edit it.

On verification, be strict: passing tests is not the same as working. An
earlier run in this repo shipped a water-levelling fix with a green suite,
clean clippy, and three passing e2e tests — and the water still visibly
oscillated when a human looked at it. Before claiming something works, ask
what a human would see, and go and check that.

Finish by appending a dated JOURNAL.md entry: what you chose and why, what
you built, what you learned that is worth keeping (findings, not narration),
and what you deliberately left undone. State plainly what you verified and
what you did NOT — an honest "I did not check X" is worth more than a
confident claim that has to be disproved later. Then commit your work.

The next session starts cold and has only what you wrote down.
```

## Why the closing line matters

Each night's session is discarded when it ends. The only things that survive
are the commit and the journal entry — so the journal entry is not
bookkeeping, it is the entire handoff. A run that builds well and writes
nothing down has produced a substrate the next run cannot reason about, and
the next run will be tempted to do what every experiment in this repo's
history has done, and start over.
