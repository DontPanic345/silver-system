//! Light: how far down into the world the sky reaches, and the day that
//! turns it on and off.
//!
//! Two separate things live here, and keeping them separate is the point.
//!
//! - [`illuminate`] computes *visibility*: how much of the sky each cell can
//!   see, marched straight down each column and attenuated by whatever is in
//!   the way ([`Material::opacity`]). It is free, it changes nothing, and
//!   every world gets it every step. `src/life.rs` reads it to decide where
//!   a plant can grow, which is what makes shade a thing worth caring about:
//!   a bush under a stone shelf is in the dark, and a bush under another
//!   bush is in the gloom.
//! - [`Sun`] delivers the *joules*, and because joules entering the world
//!   are a hole in its books, every one of them goes through
//!   [`World::conjure_energy`] and lands in the ledger — the same mechanism
//!   `src/terrarium.rs`'s [`Thermostat`](crate::terrarium::Thermostat) has
//!   used since night 1. A jar on a windowsill is not a closed system, and
//!   this says by exactly how much, in joules, per day.
//!
//! ## Why a terrarium wanted a day
//!
//! Night 4 left the water cycle running but *invisible*: a milligram of
//! condensation a step, no fog, nothing to watch. The reason is that a jar
//! held at one temperature by two thermostats has no weather. A day gives it
//! one. The pool warms and gives up vapour while the sun is on it; at night
//! the jar cools, the air passes saturation everywhere at once, and the
//! water comes back as mist and rain. That is a cycle with a beat, and it is
//! also, not incidentally, the only reason a plant's night is different from
//! its day.
//!
//! The very first experiment in this repo — the JS terrarium shelved in
//! September — had a light cycle and lost it in the rewrite. This is it
//! back, with a ledger under it.
//!
//! [`Material::opacity`]: crate::material::Material::opacity

use crate::math::Scalar;
use crate::world::World;

/// Recomputes `world`'s light field from its current [`World::sky`].
///
/// One downward march per column: a cell sees whatever reached the cell
/// above it, and passes on what it does not absorb. A cell's absorption is
/// its material's [`Material::opacity`] scaled by how *full* it is, so a
/// cell holding a raindrop's worth of water is nearly clear and a full one
/// is not — the same fill fraction `src/gnome.rs` uses to decide whether a
/// gnome is standing in a puddle or a pond.
///
/// [`Material::opacity`]: crate::material::Material::opacity
pub fn illuminate(world: &mut World) {
    let (w, h) = (world.width(), world.height());
    let sky = world.sky.clamp(0.0, 1.0);
    for i in 0..w {
        let mut carry = sky;
        for j in (0..h).rev() {
            let p = j * w + i;
            world.light[p] = carry;
            carry *= 1.0 - absorption(world, p);
        }
    }
}

/// The fraction of the light passing through cell `p` that it absorbs.
fn absorption(world: &World, p: usize) -> Scalar {
    let material = world.material_of(p);
    if material.opacity <= 0.0 {
        return 0.0;
    }
    let fill = if material.density > 0.0 {
        (world.cell_at(p).mass / material.density).clamp(0.0, 1.0)
    } else {
        1.0
    };
    material.opacity * fill
}

/// The sky over a scenario: a day/night cycle, and the energy the daylight
/// half of it carries into the world.
///
/// `power` is joules per second delivered to one fully-absorbing cell in
/// full sun. It is a tuned number in the same sense this crate's
/// conductivities are: the real figure depends on a cell's area, which this
/// simulation has never pinned to metres.
#[derive(Debug, Clone, Copy)]
pub struct Sun {
    pub power: Scalar,
    /// Steps in one whole day-and-night.
    pub day_steps: u64,
    /// Where in the day the world starts, 0 to 1 — `0.25` is dawn, since
    /// the intensity curve below is a sine that peaks at noon.
    pub phase: Scalar,
}

impl Sun {
    pub fn new(power: Scalar, day_steps: u64) -> Self {
        Sun {
            power,
            day_steps: day_steps.max(1),
            phase: 0.0,
        }
    }

    /// Starts the world part-way through its day, builder-style.
    pub fn starting_at(mut self, phase: Scalar) -> Self {
        self.phase = phase;
        self
    }

    /// How bright the sky is at `step`, 0 to 1: a sine peaking at midday and
    /// clamped flat at zero through the night, which is what a sky over a
    /// point on a rotating planet actually does.
    pub fn intensity(&self, step: u64) -> Scalar {
        let turn = (step % self.day_steps) as Scalar / self.day_steps as Scalar + self.phase;
        let s = (turn * std::f64::consts::TAU as Scalar).sin();
        // Snapped to zero rather than merely clamped: `sin(π)` is 1e-16, not
        // 0, and a sky that is 1e-16 bright still runs the whole
        // illuminate-and-deliver pass and books femtojoules of midnight
        // sunshine into the ledger.
        if s < 1e-6 {
            0.0
        } else {
            s
        }
    }

