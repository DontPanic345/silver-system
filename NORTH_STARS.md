# North Stars

Vague, aspirational statements of what this project is *for* — dictation
dumps, capstone visions, the long version someone typed out once and meant.
Each one below captures a sliver of the same underlying motivation, in
whatever words it was stated in at the time. None of these is a tactical
goal, and none of them "completes" or gets abandoned the way an experiment
does — see `JOURNAL.md` for the dated history of what got tried, and
`PRINCIPLES.md` for the aphorisms distilled along the way. A north star can
be narrowed, restated more sharply, or have a piece of it explicitly walked
back (noted inline below when that happened) — but it isn't retired just
because the experiment that was chasing it stopped.

Read this before writing a new one. If what you're about to write already
captures the same sliver as something below, restate it there instead of
starting a new entry.

---

## 1. The long-term interest (2026-09-02)

> A small world that is part of a much larger universe — emergent physical
> behaviour from simple interacting rules (fluids, materials, temperature,
> pressure, reactions), in the spirit of Oxygen Not Included and Noita.

**Original context:** first stated at the repo root the day the JS terrarium
was shelved, framing what came next — commit `7aaa69b` ("Shelve the JS
terrarium under `terrarium/`; reset root for the next experiment"),
`README.md` at that commit. Still active — restated in the current
`README.md` and (more tersely) `CLAUDE.md` unchanged since.

## 2. The GPU-native vision doc (2026-09-02)

> Don't script interesting outcomes — build simple interacting physical
> systems (materials, fluids, temperature, pressure, reactions) and let
> behaviour emerge.

**Original context:** a reconstructed development-handoff doc (Oxygen Not
Included + Noita inspiration), explicit at the time that it was a first pass
at capturing an idea, not settled design. No separate file survives — this
paraphrase, written when this entry was first added, is the only remaining
record of it.

**Status:** partially adopted into night-shift's north star below (the
emergent-behaviour framing, Rust, the tranche ordering); two pieces of it
were explicitly walked back rather than carried forward: GPU compute as
*foundational* architecture (not a later optimisation) and determinism as a
standing requirement. Both are now treated as architecture-contingent
decisions to make later, if a real need arises — see `src/math.rs`'s
`Scalar` doc comment for where that reasoning currently lives in code.

## 3. A believable small world inside a large universe (2026-09-05)

> **A believable small world inside a large universe.**
>
> **A terrarium people can see on their screens and interact with.**

Two statements of the same goal, stated as both a standard (believability
comes from the universe underneath being genuinely full — see
`PRINCIPLES.md`'s first entry, which is this standard restated as a rule of
thumb) and a deliverable (a thing on a screen a person opens, watches,
reaches into, and comes back to).

**Original context:** `night-shift`'s `CLAUDE.md` and `PLAN.md` (long
version), commit `d83f3b0` ("Flesh out the plan; state the north star; tell
planners to reach"). `night-shift/PLAN.md` and
`night-shift/skills/cycle-contract/SKILL.md` still carry it verbatim, kept
for reference under the shelved experiment.

**Status:** the process built to chase this (`night-shift`'s
tranche/milestone/round/phase cycle) was shelved — see `JOURNAL.md` — but
this is a sharper restatement of #1, not a competing or abandoned one. Still
live.
