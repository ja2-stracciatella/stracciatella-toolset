//! Map coordinates: grid positions, game screen pixels and Bevy world space
//!
//! The game places the tile at column `x` and row `y` so that the top left corner of its 40×20
//! bounding box is at `(20 * (x - y), 10 * (x + y))` screen pixels, with y pointing down. The
//! renderer works in these "game pixels". Bevy world space has y pointing up and puts the center of
//! the map (the center of tile (80, 80)) at the origin, so every game pixel covers exactly one
//! world unit square with integer corners.

use bevy::prelude::*;

/// Number of rows and columns of a map (`WORLD_ROWS`, `WORLD_COLS`)
pub const JA2_MAP_SIZE: u8 = 160;
/// Number of tiles of a map (`WORLD_MAX`)
pub const JA2_MAP_TILES: usize = JA2_MAP_SIZE as usize * JA2_MAP_SIZE as usize;

/// The tile width of a single Ja2 tile
pub const JA2_TILE_WIDTH: i32 = 40;
/// The tile height of a single Ja2 tile
pub const JA2_TILE_HEIGHT: i32 = 20;

/// Game pixel that is at the world origin: the center of the map
const WORLD_ORIGIN: IVec2 = IVec2::new(20, 1610);

/// Position of a tile on the map, in columns (x) and rows (y)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct GridPosition {
    pub x: u8,
    pub y: u8,
}

impl GridPosition {
    pub fn new(x: u8, y: u8) -> Self {
        Self { x, y }
    }

    /// The tile at a position, `None` if it is outside of the map
    pub fn from_xy(x: i32, y: i32) -> Option<Self> {
        let size = i32::from(JA2_MAP_SIZE);
        ((0..size).contains(&x) && (0..size).contains(&y)).then(|| Self::new(x as u8, y as u8))
    }

    /// Top left corner of the tile's bounding box in game pixels
    pub fn tile_box(&self) -> IVec2 {
        tile_box(i32::from(self.x), i32::from(self.y))
    }
}

/// Top left corner of the bounding box of the tile at column `x` and row `y`, in game pixels
pub fn tile_box(x: i32, y: i32) -> IVec2 {
    IVec2::new(JA2_TILE_WIDTH / 2 * (x - y), JA2_TILE_HEIGHT / 2 * (x + y))
}

/// World position of the top left corner of a game pixel
pub fn game_to_world(game: IVec2) -> Vec2 {
    let world = game - WORLD_ORIGIN;
    Vec2::new(world.x as f32, -world.y as f32)
}

/// Game pixel position (with fractions) of a world position
pub fn world_to_game(world: Vec2) -> Vec2 {
    Vec2::new(world.x, -world.y) + WORLD_ORIGIN.as_vec2()
}

/// World rectangle covered by a rectangle of game pixels
pub fn game_rect_to_world(min: IVec2, max: IVec2) -> Rect {
    Rect::from_corners(game_to_world(min), game_to_world(max))
}

/// Fractional tile coordinates of a game pixel position, ignoring the terrain height
///
/// The tile (x, y) covers the coordinates `[x, x + 1) × [y, y + 1)`.
pub fn game_to_tile(game: Vec2) -> Vec2 {
    let half_tile = Vec2::new(JA2_TILE_WIDTH as f32, JA2_TILE_HEIGHT as f32) / 2.;
    // x - y and x + y of the tile whose center is at `game`
    let diff = (game.x - half_tile.x) / half_tile.x;
    let sum = (game.y - half_tile.y) / half_tile.y;
    Vec2::new((sum + diff) / 2. + 0.5, (sum - diff) / 2. + 0.5)
}

/// Highest terrain height of a tile in game pixels (`sHeight` is stored in a byte)
const MAX_TERRAIN_HEIGHT: i32 = u8::MAX as i32;

