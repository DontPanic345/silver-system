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

Entries here are the distillation, not the source. Raw material (voice
dictations, reflections) is archived verbatim under `dictation-dumps/` —
nobody needs to read those to work in this repo; they exist for provenance
and for the open task noted in `dictation-dumps/README.md` (revisiting all
of it for ideas that never made it into a distillation the first time).

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
physics→chemistry→biology→game-layer ordering were all carried forward. GPU
compute as *foundational* architecture and determinism as a standing
requirement were not — three days later, in the dump behind entry #3 below,
the same author had gone "iffy about the deterministic requirement... I
think we will [move to the GPU but] I'm really worried these agents are
going [to try] to get determinism when we don't actually need it —
'architecture contingent.'" That's the walk-back, in its original words —
see `dictation-dumps/agentic-development.md`. Both are now treated as
decisions to make later if a real need arises, not standing requirements —
see `src/math.rs`'s `Scalar` doc comment for where that reasoning currently
lives in code.

## 3. A believable small world inside a large universe (2026-09-05)

> Only when the big universe is sufficiently full will the small world be
> believable.

> **A believable small world inside a large universe. A terrarium people
> can see on their screens and interact with.**

The same dump also restates #1's domain ordering with real detail: pressure
and fluid dynamics in physics (a resting pool stays flat, a column of water
finds its level, a U-shaped pipe brings water to a level across both arms);
state changes, burning, iron/carbon → steel by more than one route, and the
full water cycle in chemistry; fungus, humus, bacteria, slime mould, and the
carbon/nitrogen cycles in biology; then "the glass pane" — human interaction
with the world, sandbox and terrarium both.

**Original context:** dictated by the user,
`dictation-dumps/agentic-development.md` (2026-09-05) — mixed in,
undifferentiated, with a full specification of the `night-shift`
tranche/round/Red-Green-Refactor cycle (see `dictation-dumps/README.md`).
Landed in committed form as `night-shift`'s `CLAUDE.md`/`PLAN.md`, commit
`d83f3b0`.

**Status:** the process built to chase this (`night-shift`'s
tranche/milestone/round/phase cycle) was shelved — see `JOURNAL.md` — but
this is a sharper restatement of #1, not a competing or abandoned one. Still
live.
