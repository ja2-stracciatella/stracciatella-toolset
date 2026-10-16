//! Spatial bins of the draw list
//!
//! The replay shader computes each pixel from the draws that cover it. To find them quickly, the
//! drawn area is split into square bins, and each bin lists the draws that overlap it, in draw
//! list order.

use bevy::prelude::*;

/// Size of a bin in game pixels
pub const BIN_SIZE: i32 = 32;

/// The draws of each bin
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bins {
    /// Game pixel position of the top left corner of the first bin
    pub origin: IVec2,
    /// Number of bins in x and y
    pub count: UVec2,
    /// The draws of bin `i` (row by row) are `indices[offsets[i]..offsets[i + 1]]`
    pub offsets: Vec<u32>,
    /// Indices into the draw list
    pub indices: Vec<u32>,
}

impl Bins {
    /// Bins that cover `area` (in game pixels) and list the draws with the given bounds
    pub fn build(area: IRect, draw_bounds: impl Iterator<Item = IRect> + Clone) -> Self {
        let origin = IVec2::new(
            area.min.x.div_euclid(BIN_SIZE) * BIN_SIZE,
            area.min.y.div_euclid(BIN_SIZE) * BIN_SIZE,
        );
        let extent = (area.max - origin).max(IVec2::ZERO);
        let count = UVec2::new(
            (extent.x as u32).div_ceil(BIN_SIZE as u32),
            (extent.y as u32).div_ceil(BIN_SIZE as u32),
        );
        // Range of bins that a draw overlaps, `None` if it is outside of the area
        let bin_range = |bounds: IRect| -> Option<(UVec2, UVec2)> {
            let bounds = bounds.intersect(area);
            if bounds.is_empty() {
                return None;
            }
            let min = ((bounds.min - origin) / BIN_SIZE).as_uvec2();
            let max = ((bounds.max - 1 - origin) / BIN_SIZE).as_uvec2();
            Some((min, max))
        };

        // Count first, so the indices can be written in place
        let bin_count = (count.x * count.y) as usize;
        let mut counts = vec![0u32; bin_count + 1];
        for bounds in draw_bounds.clone() {
            if let Some((min, max)) = bin_range(bounds) {
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        counts[(y * count.x + x) as usize] += 1;
                    }
                }
            }
        }
        let mut offsets = Vec::with_capacity(bin_count + 1);
        let mut total = 0;
        for bin_draws in &counts {
            offsets.push(total);
            total += bin_draws;
        }
        let mut next = offsets.clone();
        let mut indices = vec![0; total as usize];
        for (draw, bounds) in draw_bounds.enumerate() {
            if let Some((min, max)) = bin_range(bounds) {
                for y in min.y..=max.y {
                    for x in min.x..=max.x {
                        let bin = (y * count.x + x) as usize;
                        indices[next[bin] as usize] = draw as u32;
                        next[bin] += 1;
                    }
                }
            }
        }
        Bins {
            origin,
            count,
            offsets,
            indices,
        }
    }

    /// Draws of the bin that contains a game pixel
    pub fn draws_at(&self, pixel: IVec2) -> &[u32] {
        let bin = (pixel - self.origin).div_euclid(IVec2::splat(BIN_SIZE));
        if bin.x < 0 || bin.y < 0 || bin.x as u32 >= self.count.x || bin.y as u32 >= self.count.y {
            return &[];
        }
        let bin = (bin.y as u32 * self.count.x + bin.x as u32) as usize;
        &self.indices[self.offsets[bin] as usize..self.offsets[bin + 1] as usize]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draws_keep_their_order_in_every_bin() {
        let area = IRect::new(-40, 0, 64, 64);
        let draws = [
            // Covers all bins
            IRect::new(-40, 0, 64, 64),
            // Only the top left bin
            IRect::new(-64, -10, -33, 5),
            // Ends exactly at a bin border
            IRect::new(0, 0, 32, 32),
            // Outside of the area
            IRect::new(100, 100, 120, 120),
            // Empty
            IRect::new(10, 10, 10, 20),
        ];
        let bins = Bins::build(area, draws.iter().copied());
        assert_eq!(bins.origin, IVec2::new(-64, 0));
        assert_eq!(bins.count, UVec2::new(4, 2));
        assert_eq!(bins.offsets.len(), 9);
        assert_eq!(bins.draws_at(IVec2::new(-40, 0)), &[0, 1]);
        assert_eq!(bins.draws_at(IVec2::new(-1, 31)), &[0]);
        assert_eq!(bins.draws_at(IVec2::new(0, 0)), &[0, 2]);
        assert_eq!(bins.draws_at(IVec2::new(31, 31)), &[0, 2]);
        assert_eq!(bins.draws_at(IVec2::new(32, 32)), &[0]);
        assert_eq!(bins.draws_at(IVec2::new(0, 64)), &[] as &[u32]);
        assert_eq!(bins.draws_at(IVec2::new(-65, 0)), &[] as &[u32]);
    }
}
