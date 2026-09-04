# silver-system

Building a small world that is part of a much larger universe — emergent physical
behaviour from simple interacting rules (fluids, materials, temperature, pressure,
reactions), in the spirit of Oxygen Not Included and Noita.

The work plan is [`PLAN.md`](PLAN.md): mathematics and tooling foundations, then
physics, chemistry, biology, and the glass pane. The language is Rust.

## How the work runs

`tranche → milestone → round → phase`. A round is one Red → Green → Refactor pass;
a phase is one agent's turn. The cycle is defined by the skills in
[`.claude/skills/`](.claude/skills/), starting with `cycle-contract`. `cycle-tranche`
is the top of it — it runs indefinitely, chaining from tranche to tranche once
`PLAN.md`'s are done. Round logs live under `cycle-log/`.

## Shelved experiments

Kept for reference, not extended.

- [`terrarium/`](terrarium/) — a dependency-free browser falling-sand sim that grew
  a sealed glass jar and a closed water cycle. A successful test.
- [`stable-fluids/`](stable-fluids/) — a browser Stam stable-fluids sim with
  conservative advection, temperature and buoyancy, built test-first over seven
  rounds. Halted on a checkerboard mode in the colocated pressure projection. Its
  retrospective is why the current cycle skills look the way they do.
