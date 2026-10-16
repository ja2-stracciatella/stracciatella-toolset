//! Exit grids, the tiles that lead to another sector (`EXITGRID`)

use std::fmt;

use serde::{Deserialize, Serialize};

use super::GridNo;

/// A sector of the strategic map
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SectorId {
    /// Column, 1 to 16
    pub x: u8,
    /// Row, 1 (A) to 16 (P)
    pub y: u8,
    /// Underground level, 0 on the surface
    pub z: u8,
}

impl SectorId {
    /// The sector of a map file like `maps/a9.dat`, as the editor of the game reads it from the
    /// file name (`SetGlobalSectorValues`), `None` if the name doesn't start with a sector
    ///
    /// Like the game, anything may follow the sector, and only a lower case `_b1` to `_b3` is
    /// read as the level, so `H5_A.DAT` is H5 and `J11_B1.DAT` is J11 on the surface.
    pub fn from_map_path(path: &str) -> Option<Self> {
        let name = path.rsplit(['/', '\\']).next()?.as_bytes();
        let at = |index: usize| name.get(index).copied().unwrap_or(0);
        let y = match at(0) {
            row @ b'A'..=b'P' => row - b'A' + 1,
            row @ b'a'..=b'p' => row - b'a' + 1,
            _ => return None,
        };
        let (x, rest) = match (at(1), at(2)) {
            (digit @ b'1'..=b'9', next) if !next.is_ascii_digit() => (digit - b'0', 2),
            (b'1', digit @ b'0'..=b'6') => (10 + digit - b'0', 3),
            _ => return None,
        };
        let z = match (at(rest), at(rest + 1), at(rest + 2)) {
            (b'_', b'b', level @ b'1'..=b'3') => level - b'0',
            _ => 0,
        };
        Some(Self { x, y, z })
    }
}

/// `J11`, or `J11 B1` underground
impl fmt::Display for SectorId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let row = match self.y {
            1..=26 => char::from(b'A' + self.y - 1),
            _ => '?',
        };
        write!(f, "{}{}", row, self.x)?;
        if self.z > 0 {
            write!(f, " B{}", self.z)?;
        }
        Ok(())
    }
}

/// A tile that leads to another sector
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExitGrid {
    pub grid_no: GridNo,
    /// Tile in the destination sector
    pub dest_grid_no: GridNo,
    pub dest_sector: SectorId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sectors_are_read_from_map_names() {
        let sector = |x, y, z| Some(SectorId { x, y, z });
        assert_eq!(SectorId::from_map_path("maps/a9.dat"), sector(9, 1, 0));
        assert_eq!(SectorId::from_map_path("MAPS\\I14.DAT"), sector(14, 9, 0));
        assert_eq!(SectorId::from_map_path("j11_b1.dat"), sector(11, 10, 1));
        assert_eq!(SectorId::from_map_path("J11_B1.DAT"), sector(11, 10, 0));
        assert_eq!(SectorId::from_map_path("H5_A.DAT"), sector(5, 8, 0));
        assert_eq!(
            SectorId::from_map_path("maps/p16_b3.dat"),
            sector(16, 16, 3)
        );
        for other in [
            "maps/q1.dat",
            "maps/a17.dat",
            "maps/a0.dat",
            "maps/test.dat",
        ] {
            assert_eq!(SectorId::from_map_path(other), None, "{other}");
        }
        assert_eq!(sector(11, 10, 1).unwrap().to_string(), "J11 B1");
        assert_eq!(sector(9, 1, 0).unwrap().to_string(), "A9");
    }
}
