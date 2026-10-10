//! The regions that the map is baked in, see `systems/bake.rs`
//!
//! The area that the sprites of a map can cover is split into equally sized regions. Each region
//! has its own draws and bins (`ReplayScene`), so an edit only rebuilds and bakes the regions it
//! touches.

use bevy::prelude::*;

use crate::components::map::{JA2_MAP_SIZE, JA2_TILE_HEIGHT, JA2_TILE_WIDTH};

/// Maximum size of a region texture in texels. Small regions keep the work of an edit small, the
/// texture memory is the same for any size.
pub const MAX_REGION_SIZE: u32 = 512;
/// How far sprites may reach outside of the tile boxes of the map, in game pixels. Tall sprites
/// and raised terrain reach above the top corner. `warn_about_draws_outside` in `bake.rs` warns
/// about draws outside of the baked area.
const MARGIN_SIDES: i32 = 64;
const MARGIN_BOTTOM: i32 = 64;
const MARGIN_TOP: i32 = 320;

/// Game pixel area that the sprites of a map can cover
pub fn map_bounds() -> IRect {
    let size = i32::from(JA2_MAP_SIZE);
    let half_tile = JA2_TILE_WIDTH / 2;
    // The tile boxes reach from the left corner (0, 159) to the right corner (159, 0) and from
    // the top corner (0, 0) to the bottom corner (159, 159)
    IRect::new(
        -(size - 1) * half_tile - MARGIN_SIDES,
        -MARGIN_TOP,
        (size + 1) * half_tile + MARGIN_SIDES,
        size * JA2_TILE_HEIGHT + MARGIN_BOTTOM,
    )
}

/// Grid of equally sized regions covering an area of game pixels
#[derive(Debug, Clone, PartialEq)]
pub struct RegionLayout {
    /// Top left corner of the grid in game pixels
    pub origin: IVec2,
    /// Number of regions in each direction
    pub count: UVec2,
    /// Size of each region in texels, always even, so that the regions have integer centers
    pub size: UVec2,
}

impl Default for RegionLayout {
    /// The regions of the map
    fn default() -> Self {
        Self::for_bounds(map_bounds())
    }
}

impl RegionLayout {
    pub fn for_bounds(bounds: IRect) -> Self {
        let origin = bounds.min;
        let extent = bounds.size().as_uvec2();
        let div_ceil = |a: UVec2, b: UVec2| UVec2::new(a.x.div_ceil(b.x), a.y.div_ceil(b.y));
        let count = div_ceil(extent, UVec2::splat(MAX_REGION_SIZE));
        let size = div_ceil(extent, count);
        let size = (size + UVec2::ONE) / 2 * 2;
        RegionLayout {
            origin,
            count,
            size,
        }
    }

    pub fn len(&self) -> usize {
        (self.count.x * self.count.y) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Game pixel positions of the top left corners of the regions, row by row
    pub fn origins(&self) -> impl Iterator<Item = IVec2> + '_ {
        (0..self.count.y).flat_map(move |y| {
            (0..self.count.x).map(move |x| self.origin + (UVec2::new(x, y) * self.size).as_ivec2())
        })
    }

    /// Game pixel area of a region
    pub fn rect(&self, index: usize) -> IRect {
        let count_x = self.count.x as usize;
        let cell = UVec2::new((index % count_x) as u32, (index / count_x) as u32);
        let min = self.origin + (cell * self.size).as_ivec2();
        IRect::from_corners(min, min + self.size.as_ivec2())
    }

    /// Indices of the regions that overlap `rect`
    pub fn overlapping(&self, rect: IRect) -> impl Iterator<Item = usize> + '_ {
        let size = self.size.as_ivec2();
        let covered = IRect::from_corners(self.origin, self.origin + self.count.as_ivec2() * size);
        let rect = rect.intersect(covered);
        let (min, max) = if rect.is_empty() {
            (IVec2::ZERO, IVec2::splat(-1))
        } else {
            (
                (rect.min - self.origin).div_euclid(size),
                (rect.max - 1 - self.origin).div_euclid(size),
            )
        };
        let count_x = self.count.x as usize;
        (min.y..=max.y)
            .flat_map(move |y| (min.x..=max.x).map(move |x| y as usize * count_x + x as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::map::{game_rect_to_world, tile_box, world_to_game};

    /// Game pixel under a world position
    fn game_pixel(world: Vec2) -> IVec2 {
        world_to_game(world).floor().as_ivec2()
    }

    #[test]
    fn regions_cover_bounds() {
        let bounds = map_bounds();
        let layout = RegionLayout::for_bounds(bounds);
        assert!(layout.size.x <= MAX_REGION_SIZE && layout.size.y <= MAX_REGION_SIZE);
        assert!(layout.size.x.is_multiple_of(2) && layout.size.y.is_multiple_of(2));
        let covered_max = layout.origin + (layout.count * layout.size).as_ivec2();
        assert_eq!(layout.origin, bounds.min);
        assert!(covered_max.cmpge(bounds.max).all());
        assert_eq!(layout.origins().count(), layout.len());
    }

    #[test]
    fn map_bounds_contain_the_tile_boxes() {
        let bounds = map_bounds();
        // Left, right, top and bottom corners
        assert!(bounds.contains(tile_box(0, 159)));
        assert!(bounds.contains(tile_box(159, 0) + IVec2::new(39, 0)));
        assert!(bounds.contains(IVec2::new(20, 0)));
        assert!(bounds.contains(tile_box(159, 159) + IVec2::new(20, 19)));
    }

    #[test]
    fn region_centers_are_integer_world_positions() {
        let layout = RegionLayout {
            origin: IVec2::new(-10, -4),
            count: UVec2::new(2, 1),
            size: UVec2::new(10, 8),
        };
        let origins = layout.origins().collect::<Vec<_>>();
        assert_eq!(origins, vec![IVec2::new(-10, -4), IVec2::new(0, -4)]);
        let rect = game_rect_to_world(origins[1], origins[1] + layout.size.as_ivec2());
        assert_eq!(rect.center(), rect.center().round());
        // The top left texel of the region is its game origin
        assert_eq!(
            game_pixel(Vec2::new(rect.min.x, rect.max.y) + Vec2::new(0.5, -0.5)),
            origins[1]
        );
    }

    #[test]
    fn overlapping_regions() {
        let layout = RegionLayout {
            origin: IVec2::new(-10, -4),
            count: UVec2::new(3, 2),
            size: UVec2::new(10, 8),
        };
        let overlapping = |rect| layout.overlapping(rect).collect::<Vec<_>>();
        assert_eq!(layout.rect(4), IRect::new(0, 4, 10, 12));
        assert_eq!(overlapping(IRect::new(0, 4, 10, 12)), vec![4]);
        assert_eq!(overlapping(IRect::new(-1, 3, 1, 5)), vec![0, 1, 3, 4]);
        assert_eq!(
            overlapping(IRect::new(-100, -100, 100, 100)),
            (0..6).collect::<Vec<_>>()
        );
        assert_eq!(
            overlapping(IRect::new(100, 100, 110, 110)),
            Vec::<usize>::new()
        );
        assert_eq!(overlapping(IRect::new(5, 5, 5, 9)), Vec::<usize>::new());
    }
}
