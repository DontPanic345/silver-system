# /tdd-cycle close-out — how to improve the cycle

First full run of the skill (Rounds 1–7 of `WATER_SIM_AC.md`, ~19 phase agents).
Compiled from every phase's `## Cycle debug` section plus orchestrator observations.
Rounds 1–6 completed and passed their final gate; Round 7 is halted on a structural
solver decision (separate writeup).

---

## What worked — keep doing it

- **Fresh isolated agents per phase (never forks).** The payoff case is Round 3:
  Red and Green both shipped an inverted buoyancy sign (warm water sank), and the
  AC 6 test passed anyway by luck. Refactor, reading the code *cold* with no memory
  of the sign debate, caught it with two throwaway probes. A fork would have carried
  the same wrong mental model in.
- **Intent statement travelling with the ACs.** The "literal AC wording is a means"
  clause was explicitly invoked to unblock work at least four times: R2 (AC 5
  convergence constants ~10× too short), R3 (buoyancy sign — follow observed grid
  dynamics over the comment), R6 (AC 19 reframed to a boiling plateau), R6
  (`SHIPPED_N` honesty). Without it each of those is a halt-to-user.
- **Persisting every phase report to `scratch/tdd-cycle-log.md`.** Under the tight
  autocompact this was not optional — the log became the *only* continuity artifact
  across three context windows. Worth making a default step in the skill, not an
  ad-hoc thing the user had to ask for.
- **Red stating the assumed API precisely and hard-asserting scenario ids / opts by
  exact string.** When Red's handoff was complete (R6: "handoff unusually complete")
  the round was smooth. When it was thin (R3) the round thrashed. This is the single
  highest-leverage Red deliverable.
- **Committing the sound partial on a halt (R7)** rather than leaving an hour of
  correct work uncommitted in the tree where an autocompact could strand it.

---

## Cycle mechanics — worth fixing in the skills

1. **Red's remit on currently-passing guard tests is undefined.** R1, R2 and R7 all
   flagged uncertainty about whether Red may add green guard tests for cross-cutting
   ACs. R7's AC 26 is an extreme case: its direct-physics assertions are *entirely
   green* at Red time (nothing moves before the forces exist), so the red signal
   rides solely on scenario-existence checks. This apparently works but nobody was
   sure it was allowed. `tdd-red` should say explicitly: green guards for
   cross-cutting invariants are in remit; a round whose physics can't be exercised
   until Green's code exists is allowed to carry its red signal on structural /
   existence assertions.

2. **"Green must run the test file, not just import it"** had to be spelled out by
   R5 Red in the handoff. Make it a hard step in `tdd-green`.

3. **Refactor has no sanctioned path for "the shipped code is wrong."** The skill
   gives it "reconcile comments" and "flag a coverage gap" — but R3 Refactor
   *determined the shipped buoyancy sign was inverted* and had to improvise a
   corrective-cycle recommendation because neither box fit. Add an explicit branch:
   Refactor may report a correctness finding (with evidence) and the orchestrator
   runs a corrective mini-cycle, exactly as happened in R3→R3b.

4. **No guidance on verifying a physics / coordinate convention empirically.** Grid
   orientation (which way is "up") burned the most cumulative time of anything in
   the run — R3 Red traced it through `putImageData`, R3 Green flipped a sign over
   it, R3 Refactor re-derived it with probes. One pinned "conventions" test +
   comment, written once, would have saved all of it. Recommend the skill prompt a
   conventions-doc check at round 1 for any physics/graphics domain.

