//! Random numbers for the choices the game makes at random when it creates soldiers

use crate::{map::GridNo, random::Random};

/// The generator of the placement with the given index in the map file and starting tile, so
/// that the choices are the same every time
pub fn placement_random(index: usize, grid_no: GridNo) -> Random {
    Random::new(index as u64 ^ (grid_no.index() as u64) << 16)
}
