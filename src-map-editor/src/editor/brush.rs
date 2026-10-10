//! The tiles a brush covers (`UpdateCursorAreas` in `Cursor_Modes.cc`) and the density of
//! placements (`PerformDensityTest`)

use bevy::math::Vec2;
use serde::Deserialize;

use crate::{
    components::map::{GridPosition, JA2_MAP_SIZE},
    map::GridNo,
    random::Random,
};

/// The largest size of the square and circle brushes and width of the line brush
pub const MAX_BRUSH_SIZE: u8 = 15;

/// Size and shape of the brush (`gusSelectionType`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Brush {
    /// `size`×`size` tiles around the cursor
    Square { size: u8 },
    /// The tiles of the `size`×`size` square that are within the circle inside it
    Circle { size: u8 },
    /// A line from where the mouse was pressed, `width` tiles wide, applied on release
    Line { width: u8 },
    /// A rectangle from where the mouse was pressed, applied on release
    Area,
    /// The connected area of the topmost texture of the clicked tile, only for the terrain
    Fill,
}

impl Default for Brush {
    fn default() -> Self {
        Brush::Square { size: 1 }
    }
}

impl Brush {
    /// Whether the brush is applied once when the mouse is released, instead of while it moves
    pub fn applies_on_release(self) -> bool {
        matches!(self, Brush::Line { .. } | Brush::Area)
    }

    /// The tiles the brush covers with the cursor on `cursor`, `anchor` is where the mouse was
    /// pressed for the brushes that are applied on release
    pub fn region(self, anchor: Option<GridPosition>, cursor: GridPosition) -> BrushArea {
        let tiles = match (self, anchor) {
            (Brush::Fill, _) | (Brush::Line { .. } | Brush::Area, None) => {
                TileRect::single(cursor).tiles().collect()
            }
            (Brush::Square { size }, _) => TileRect::square(cursor, size).tiles().collect(),
            (Brush::Circle { size }, _) => circle(cursor, size),
            (Brush::Area, Some(anchor)) => TileRect::between(anchor, cursor).tiles().collect(),
            (Brush::Line { width }, Some(anchor)) => {
                TileRect::line(anchor, cursor, width).tiles().collect()
            }
        };
        BrushArea { tiles }
    }
}

/// The offsets from the cursor of a square brush: the cursor is in the middle, or left of and
/// above the middle for even sizes
fn square_offsets(size: u8) -> (i32, i32) {
    let size = i32::from(size.clamp(1, MAX_BRUSH_SIZE));
    (-(size - 1) / 2, size / 2)
}

/// The tiles of a circle brush, column by column and clamped to the map
fn circle(cursor: GridPosition, size: u8) -> Vec<GridNo> {
    let (min, max) = square_offsets(size);
    // The middle of the square, relative to the cursor tile
    let center = (min + max) as f32 / 2.;
    let radius = (max - min + 1) as f32 / 2.;
    let (x, y) = (i32::from(cursor.x), i32::from(cursor.y));
    (min..=max)
        .flat_map(|dx| (min..=max).map(move |dy| (dx, dy)))
        .filter(|&(dx, dy)| {
            let offset = Vec2::new(dx as f32 - center, dy as f32 - center);
            offset.length_squared() <= radius * radius
        })
        .filter_map(|(dx, dy)| GridPosition::from_xy(x + dx, y + dy))
        .map(GridNo::from)
        .collect()
}

/// The tiles a brush covers, column by column
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrushArea {
    pub tiles: Vec<GridNo>,
}

impl BrushArea {
    /// The tile in the first column and row, the clicked tile for roads and fills, which cover
    /// a single tile, and the tile where a drag started
    pub fn first(&self) -> Option<GridNo> {
        self.tiles.first().copied()
    }

    /// The tile where a drag ended
    pub fn last(&self) -> Option<GridNo> {
        self.tiles.last().copied()
    }

    /// Whether the brush covers a single tile
    pub fn is_single(&self) -> bool {
        self.tiles.len() == 1
    }

    pub fn tiles(&self) -> impl Iterator<Item = GridNo> + '_ {
        self.tiles.iter().copied()
    }
}

/// A rectangle of tiles, the maximum is included (`gSelectRegion`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TileRect {
    pub min: GridPosition,
    pub max: GridPosition,
}

/// Clamps a column or row to the map (`ValidateSelectionRegionBoundaries`)
fn clamp(value: i32) -> u8 {
    value.clamp(0, i32::from(JA2_MAP_SIZE) - 1) as u8
}

impl TileRect {
    fn single(position: GridPosition) -> Self {
        Self {
            min: position,
            max: position,
        }
    }

    /// The tiles of a square brush, clamped to the map
    fn square(cursor: GridPosition, size: u8) -> Self {
        let (min, max) = square_offsets(size);
        let (x, y) = (i32::from(cursor.x), i32::from(cursor.y));
        Self {
            min: GridPosition::new(clamp(x + min), clamp(y + min)),
            max: GridPosition::new(clamp(x + max), clamp(y + max)),
        }
    }

