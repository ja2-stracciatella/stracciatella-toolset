//! Grid numbers, the tile indices of the game, and per tile data indexed by them

use std::ops::{Index, IndexMut};

use crate::components::map::{GridPosition, JA2_MAP_SIZE, JA2_MAP_TILES};

const WORLD_COLS: i32 = JA2_MAP_SIZE as i32;
const WORLD_MAX: i32 = JA2_MAP_TILES as i32;

/// A tile of the map (`GridNo`), `y * 160 + x`
///
/// Always on the map. `NOWHERE` and other invalid grid numbers of the map file are `None`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GridNo(u16);

impl GridNo {
    /// The grid number with the given index, `None` if it is not on the map
    pub fn new(index: usize) -> Option<Self> {
        (index < JA2_MAP_TILES).then_some(Self(index as u16))
    }

    /// The grid number of a raw number of the game, `None` if it is not on the map
    pub fn from_i32(grid_no: i32) -> Option<Self> {
        (0..WORLD_MAX)
            .contains(&grid_no)
            .then_some(Self(grid_no as u16))
    }

    /// All grid numbers in ascending order
    pub fn all() -> impl DoubleEndedIterator<Item = GridNo> + ExactSizeIterator {
        (0..JA2_MAP_TILES as u16).map(Self)
    }

    pub const fn index(self) -> usize {
        self.0 as usize
    }

    pub fn position(self) -> GridPosition {
        let size = u16::from(JA2_MAP_SIZE);
        GridPosition::new((self.0 % size) as u8, (self.0 / size) as u8)
    }

    /// The grid number `delta` away, `None` if that is not on the map
    ///
    /// This is the plain arithmetic of the game: steps past the left or right edge wrap into the
    /// neighbouring row.
    pub fn offset(self, delta: i32) -> Option<GridNo> {
        Self::from_i32(i32::from(self.0) + delta)
    }

    /// The tile `dx` columns and `dy` rows away, `None` if that is off the map
    ///
    /// Unlike [`GridNo::offset`], steps past the left or right edge don't wrap into the
    /// neighbouring row.
    pub fn neighbour(self, dx: i32, dy: i32) -> Option<GridNo> {
        let position = self.position();
        GridPosition::from_xy(i32::from(position.x) + dx, i32::from(position.y) + dy)
            .map(GridNo::from)
    }

    /// The grid number one step away, or this one if the step leaves the map (`NewGridNo`)
    pub fn step(self, direction: Direction) -> GridNo {
        let grid_no = i32::from(self.0);
        let proposed = grid_no + direction.grid_no_increment();
        if out_of_bounds(grid_no, proposed) {
            self
        } else {
            GridNo(proposed as u16)
        }
    }
}

impl From<GridPosition> for GridNo {
    fn from(position: GridPosition) -> Self {
        Self(u16::from(position.y) * u16::from(JA2_MAP_SIZE) + u16::from(position.x))
    }
}

/// Whether a step from `grid_no` to `proposed` leaves the map (`OutOfBounds`)
pub(crate) fn out_of_bounds(grid_no: i32, proposed: i32) -> bool {
    let column = grid_no % WORLD_COLS;
    let last_row_start = WORLD_MAX - WORLD_COLS;
    if column != 0 && column != WORLD_COLS - 1 && grid_no < last_row_start && grid_no > WORLD_COLS {
        return false;
    }
    if proposed < 0 {
        return true;
    }
    let proposed_column = proposed % WORLD_COLS;
    (column == 0 && proposed_column == WORLD_COLS - 1)
        || (column == WORLD_COLS - 1 && proposed_column == 0)
        || (grid_no >= last_row_start && proposed >= WORLD_MAX)
}

/// One value for every tile of the map, indexed by [`GridNo`]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GridArray<T>(Box<[T]>);

impl<T> GridArray<T> {
    pub fn from_fn(mut f: impl FnMut(GridNo) -> T) -> Self {
        Self(GridNo::all().map(&mut f).collect())
    }

    /// The values with their grid numbers, in ascending order
    pub fn iter(&self) -> impl DoubleEndedIterator<Item = (GridNo, &T)> + ExactSizeIterator {
        GridNo::all().zip(self.0.iter())
    }

    pub fn values(&self) -> impl DoubleEndedIterator<Item = &T> + ExactSizeIterator {
        self.0.iter()
    }

    pub fn values_mut(&mut self) -> impl DoubleEndedIterator<Item = &mut T> + ExactSizeIterator {
        self.0.iter_mut()
    }
}

impl<T: Clone> GridArray<T> {
    /// Every tile with the same value
    pub fn filled(value: T) -> Self {
        Self(vec![value; JA2_MAP_TILES].into_boxed_slice())
    }
}

impl<T: Default> Default for GridArray<T> {
    fn default() -> Self {
        Self::from_fn(|_| T::default())
    }
}

impl<T> Index<GridNo> for GridArray<T> {
    type Output = T;

    fn index(&self, grid_no: GridNo) -> &T {
        &self.0[grid_no.index()]
    }
}

