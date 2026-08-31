# Falling Sand

A small falling-sand simulation that runs entirely in the browser — a
cellular automaton where each cell on a grid is a material (sand, water,
stone, wood, fire, smoke) that follows simple movement rules every tick.

No build step, no dependencies. Just open `index.html`, or serve the
folder with any static file server:

```bash
python3 -m http.server 8000
# then visit http://localhost:8000
```

## Materials

| Material | Behavior |
| --- | --- |
| Sand | Falls straight down; slides diagonally when blocked; sinks through water. |
| Water | Falls down, then spreads diagonally and sideways to find its level. |
| Stone | Static — a solid wall/platform other materials rest on. |
| Wood | Static, but flammable — catches fire from an adjacent burning cell. |
| Fire | Flickers and drifts upward, ignites nearby wood, is doused by water, and burns out into smoke. |
| Eraser | Clears cells back to empty. |

## Controls

- **Click / touch and drag** on the canvas to paint the selected material.
- **Brush size** slider controls how wide each stroke is.
- **Pause / Resume** freezes the simulation (also toggled with `Space`).
- **Clear** wipes the grid.

## How it works

- `js/main.js` holds the whole engine: a flat `Uint8Array` grid of material
  ids, plus parallel arrays for per-cell color jitter and life/fuel
  countdowns (used by fire and smoke).
- Each frame scans the grid bottom-to-top so a particle can't fall through
  a cell it just vacated in the same tick, and alternates left-to-right /
  right-to-left per row per frame to avoid a directional bias in how sand
  piles or water spreads.
- Denser materials sink through lighter ones (sand through water) via a
  simple density comparison on swap.
- Rendering writes directly into a `Canvas ImageData` buffer sized to the
  simulation grid (one pixel per cell) and lets the browser upscale it
  with `image-rendering: pixelated` for the chunky, classic falling-sand
  look — this keeps the simulation fast even at a few hundred thousand
  cells.