/// The tile below a game position, whose diamond is raised by its terrain height like the
/// renderer draws it
///
/// Tiles further down are drawn over the ones above them, so the one lowest on the map that is
/// raised up to the position wins. Where no raised tile covers the position (e.g. on the face of
/// a cliff), it is the tile of the flat ground.
pub fn raised_tile_at(game: Vec2, height: impl Fn(GridPosition) -> i32) -> Option<GridPosition> {
    let flat = |game: Vec2| {
        let tile = game_to_tile(game).floor();
        GridPosition::from_xy(tile.x as i32, tile.y as i32)
    };
    (1..=MAX_TERRAIN_HEIGHT)
        .rev()
        .filter_map(|lift| flat(game + Vec2::new(0., lift as f32)))
        .find(|&tile| {
            let lift = height(tile);
            lift > 0 && flat(game + Vec2::new(0., lift as f32)) == Some(tile)
        })
        .or_else(|| flat(game))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions() {
        assert_eq!(GridPosition::from_xy(3, 2), Some(GridPosition::new(3, 2)));
        assert_eq!(GridPosition::from_xy(-1, 0), None);
        assert_eq!(GridPosition::from_xy(0, 160), None);
    }

    #[test]
    fn raised_tiles_are_picked_where_they_are_drawn() {
        let center = |x, y| tile_box(x, y).as_vec2() + Vec2::new(20., 10.);
        let flat = |_| 0;
        assert_eq!(
            raised_tile_at(center(10, 10), flat),
            Some(GridPosition::new(10, 10))
        );
        // (12, 12) is raised by 40 px, so it is drawn where (10, 10) would be
        let hill = |p: GridPosition| {
            if p == GridPosition::new(12, 12) {
                40
            } else {
                0
            }
        };
        assert_eq!(
            raised_tile_at(center(10, 10), hill),
            Some(GridPosition::new(12, 12))
        );
        // Its flat place is empty, the ground there is picked
        assert_eq!(
            raised_tile_at(center(12, 12), hill),
            Some(GridPosition::new(12, 12))
        );
        assert_eq!(
            raised_tile_at(center(11, 11), hill),
            Some(GridPosition::new(11, 11))
        );
        // A raised plateau is shifted as a whole
        let plateau = |_| 20;
        assert_eq!(
            raised_tile_at(center(10, 10), plateau),
            Some(GridPosition::new(11, 11))
        );
    }

    #[test]
    fn tile_boxes() {
        assert_eq!(tile_box(0, 0), IVec2::new(0, 0));
        assert_eq!(tile_box(1, 0), IVec2::new(20, 10));
        assert_eq!(tile_box(0, 1), IVec2::new(-20, 10));
        assert_eq!(tile_box(80, 80), IVec2::new(0, 1600));
    }

    #[test]
    fn world_origin_is_map_center() {
        let center = tile_box(80, 80) + IVec2::new(20, 10);
        assert_eq!(game_to_world(center), Vec2::ZERO);
        assert_eq!(game_to_world(center + IVec2::new(1, 1)), Vec2::new(1., -1.));
        assert_eq!(world_to_game(Vec2::new(1., -1.)), center.as_vec2() + 1.);
        let rect = game_rect_to_world(center, center + IVec2::new(2, 3));
        assert_eq!(rect, Rect::new(0., -3., 2., 0.));
    }

    #[test]
    fn tile_under_game_pixel() {
        let tile = |game: Vec2| game_to_tile(game).floor();
        let center = |x, y| (tile_box(x, y) + IVec2::new(20, 10)).as_vec2();
        assert_eq!(game_to_tile(center(5, 7)), Vec2::new(5.5, 7.5));
        assert_eq!(tile(center(5, 7)), Vec2::new(5., 7.));
        // Just inside of the corners of the diamond
        assert_eq!(tile(center(5, 7) + Vec2::new(-19.5, 0.)), Vec2::new(5., 7.));
        assert_eq!(tile(center(5, 7) + Vec2::new(19.5, 0.)), Vec2::new(5., 7.));
        assert_eq!(tile(center(5, 7) + Vec2::new(0., -9.5)), Vec2::new(5., 7.));
        assert_eq!(tile(center(5, 7) + Vec2::new(0., 9.5)), Vec2::new(5., 7.));
        // Neighbours
        assert_eq!(tile(center(5, 7) + Vec2::new(10., 5.5)), Vec2::new(6., 7.));
        assert_eq!(tile(center(5, 7) + Vec2::new(-10., 5.5)), Vec2::new(5., 8.));
    }
}