impl<T> IndexMut<GridNo> for GridArray<T> {
    fn index_mut(&mut self, grid_no: GridNo) -> &mut T {
        &mut self.0[grid_no.index()]
    }
}

/// Height level of the game (`bLevel`), also the floor of the viewer (`"ground"` or `"roof"`)
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Level {
    #[default]
    Ground = 0,
    /// On a roof
    Roof = 1,
}

impl Level {
    /// The level from the number of the map file, anything but 0 is the roof
    pub fn from_number(number: u8) -> Self {
        if number == 0 {
            Level::Ground
        } else {
            Level::Roof
        }
    }

    /// Number of the level, 0 or 1
    pub const fn number(self) -> u8 {
        self as u8
    }
}

/// Directions of the game (`WorldDirections`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[repr(u8)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub const ALL: [Direction; 8] = [
        Direction::North,
        Direction::NorthEast,
        Direction::East,
        Direction::SouthEast,
        Direction::South,
        Direction::SouthWest,
        Direction::West,
        Direction::NorthWest,
    ];

    /// Direction with the given number, like `DirectionInc` numbers > 7 become north east
    pub fn from_number(number: u8) -> Direction {
        Direction::ALL
            .get(usize::from(number))
            .copied()
            .unwrap_or(Direction::NorthEast)
    }

    /// Whether the direction is diagonal on the grid, the odd directions of the game
    pub fn is_diagonal(self) -> bool {
        (self as u8) & 1 == 1
    }

    /// Grid number change of a step in this direction (`DirIncrementer`)
    pub fn grid_no_increment(self) -> i32 {
        match self {
            Direction::North => -WORLD_COLS,
            Direction::NorthEast => 1 - WORLD_COLS,
            Direction::East => 1,
            Direction::SouthEast => 1 + WORLD_COLS,
            Direction::South => WORLD_COLS,
            Direction::SouthWest => WORLD_COLS - 1,
            Direction::West => -1,
            Direction::NorthWest => -WORLD_COLS - 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid_no(index: usize) -> GridNo {
        GridNo::new(index).unwrap()
    }

    #[test]
    fn grid_numbers() {
        assert_eq!(GridNo::from(GridPosition::new(3, 2)), grid_no(323));
        assert_eq!(grid_no(323).position(), GridPosition::new(3, 2));
        assert_eq!(GridNo::new(JA2_MAP_TILES), None);
        assert_eq!(GridNo::from_i32(-1), None);
        assert_eq!(GridNo::from_i32(WORLD_MAX), None);
        assert_eq!(
            GridNo::from_i32(WORLD_MAX - 1),
            Some(grid_no(JA2_MAP_TILES - 1))
        );
        assert_eq!(GridNo::all().len(), JA2_MAP_TILES);
    }

    #[test]
    fn offsets_wrap_across_rows() {
        assert_eq!(grid_no(320).offset(-1), Some(grid_no(319)));
        assert_eq!(grid_no(5).offset(-WORLD_COLS), None);
        assert_eq!(grid_no(JA2_MAP_TILES - 5).offset(WORLD_COLS), None);
        assert_eq!(grid_no(0).offset(0), Some(grid_no(0)));
    }

    #[test]
    fn neighbours_do_not_wrap() {
        assert_eq!(grid_no(321).neighbour(-1, -1), Some(grid_no(160)));
        assert_eq!(grid_no(320).neighbour(-1, 0), None);
        assert_eq!(grid_no(159).neighbour(1, 0), None);
        assert_eq!(grid_no(5).neighbour(0, -1), None);
        assert_eq!(grid_no(5).neighbour(0, 2), Some(grid_no(325)));
    }

    #[test]
    fn steps_stay_on_the_map() {
        let step = |index, direction| grid_no(index).step(direction).index();
        // Inside of the map
        assert_eq!(step(161 * 2, Direction::North), 161 * 2 - 160);
        // Left edge
        assert_eq!(step(320, Direction::West), 320);
        assert_eq!(step(320, Direction::East), 321);
        // Right edge
        assert_eq!(step(479, Direction::East), 479);
        // Top edge
        assert_eq!(step(5, Direction::North), 5);
        // Bottom edge
        assert_eq!(step(JA2_MAP_TILES - 5, Direction::South), JA2_MAP_TILES - 5);
        // The game treats grid number 160 as an edge tile, but it still moves inside
        assert_eq!(step(161, Direction::North), 1);
    }

    #[test]
    fn grid_arrays() {
        let mut array = GridArray::filled(0u8);
        array[grid_no(7)] = 3;
        assert_eq!(array[grid_no(7)], 3);
        let set: Vec<_> = array.iter().filter(|(_, &v)| v != 0).collect();
        assert_eq!(set, vec![(grid_no(7), &3)]);
        let indices = GridArray::from_fn(|grid_no| grid_no.index());
        assert_eq!(indices[grid_no(JA2_MAP_TILES - 1)], JA2_MAP_TILES - 1);
    }
}