    /// Sets `world.sky` for this step, re-marches the light field, and pays
    /// the world the joules that light carries — all of it ledgered.
    ///
    /// Energy lands where light is *absorbed*, not where it falls: the air
    /// is transparent, so a sunbeam warms the pool and the rock it reaches
    /// and passes through everything on the way. That is why the jar heats
    /// from its floor up rather than uniformly, and therefore why it
    /// convects at all.
    pub fn shine(&self, world: &mut World, step: u64, dt: Scalar) {
        world.sky = self.intensity(step);
        illuminate(world);
        if world.sky <= 0.0 || self.power <= 0.0 {
            return;
        }
        let n = world.width() * world.height();
        for p in 0..n {
            let absorbed = absorption(world, p);
            if absorbed <= 0.0 {
                continue;
            }
            let joules = world.light_at(p) * absorbed * self.power * dt;
            if joules > 0.0 {
                world.conjure_energy_at(p, joules as f64);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{terrarium as t, MaterialTable};
    use crate::math::GridIndex;

    fn world() -> World {
        World::new_open(8, 10, MaterialTable::terrarium(), 291.0)
    }

    #[test]
    fn open_air_is_transparent_all_the_way_down() {
        let mut w = world();
        illuminate(&mut w);
        for j in 0..10 {
            let p = w.linear_index(GridIndex::new(3, j));
            assert!(
                (w.light_at(p) - 1.0).abs() < 1e-9,
                "air should not shade: {} at row {j}",
                w.light_at(p)
            );
        }
    }

    #[test]
    fn a_stone_shelf_puts_what_is_under_it_in_the_dark() {
        let mut w = world();
        w.fill(GridIndex::new(3, 6), t::STONE, 291.0);
        illuminate(&mut w);
        let above = w.linear_index(GridIndex::new(3, 7));
        let below = w.linear_index(GridIndex::new(3, 5));
        assert!(w.light_at(above) > 0.9);
        assert!(w.light_at(below) < 1e-6, "{}", w.light_at(below));
        // ...and the column beside it is untouched.
        let beside = w.linear_index(GridIndex::new(4, 5));
        assert!(w.light_at(beside) > 0.9);
    }

    /// A bush shades, but not absolutely: something can still live under it.
    #[test]
    fn a_bush_casts_shade_rather_than_a_shadow() {
        let mut w = world();
        w.fill(GridIndex::new(2, 6), t::JUNIPER, 291.0);
        illuminate(&mut w);
        let under = w.linear_index(GridIndex::new(2, 5));
        let light = w.light_at(under);
        assert!(
            light > 0.0 && light < 0.5,
            "under a bush should be gloomy, not black: {light}"
        );
    }

    /// A shallow puddle barely shades; a full cell of water shades more.
    #[test]
    fn shading_scales_with_how_full_a_cell_is() {
        let mut deep = world();
        deep.fill(GridIndex::new(1, 6), t::WATER, 291.0);
        illuminate(&mut deep);
        let mut shallow = world();
        let mut drop = crate::world::Cell::full(shallow.materials(), t::WATER, 291.0);
        drop.mass = 0.05;
        shallow.set_cell(GridIndex::new(1, 6), drop);
        illuminate(&mut shallow);
        let p = deep.linear_index(GridIndex::new(1, 5));
        assert!(
            shallow.light_at(p) > deep.light_at(p) + 0.1,
            "a drop shaded as much as a pool: {} vs {}",
            shallow.light_at(p),
            deep.light_at(p)
        );
    }

    #[test]
    fn the_sun_rises_and_sets() {
        let sun = Sun::new(1.0, 100);
        assert!(sun.intensity(0) < 1e-9, "the day starts at dawn");
        assert!(sun.intensity(25) > 0.99, "and peaks at midday");
        assert!(sun.intensity(50) < 1e-9);
        assert!(sun.intensity(75) < 1e-9, "and is dark all night");
    }

    #[test]
    fn every_joule_of_sunlight_is_on_the_ledger() {
        let mut w = world();
        for i in 0..8 {
            w.fill(GridIndex::new(i, 0), t::STONE, 291.0);
        }
        w.rebaseline();
        let sun = Sun::new(2.0, 100);
        for step in 0..40 {
            sun.shine(&mut w, step, 0.05);
        }
        assert!(
            w.ledger().energy_conjured > 0.0,
            "the sun should have delivered something"
        );
        let r = w.conservation_residuals();
        assert!(
            r.energy_relative.abs() < 1e-9,
            "sunlight must be booked, not smuggled: {:e}",
            r.energy_relative
        );
        assert!(
            w.cell(GridIndex::new(4, 0)).temperature > 291.0,
            "the floor should have warmed"
        );
    }

    #[test]
    fn nothing_is_delivered_at_night() {
        let mut w = world();
        for i in 0..8 {
            w.fill(GridIndex::new(i, 0), t::STONE, 291.0);
        }
        w.rebaseline();
        let sun = Sun::new(2.0, 100);
        for step in 50..75 {
            sun.shine(&mut w, step, 0.05);
        }
        assert_eq!(w.ledger().energy_conjured, 0.0);
        assert_eq!(w.sky, 0.0);
    }
}
