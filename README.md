# Falling Sand

A falling-sand simulation that runs entirely in the browser — a cellular
automaton where every cell on a grid is not a single material but a
**mixture**: fractional amounts of sand, clay, biomass and water, plus a
gas headspace of N&#8322;, O&#8322;, CO&#8322; and smoke. There's no dedicated "soil"
material — pour water over a sand/clay/biomass pile and the composition
blends together over time into something that looks and behaves like soil,
purely as an emergent result of diffusion between neighboring cells.

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
| Wood | Static, but flammable — catches fire from an adjacent burning cell. |
| Fire | A tool, not a substance — ignites Wood or Biomass-bearing cells under the brush. Fire itself is just a "burning" flag on flammable matter. |
| Eraser | Clears cells back to open air (real atmosphere, not nothingness). |

## Controls

- **Click / touch and drag** on the canvas to paint the selected material.
- **Brush size** slider controls how wide each stroke is.
- **Pause / Resume** freezes the simulation (also toggled with `Space`).
- **Clear** wipes the grid back to open air.

## How it works

- **Composition, not identity.** Every cell holds eight fractions —
  `sand, clay, biomass, water, N2, O2, CO2, smoke` — that always sum to 1.
  A cell is never "empty"; an unpainted cell is just full of atmosphere
  (78% N2 / 21% O2 / 1% CO2, matching real air). Stone and wood are the only
  pure, immovable materials, stored separately since they don't mix.
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
- **Rendering** writes directly into a `Canvas ImageData` buffer sized to the
  grid (one pixel per cell): solids render their base color; matter-bearing
  cells render a weighted blend of sand/clay/biomass colors, with water's
  contribution shading from a vivid open-water blue toward a dark, muted
  "soaked-in" tint as more solid grit shares the cell (so wet soil reads as
  darker earth, not blue-gray sludge); mostly-air cells render as background
  tinted by their smoke/CO2 content. The browser upscales this with
  `image-rendering: pixelated` for the chunky, classic falling-sand look —
  this keeps the simulation fast even at a few hundred thousand cells.