    /// The rectangle with two tiles in its corners
    fn between(a: GridPosition, b: GridPosition) -> Self {
        Self {
            min: GridPosition::new(a.x.min(b.x), a.y.min(b.y)),
            max: GridPosition::new(a.x.max(b.x), a.y.max(b.y)),
        }
    }

    /// A line of `width` tiles from the anchor towards the cursor, along the axis on which the
    /// cursor is farther away (`ForceAreaSelectionWidth`)
    fn line(anchor: GridPosition, cursor: GridPosition, width: u8) -> Self {
        let rect = Self::between(anchor, cursor);
        let extra = i32::from(width.max(1)) - 1;
        let (ax, ay) = (i32::from(anchor.x), i32::from(anchor.y));
        let (cx, cy) = (i32::from(cursor.x), i32::from(cursor.y));
        let (min, max) = if (cx - ax).abs() < (cy - ay).abs() {
            // Restrict the columns
            let (left, right) = if cx < ax {
                (ax - extra, ax)
            } else {
                (ax, ax + extra)
            };
            (
                GridPosition::new(clamp(left), rect.min.y),
                GridPosition::new(clamp(right), rect.max.y),
            )
        } else {
            // Restrict the rows
            let (top, bottom) = if cy < ay {
                (ay - extra, ay)
            } else {
                (ay, ay + extra)
            };
            (
                GridPosition::new(rect.min.x, clamp(top)),
                GridPosition::new(rect.max.x, clamp(bottom)),
            )
        };
        Self { min, max }
    }

    /// The tiles, column by column like the original places them
    fn tiles(self) -> impl Iterator<Item = GridNo> {
        (self.min.x..=self.max.x).flat_map(move |x| {
            (self.min.y..=self.max.y).map(move |y| GridNo::from(GridPosition::new(x, y)))
        })
    }
}

/// Whether a tile gets a placement, `density` is in percent (`PerformDensityTest`)
pub fn density_test(random: &mut Random, density: u8) -> bool {
    random.next(100) < u32::from(density)
}

