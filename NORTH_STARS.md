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

A "2D Physics Simulation — Development Handoff" doc: Oxygen Not Included +
Noita inspiration, not reproducing either. Explicit at the time that it was
**a first pass at capturing and exploring the idea, not settled design** — no
specific detail below should be read as a decision just because it's
written down.

What it actually said, beyond the one-line framing above:

- Materials are data-driven (density/viscosity/heat capacity/conductivity/
  phase/colour...), never bespoke code per material — the one piece of this
  that made it furthest unchanged; see `src/material.rs`.
- **The phased content order — physics, then chemistry, then biology, then
  a game/interaction layer** (in the doc's own words: "GPU grid prototype →
  falling sand → liquids → multi-material → temperature → pressure →
  reactions → game layer"). This ordering is *this doc's content*, not
  night-shift's invention — night-shift's tranches 1–4 (Physics / Chemistry
  / Biology / "the glass pane") are this same order wearing night-shift's
  own process vocabulary. The tranche/milestone/round/phase *mechanism*
  that ran them was the experiment, and that's what got shelved; the
  ordering itself is vision content and survives that shelving.
- GPU compute as *foundational* architecture, not a later optimisation:
  structure-of-arrays buffers kept on the GPU, double buffering, fixed
  timestep, determinism as an explicit goal, cross-vendor (no CUDA
  assumptions). Rust floated as a natural core language.
- Benchmarking as a first-class feature; conservation-law tests (mass,
  energy, determinism, boundaries) — this repo's headless-verification
  habit and `src/grid.rs`'s reference-grid timing test are downstream of
  this, even though the GPU part of it was dropped (below).

**Original context:** reconstructed by ChatGPT from a lost transcript, then
handed to Claude Code on 2026-09-02. No file of it was ever committed to
this repo — the fullest surviving record is
`~/.claude/projects/-home-fallo-silver-system/memory/sim-vision-doc.md`, a
Claude Code memory file local to one machine, not tracked by this repo. This
entry is now the more durable copy; if the two ever disagree, trust this
one and update that memory file to match, not the other way around.

**Status:** the emergent-behaviour framing, materials-as-data, Rust, and the
physics→chemistry→biology→game-layer ordering were all carried forward.
Two pieces were explicitly walked back rather than carried forward: GPU
compute as *foundational* architecture (not a later optimisation) and
determinism as a standing requirement. Both are now treated as
architecture-contingent decisions to make later, if a real need arises —
see `src/math.rs`'s `Scalar` doc comment for where that reasoning currently
lives in code.

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