5. **Test-suite runtime has no owner.** It grew from ~11 s (R2) to ~5 min (R7 — two
   independent 500-step stability runs of the same physics). Nobody's job is to keep
   it fast. Add a soft budget to `tdd-refactor` ("collapse redundant long runs;
   flag if the suite exceeds N seconds").

---

## Recurring slips (same mistake, multiple rounds)

- **`git add -A` staging the untracked `WATER_SIM_AC.md`** — R3b and R4 both did it
  (R3b committed it). Phases should `git add` explicit paths only.
- **Green ships dead code / stale comments under time pressure.** R6: `waterStep`
  advected `air` every step then overwrote the result — dead compute that Refactor
  removed. R6: `conduct()`'s doc comment described both the old Jacobi scheme and
  the new flux-form scheme, contradicting itself. R2, R5 similar. Green doesn't
  audit its own additions once the tests are green; Refactor reliably catches it,
  which is arguably the system working — but a 30-second self-diff in `tdd-green`
  would be cheaper.
- **Fictional constants.** `SHIPPED_N` lived in a test file labelled "the shipped
  grid size" while nothing shipped at that size (R6). `iter: 100` cranked into a
  scenario's opts as a silent workaround for non-conservative conduction (R4) —
  flagged by Refactor, not Green. Pattern: when the honest fix is out of scope,
  Green hides the cost in a knob instead of reporting it.
- **Parameter-tuning guesses that over-force.** R4 Green's first buoyancy guess (4)
  drove the plume the wrong way; R6 Green's first renorm (proportional 3-way) leaked
  57% of the water. Both recovered by falling back to constants the tests already
  proved. `tdd-green` should say: start from the test's own working constants, don't
  re-guess.
- **MacCormack listed as a viable advection option** in an early prompt when it is
  not conservative — cost R1 Green a false start. Prune misleading options from
  handoffs.

---

## AC doc / process observations

- **Hidden round-ordering dependencies.** Some ACs are only honestly testable once a
  *later* round's mechanism exists: R6 AC 19 ("cell ends cooler") needs vapour
  transport that arrives in R7; R7 AC 26 (resting-pool stability) needs forces that
  don't exist when Red runs. The 9-round order looks linear but isn't. Either mark
  these in the AC doc, or let Red write an explicit forward-referencing xfail with a
  note.
- **AC 11 vs the intent statement.** The AC doc mandates per-name scenario
  assertions; the intent statement separately warns against re-asserting the same
  invariant across scenarios. The orchestrator had to re-adjudicate this tension
  every round it came up. Reconcile the two in the doc.
- **The "propose a number, don't loosen silently" rule worked.** R2, R6 and R7 all
  produced defensible revised-or-kept thresholds with evidence rather than either
  contorting the solver or quietly weakening a test. R7 Green refusing to loosen
  AC 26 26× to make the failure disappear is the rule doing its job.

---

## The structural blind spot (Round 7)

Worth stating on its own: the isolated-phase design is excellent at catching
sign/logic errors *in the diff under review*, and blind to **latent bugs in code
nobody is touching**. The checkerboard mode in the colocated pressure projection has
been latent since Round 3's `buoyancyStep`; it surfaced in Round 7 only because that
was the first test to hold a flat interface still for 500 steps. Four rounds of
green suites did not exercise it.

Recommendation: a periodic **adversarial-scenario pass** not tied to any round —
long runs, resting states, sharp interfaces, symmetry tests — run by the orchestrator
between rounds. It would have caught this at Round 3.

---

## Round-by-round outcome

| Round | Result | Notes |
|---|---|---|
| 1 Conservative transport | ✅ | MUSCL flux advection; drift 7–26% → 0.00% |
| 2 Temperature field | ✅ | advect + Jacobi conduct; AC 5 constants revised with evidence |
| 3+3b Buoyancy | ✅ | inverted sign caught by Refactor's cold read; corrective mini-cycle |
| 4 Scenarios as data | ✅ | one definition / two consumers; `iter:100` conduction workaround flagged |
| 5 Scenario player + page | ✅ | DOM-free controller; `f.channels` registry; `npm run shot` repointed |
| 6 Water/vapour/air + phase change | ✅ | 0.00% drift on all conservation proxies; conduction made conservative; AC 19 reframed |
| 7 Liquid falls / gas rises | ⚠️ halted | AC 24/25 green; AC 26 blocked on colocated-projection checkerboard mode — needs a staggered/compact `project()` rewrite as its own slice |
