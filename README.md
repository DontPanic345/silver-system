# Falling Sand

A falling-sand simulation that runs entirely in the browser — a cellular
automaton where every cell on a grid is not a single material but a
**mixture**: fractional amounts of sand, clay, biomass and water, plus a
gas headspace of N&#8322;, O&#8322;, CO&#8322;, smoke and water vapour. There's no
dedicated "soil" material — pour water over a sand/clay/biomass pile and the
composition blends together over time into something that looks and behaves
like soil, purely as an emergent result of diffusion between neighboring
cells.

It's slowly growing into a **terrarium**: build a sealed glass jar, and a
day/night light cycle drives a closed water cycle inside it — the pool
evaporates in the daytime warmth, the vapour rises and condenses as dew on
the cool glass, and the dew runs back down into the pool. Nothing is added
or lost once the lid is sealed. See [`TERRARIUM_PLAN.md`](TERRARIUM_PLAN.md)
for where this is headed (plants next).

No build step, no dependencies. Just open `index.html`, or serve the
folder with any static file server:

```bash
python3 -m http.server 8000
# then visit http://localhost:8000
```

## Materials

| Material | Behavior |
| --- | --- |
| Sand | Dense, dry granular solid. Falls straight down, slides diagonally when blocked. |
| Clay | Denser than sand — sinks through both sand and water. |
| Biomass | Light organic matter — floats on water, mixes into soil, burns readily. |
| Water | Falls and spreads to find its level; soaks into adjacent solids over time. |
| Soil | A convenience preset that paints a pre-mixed sand/clay/biomass/water blend — the same composition sand+clay+biomass+water settle into on their own. |
| Stone | Static and immovable — a solid wall/platform other materials rest on. |
| Glass | Static and immovable like stone, but drawn translucent — and cool enough that water vapour condenses on its inner surface. |
| Wood | Static, but flammable — catches fire from an adjacent burning cell. |
| Fire | A tool, not a substance — ignites Wood or Biomass-bearing cells under the brush. Fire itself is just a "burning" flag on flammable matter. |
| Eraser | Clears cells back to open air (real atmosphere, not nothingness). |

## Controls

- **Click / touch and drag** on the canvas to paint the selected material.
- **Brush size** slider controls how wide each stroke is.
- **Pause / Resume** freezes the simulation (also toggled with `Space`).
- **Clear** wipes the grid back to open air.
- **Build Jar** stamps a glass enclosure with an open neck; **Seal Lid**
  closes the neck for a fully closed system.
- **Day length** sets how many ticks a full day/night cycle takes. The
  header shows the current light level and jar humidity.

## Tests

No framework — one headless script that stubs the DOM, drives `step()`, and
reads the composition arrays back:

```bash
node test/sim-invariants.js
```

It checks mass conservation (including the liquid ↔ vapour water cycle),
that water finds its level without strobing, that dry powder still piles,
and a rough per-step performance budget.

## How it works

- **Composition, not identity.** Every cell holds nine fractions —
  `sand, clay, biomass, water, N2, O2, CO2, smoke, vapour` — that always sum
  to 1. A cell is never "empty"; an unpainted cell is just full of atmosphere
  (78% N2 / 21% O2 / 1% CO2, matching real air). Stone, glass and wood are
  the only pure, immovable materials, stored separately since they don't mix.
- **One movement rule for everything.** Each cell's effective density is the
  weighted sum of its components' densities (clay > sand > water > biomass >
  air > smoke, with smoke *negatively* dense so it's buoyant). Every non-solid
  cell tries to swap into a less-dense neighbor below it, and — if it's
  itself lighter than what's above it — tries to rise, which is what makes
  smoke climb through air and gas bubbles rise through water, using the same
  code path that makes sand sink through water. Water-heavy cells also spread
  sideways; soggy (but not fully liquid) cells move less often, giving wet
  soil a thicker, more viscous feel than dry sand.
- **Diffusion is what creates soil.** After movement, every adjacent pair of
  cells exchanges a small fraction of their composition. Between two
  "ground" cells this blends sand/clay/biomass/water directly — this is the
  entire mechanism behind soil forming wherever those materials sit next to
  each other. Between a matter-heavy cell and open air, only the *ratio* of
  gases in each side's headspace mixes, never the absolute amount — a packed
  cell has no room for air and can't dissolve into its neighbor just because
  it borders one.
- **Combustion.** A burning Wood or Biomass-bearing cell consumes its own
  fuel each tick, converting it into CO2 and smoke released into that same
  cell's gas fraction (or a neighbor's, for wood), and can ignite nearby
  flammable cells unless they're wet. Every gas exchange in this pass is
  mass-neutral — nothing is created or destroyed, just converted or moved.
- **The water cycle** is the same discipline applied to a loop. A global
  clock sets `lightLevel` (0 at midnight, 1 at noon). Each tick, a liquid
  water surface hands a little water to the air above it as *vapour* and
  takes an equal volume of that air back — scaled by the day's heat, and
  tapering to nothing as the local air saturates, so a sealed jar settles at
  a steady humidity instead of drying out. Vapour is buoyant, so it rises on
  the ordinary movement rule; where it meets the cool glass (or gets
  supersaturated in mid-air) it condenses back to liquid, which then runs
  down and pools on the ordinary gravity rule. Every transfer moves an exact
  amount between the liquid and vapour channels, so `Σ water + Σ vapour` over
  the whole grid is invariant — that's the regression test for the feature.
- **Persistence.** The whole grid (every fraction quantised to a byte, plus
  the clock and jar state) is snapshotted to `localStorage` every few seconds
  and on tab-hide, and restored on load — so a jar you've been growing
  survives a refresh. `Clear` wipes both the grid and the snapshot.
- **Rendering** writes directly into a `Canvas ImageData` buffer sized to the
  grid (one pixel per cell): solids render their base color; matter-bearing
  cells render a weighted blend of sand/clay/biomass colors, with water's
  contribution shading from a vivid open-water blue toward a dark, muted
  "soaked-in" tint as more solid grit shares the cell (so wet soil reads as
  darker earth, not blue-gray sludge); mostly-air cells render as background
  tinted by their smoke/CO2 content. The browser upscales this with
  `image-rendering: pixelated` for the chunky, classic falling-sand look —
  this keeps the simulation fast even at a few hundred thousand cells.
