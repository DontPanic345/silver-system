# A still in the jar — what it needs, and what stopped it

Night 9 tried to put a working still inside the flagship terrarium, measured
it, and took it back out. This is the handover: what was built, what the
numbers said, and the one thing that actually blocks it. `NORTH_STARS.md` #4
calls brewing and distilling "core to the whole thing working, not flavour",
so this is not a dead end, it is a half-finished bridge with the gap marked.

## What works already

- **`/still.html` works** and has since night 3: juniper in water on a 366 K
  hob, a gap in a wall, a cold receiver, and gin out of the far end. Nothing
  in it is a brewing mechanic.
- **The jar has both temperatures it needs**, declared and ledgered: the
  spring at 365 K (above ethanol's 351.5 K boiling point, below water's
  373.15 K) and the roof at 283 K.
- **The colony can now be told to fetch things** (`Job::Supply`, night 9),
  and a standing order re-arms itself on a cadence — which is what keeps a
  pot charged without a player tapping a cell all night.
- **A gnome can now stand next to something hot.** Until night 9 the air
  beside a 365 K pot read as lethal, so no route would go there and no supply
  order beside it was reachable. `gnome::felt_temperature` scales a cell's
  heat by what it can actually deliver, so a workshop is possible.

## What was built and measured

A hearth of the spring's own stone on the meadow's east end, a pot of water
on it, a stone hood running west over the walkway, and a "worm" — one cell
held at the roof's 283 K — for the vapour to condense against. One standing
`Supply { juniper }` order was the whole operating manual.

It ran. In order, the things that went wrong, each measured:

1. **Filling the hearth hot is not the same as holding it hot.** Without a
   thermostat the stone was back to ambient in 400 steps. With one it settles
   at 346 K, not 365: it is losing heat to sand and air on five faces.
2. **The wash does not need to boil.** At 340 K its vapour pressure is
   already about half an atmosphere (`src/vapour.rs`), so a charge evaporates
   away in about a thousand steps. Distilling in this jar is evaporation, not
   boiling — which also means the wash needs a *free surface*.
3. **Without a condenser you get nothing you can drink.** 0.098 g of spirit
   ended up spread through the whole jar's air at a twentieth of saturation,
   and 1 mg of gin. A condenser is not a detail of a still, it is half of it.
4. **A pot that must empty cannot keep standing water beside it.** This is
   the blocker. Mashing needs the botanical to touch water, so the pot needs
   a permanent water cell adjacent; but liquids level, so the moment the
   charge cell empties the water flows or swaps into it — and then the supply
   order finds the cell occupied for ever. Put the water *below* instead and
   the wash (denser) sinks under it, at which point it has no free surface
   and stops evaporating. Measured both ways: two charges in 12 000 steps,
   then a deadlock.

## The two ways out

- **Liquid cells that hold mixtures.** Gas cells have held mixtures since
  night 4; liquids still hold one material each. A pot of water with extract
  dissolved in it would be *one cell*, the whole problem above would not
  exist, and it is the same change that would finally make wash a real
  water–ethanol solution rather than a pure substance that boils over
  wholesale (listed as untouched since night 4). This is the real answer and
  it is a night's work on its own.
- **Or: let a supply order displace a liquid.** `order::place` only puts a
  load into a gas cell. If it could swap a liquid aside into a neighbouring
  gas cell — which conserves exactly, being a swap — then "drop a bush into
  the pot" would work literally, and the pot could be a cell of the pool.
  Cheaper, and it leaves the wash-as-solution problem alone.

## The other thing in the way

**There is no water in the jar a gnome can reach**, and no way to the pool at
all: it stands behind a bank five courses tall and a gnome climbs one course
at a time, while the garden's bed is roofed by its own bushes. A fetch order
for water is an order nobody can fill. Whatever shape the still takes, the
scenario needs a way down to the water — a jetty, a stair, or a dug channel
the colony makes itself.

## The economics, for whoever picks this up

Re-anchored on night 9 so that brewing is worth doing and cannot print money:

- A gram of juniper eaten off the bush is 600 Gin.
- Mashing turns the *bush* into wash and leaves the water alone, so a gram of
  bush makes a gram of wash makes a gram of gin.
- A gram of gin is 1000 Gin, because ethanol carries about 1.75 times the
  energy of dry plant matter (29.7 kJ/g against 17).
- Heat magic costs 1/30 Gin per joule, so distilling a gram with a Temper
  spell costs about 33 Gin against the 1000 it yields — worth doing, and
  worth doing on the spring rather than on magic.

So a still is worth roughly three quarters more than eating the crop raw, and
no more. It concentrates what the garden grew; it does not multiply it.