/// The tiles on the line between two tiles, without the first one (Bresenham)
///
/// While the mouse moves fast, it skips tiles. The brush is applied to the tiles in between, so
/// that strokes have no gaps.
pub fn tiles_between(from: GridPosition, to: GridPosition) -> Vec<GridPosition> {
    let (mut x, mut y) = (i32::from(from.x), i32::from(from.y));
    let (x1, y1) = (i32::from(to.x), i32::from(to.y));
    let dx = (x1 - x).abs();
    let dy = -(y1 - y).abs();
    let sx = if x < x1 { 1 } else { -1 };
    let sy = if y < y1 { 1 } else { -1 };
    let mut error = dx + dy;
    let mut tiles = vec![];
    while (x, y) != (x1, y1) {
        let e2 = 2 * error;
        if e2 >= dy {
            error += dy;
            x += sx;
        }
        if e2 <= dx {
            error += dx;
            y += sy;
        }
        tiles.extend(GridPosition::from_xy(x, y));
    }
    tiles
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: u8, y: u8) -> GridPosition {
        GridPosition::new(x, y)
    }

    fn rect(min: (u8, u8), max: (u8, u8)) -> TileRect {
        TileRect {
            min: p(min.0, min.1),
            max: p(max.0, max.1),
        }
    }

    fn area(rect: TileRect) -> BrushArea {
        BrushArea {
            tiles: rect.tiles().collect(),
        }
    }

    fn square(size: u8) -> Brush {
        Brush::Square { size }
    }

    #[test]
    fn square_brushes_are_centered_and_clamped() {
        assert_eq!(square(1).region(None, p(5, 5)), area(rect((5, 5), (5, 5))));
        assert_eq!(square(3).region(None, p(5, 5)), area(rect((4, 4), (6, 6))));
        assert_eq!(square(5).region(None, p(5, 5)), area(rect((3, 3), (7, 7))));
        assert_eq!(square(7).region(None, p(5, 5)), area(rect((2, 2), (8, 8))));
        // Even sizes reach farther right and down
        assert_eq!(square(2).region(None, p(5, 5)), area(rect((5, 5), (6, 6))));
        assert_eq!(square(4).region(None, p(5, 5)), area(rect((4, 4), (7, 7))));
        assert_eq!(
            square(7).region(None, p(1, 158)),
            area(rect((0, 155), (4, 159)))
        );
        assert_eq!(square(5).region(None, p(5, 5)).tiles().count(), 25);
        assert_eq!(
            square(MAX_BRUSH_SIZE)
                .region(None, p(80, 80))
                .tiles()
                .count(),
            225
        );
        // Out of range sizes are clamped
        assert_eq!(
            square(0).region(None, p(5, 5)),
            square(1).region(None, p(5, 5))
        );
        assert_eq!(
            square(200).region(None, p(80, 80)),
            square(MAX_BRUSH_SIZE).region(None, p(80, 80))
        );
        assert!(square(1).region(None, p(5, 5)).is_single());
        assert!(!square(2).region(None, p(5, 5)).is_single());
        assert_eq!(Brush::default(), square(1));
    }

    #[test]
    fn circle_brushes_leave_out_the_corners() {
        let circle = |size| Brush::Circle { size }.region(None, p(50, 50));
        assert_eq!(circle(1), square(1).region(None, p(50, 50)));
        assert_eq!(circle(2), square(2).region(None, p(50, 50)));
        let five = circle(5);
        assert_eq!(five.tiles().count(), 21);
        for corner in [(48, 48), (52, 48), (48, 52), (52, 52)] {
            assert!(!five.tiles.contains(&GridNo::from(p(corner.0, corner.1))));
        }
        assert!(five.tiles.contains(&GridNo::from(p(52, 50))));
        assert!(five.tiles.contains(&GridNo::from(p(51, 52))));
        // Symmetric around the middle of the square
        let four = circle(4);
        assert_eq!(four.tiles().count(), 12);
        assert!(!four.tiles.contains(&GridNo::from(p(49, 49))));
        assert!(!four.tiles.contains(&GridNo::from(p(52, 52))));
        assert!(four.tiles.contains(&GridNo::from(p(49, 50))));
        assert!(four.tiles.contains(&GridNo::from(p(52, 51))));
        // Clamped to the map
        let edge = Brush::Circle { size: 5 }.region(None, p(0, 0));
        assert_eq!(edge.tiles().count(), 8);
        assert_eq!(edge.first(), Some(GridNo::from(p(0, 0))));
    }

    #[test]
    fn area_and_line_brushes_start_at_the_anchor() {
        let rect = |min, max| area(rect(min, max));
        assert_eq!(Brush::Area.region(None, p(5, 5)), rect((5, 5), (5, 5)));
        assert_eq!(
            Brush::Area.region(Some(p(9, 2)), p(5, 5)),
            rect((5, 2), (9, 5))
        );
        let line = Brush::Line { width: 2 };
        // Farther away horizontally, so the rows are restricted
        assert_eq!(
            line.region(Some(p(10, 10)), p(20, 12)),
            rect((10, 10), (20, 11))
        );
        assert_eq!(
            line.region(Some(p(10, 10)), p(2, 8)),
            rect((2, 9), (10, 10))
        );
        // Farther away vertically
        assert_eq!(
            line.region(Some(p(10, 10)), p(8, 30)),
            rect((9, 10), (10, 30))
        );
        assert_eq!(
            Brush::Line { width: 3 }.region(Some(p(0, 10)), p(0, 30)),
            rect((0, 10), (2, 30))
        );
        assert_eq!(Brush::Fill.region(None, p(7, 8)), rect((7, 8), (7, 8)));
        assert!(Brush::Area.applies_on_release());
        assert!(!square(3).applies_on_release());
        assert!(!Brush::Circle { size: 3 }.applies_on_release());
    }

    #[test]
    fn rect_tiles_go_column_by_column() {
        let tiles: Vec<_> = rect((1, 1), (2, 2))
            .tiles()
            .map(|g| (g.position().x, g.position().y))
            .collect();
        assert_eq!(tiles, [(1, 1), (1, 2), (2, 1), (2, 2)]);
    }

    #[test]
    fn density_is_a_percentage() {
        let mut random = Random::new(1);
        let hits = (0..1000).filter(|_| density_test(&mut random, 20)).count();
        assert!((150..250).contains(&hits), "{hits}");
        assert!((0..100).all(|_| density_test(&mut random, 100)));
    }

    #[test]
    fn lines_between_tiles_have_no_gaps() {
        assert!(tiles_between(p(3, 3), p(3, 3)).is_empty());
        assert_eq!(tiles_between(p(0, 0), p(3, 0)), [p(1, 0), p(2, 0), p(3, 0)]);
        let diagonal = tiles_between(p(10, 10), p(4, 13));
        assert_eq!(diagonal.last(), Some(&p(4, 13)));
        let mut previous = p(10, 10);
        for tile in diagonal {
            assert!(tile.x.abs_diff(previous.x) <= 1 && tile.y.abs_diff(previous.y) <= 1);
            previous = tile;
        }
    }

    #[test]
    fn brushes_are_read_from_json() {
        let brush: Brush = serde_json::from_str(r#"{"kind":"line","width":4}"#).unwrap();
        assert_eq!(brush, Brush::Line { width: 4 });
        let brush: Brush = serde_json::from_str(r#"{"kind":"square","size":3}"#).unwrap();
        assert_eq!(brush, square(3));
        let brush: Brush = serde_json::from_str(r#"{"kind":"circle","size":5}"#).unwrap();
        assert_eq!(brush, Brush::Circle { size: 5 });
        let brush: Brush = serde_json::from_str(r#"{"kind":"area"}"#).unwrap();
        assert_eq!(brush, Brush::Area);
        assert!(serde_json::from_str::<Brush>(r#"{"kind":"xlarge"}"#).is_err());
    }
}
