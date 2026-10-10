//! The tile database, one entry for every tile that a map can reference
//!
//! Port of `CreateTileDatabase` (`TileDef.cc`), `SetSpecificDatabaseValues` (`TileDat.cc`) and
//! `SetRaisedObjectFlag` (`Tile_Surface.cc`) in the game. It is independent of Bevy handles, the
//! loading code passes in the loaded tile surfaces. The runtime tiles are built from it by
//! [`crate::tileset::Tileset::from_database`].

use super::{
    ja2_stci::SubImageInfo,
    ja2_structure::{DbStructure, Ja2StructureFile, StructureFlags, ZStripInfo},
    ja2_tile_types::{
        tile_index, tile_type_start_index, Orientation, TerrainType, TileElementFlags, TileType,
        ANOTHERDEBRIS, BODYEXPLOSION, BULLETTILE, CLOSED_DOOR_LIST, CLOSED_DOOR_SHADOW_LIST,
        CREATUREBLOOD, DEBRIS2MISC, DEBRISWEEDS, DEBRISWOOD, DEEPWATERTEXTURE, FIRSTCLIFFHANG,
        FIRSTCLIFFSHADOW, FIRSTDOOR, FIRSTEXPLDEBRIS, FIRSTFLOOR, FIRSTFULLSTRUCT, FIRSTHIGHROOF,
        FIRSTMISS, FIRSTROOF, FIRSTWALL, FOOTPRINTS, FOURTHDOOR, FOURTHDOORSHADOW, FOURTHFLOOR,
        FOURTHWALL, FULL_BASE_TILE_VALUES, GUNS, HUMANBLOOD, LAST_UI_ELEMENT, MOCKFLOOR,
        NUMBER_OF_TILES, NUMBER_OF_TILE_TYPES, NUM_TILES_PER_TYPE, OPEN_DOOR_LIST,
        OPEN_DOOR_SHADOW_LIST, P2ITEMS, P3ITEMS, REGWATERTEXTURE, SECONDEXPLDEBRIS,
        SECONDFULLSTRUCT, SECONDHIGHROOF, SECONDSLANTROOF, SHADOW_BUDDIES, THIRDPOINTERS,
        WALL_AFRAME_END, WALL_AFRAME_START, WINDOWSHATTER,
    },
    stci::{StciAppData, StciAppDataFlags},
};
use crate::tileset::TilesetId;

/// Armour value of plywood walls (`MATERIAL_PLYWOOD_WALL`)
const MATERIAL_PLYWOOD_WALL: u8 = 2;

/// Number of frames of the door, window and miss animations that the database sets up
const SPECIFIC_ANIMATION_FRAMES: u16 = 5;

/// Debris files whose objects use the object z height (`SetRaisedObjectFlag`)
const RAISED_OBJECT_FILES: [&str; 11] = [
    "bones", "bones2", "grass2", "grass3", "l_weed3", "litter", "miniweed", "sblast", "sweeds",
    "twigs", "wing",
];

/// A loaded tile surface (STI and optional JSD) of one tile type, the input of the database
#[derive(Clone, Debug)]
pub struct TileSurfaceInfo {
    /// Tileset the file was loaded from, differs from the map tileset for default surfaces
    pub tileset_id: TilesetId,
    pub filename: String,
    /// One entry per sub image (region) of the STI
    pub sub_images: Vec<SubImageInfo>,
    pub structure: Option<Ja2StructureFile>,
    /// Terrain of the tile type in the map tileset
    pub terrain: TerrainType,
}

/// A tile surface in the database
#[derive(Clone, Debug)]
pub struct TileSurface {
    pub tileset_id: TilesetId,
    pub filename: String,
    /// The structure data, including the database fixes
    pub structure: Option<Ja2StructureFile>,
}

/// One tile of the database (`TILE_ELEMENT`)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileElement {
    pub tile_type: TileType,
    /// Sub image of the surface, 0 for underflow fillers
    pub region_index: u16,
    pub flags: TileElementFlags,
    pub terrain: TerrainType,
    pub wall_orientation: Orientation,
    /// Whether the tile covers its whole tile (for land tiles)
    pub full_tile: bool,
    /// Tiles (relative to this one) that the sub image covers
    pub tile_locs: Vec<(i8, i8)>,
    /// Shadow of a structure or structure of a shadow
    pub buddy: Option<u16>,
    /// Database indices of the animation frames, empty for static tiles
    pub anim_frames: Vec<u16>,
    pub current_frame: u8,
    /// Index of the structure in the structure file of the surface
    pub structure: Option<u16>,
    pub z_strip: Option<ZStripInfo>,
}

/// The tile database of a map, with exactly `NUMBER_OF_TILES` entries
#[derive(Clone, Debug)]
pub struct Ja2TileDatabase {
    surfaces: Vec<Option<TileSurface>>,
    elements: Vec<TileElement>,
}

impl Ja2TileDatabase {
    /// Builds the database from one optional surface per tile type (`CreateTileDatabase`)
    pub fn build(surfaces: Vec<Option<TileSurfaceInfo>>) -> Result<Self, String> {
        if surfaces.len() != NUMBER_OF_TILE_TYPES {
            return Err(format!(
                "expected {} tile surfaces, got {}",
                NUMBER_OF_TILE_TYPES,
                surfaces.len()
            ));
        }

        let mut elements: Vec<TileElement> = Vec::with_capacity(NUMBER_OF_TILES);
        let mut database_surfaces = Vec::with_capacity(NUMBER_OF_TILE_TYPES);
        for (tile_type, surface) in surfaces.into_iter().enumerate() {
            debug_assert_eq!(
                elements.len(),
                usize::from(tile_type_start_index(tile_type))
            );
            let num_tiles = usize::from(NUM_TILES_PER_TYPE[tile_type]);
            let Some(mut surface) = surface else {
                // The game fails to load such a tileset, keep the indices of the other types
                elements.extend((0..num_tiles).map(|_| underflow_filler(tile_type)));
                database_surfaces.push(None);
                continue;
            };

            if let Some(structure) = &surface.structure {
                if usize::from(structure.number_of_structures) != surface.sub_images.len() {
                    return Err(format!(
                        "structure file of {} (tileset {}) describes {} sub images, the image has {}",
                        surface.filename,
                        surface.tileset_id,
                        structure.number_of_structures,
                        surface.sub_images.len()
                    ));
                }
            }
            let z_strips = surface
                .structure
                .as_ref()
                .map(|structure| structure.z_strip_info(&surface.sub_images))
                .unwrap_or_default();
            let raised_object = is_raised_object(tile_type, &surface.filename);

            // Aux data from the JSD, or from the STI if every sub image has app data
            let (aux_data, tile_locs) = match &surface.structure {
                Some(structure) if !structure.aux_data.is_empty() => {
                    (structure.aux_data.clone(), structure.tile_locs.clone())
                }
                _ => (
                    surface
                        .sub_images
                        .iter()
                        .map(|sub_image| sub_image.app_data.clone())
                        .collect::<Option<Vec<_>>>()
                        .unwrap_or_default(),
                    vec![],
                ),
            };

            // Overflow is cut off
            let num_regions = surface.sub_images.len().min(num_tiles);
            for region in 0..num_regions {
                let database_index = elements.len() as u16;
                let mut element = TileElement {
                    tile_type,
                    region_index: region as u16,
                    terrain: surface.terrain,
                    ..Default::default()
                };
                if let Some(Some(z_strip)) = z_strips.get(region) {
                    element.flags |= TileElementFlags::MULTI_Z_TILE;
                    element.z_strip = Some(z_strip.clone());
                }
                if let Some(structure) = &surface.structure {
                    if matches!(structure.structures.get(region), Some(Some(_))) {
                        element.structure = Some(region as u16);
                    }
                }
                if let Some(aux) = aux_data.get(region) {
                    apply_aux_data(&mut element, aux, &tile_locs, database_index);
                }

                set_specific_database_values(
                    &mut element,
                    database_index,
                    raised_object,
                    &elements,
                );
                if region == 0 && (FIRSTDOOR..=FOURTHDOOR).contains(&tile_type) {
                    if let Some(structure) = &mut surface.structure {
                        fix_double_door(&mut element, structure);
                    }
                }
                elements.push(element);
            }
            // Underflow is padded with fillers that show the first region
            elements.extend((num_regions..num_tiles).map(|_| underflow_filler(tile_type)));

            database_surfaces.push(Some(TileSurface {
                tileset_id: surface.tileset_id,
                filename: surface.filename,
                structure: surface.structure,
            }));
        }
        debug_assert_eq!(elements.len(), NUMBER_OF_TILES);

        Ok(Self {
            surfaces: database_surfaces,
            elements,
        })
    }

    /// Database index of a tile in a map file (`GetTileIndexFromTypeSubIndex`)
    ///
    /// Like in the game, sub indices that are out of range for their type spill into the next
    /// type. Returns `None` for sub index 0, for indices after the last tile and for tiles of
    /// types that could not be loaded.
    pub fn index(&self, tile_type: TileType, sub_index: u16) -> Option<u16> {
        if tile_type >= NUMBER_OF_TILE_TYPES || sub_index == 0 {
            return None;
        }
        let index = usize::from(tile_type_start_index(tile_type)) + usize::from(sub_index) - 1;
        self.element(u16::try_from(index).ok()?)?;
        Some(index as u16)
    }

    /// A tile of the database, `None` if its type could not be loaded
    pub fn element(&self, index: u16) -> Option<&TileElement> {
        let element = self.elements.get(usize::from(index))?;
        self.surface(element.tile_type)?;
        Some(element)
    }

    /// The surface of a tile type, `None` if it could not be loaded
    pub fn surface(&self, tile_type: TileType) -> Option<&TileSurface> {
        self.surfaces.get(tile_type)?.as_ref()
    }

    /// Structure data of a tile, if it has any
    pub fn structure(&self, index: u16) -> Option<&DbStructure> {
        let element = self.element(index)?;
        let structure_file = self.surface(element.tile_type)?.structure.as_ref()?;
        structure_file
            .structures
            .get(usize::from(element.structure?))?
            .as_ref()
    }

    /// The surfaces (one per tile type, `None` if not loaded) and the tiles of the database
    pub fn into_parts(self) -> (Vec<Option<TileSurface>>, Vec<TileElement>) {
        (self.surfaces, self.elements)
    }
}

fn underflow_filler(tile_type: TileType) -> TileElement {
    TileElement {
        tile_type,
        flags: TileElementFlags::UNDERFLOW_FILLER,
        ..Default::default()
    }
}

/// Port of `SetRaisedObjectFlag`
fn is_raised_object(tile_type: TileType, filename: &str) -> bool {
    if ![DEBRISWOOD, DEBRISWEEDS, DEBRIS2MISC, ANOTHERDEBRIS].contains(&tile_type) {
        return false;
    }
    let file = filename.rsplit(['/', '\\']).next().unwrap_or(filename);
    let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
    RAISED_OBJECT_FILES
        .iter()
        .any(|raised| raised.eq_ignore_ascii_case(stem))
}

fn apply_aux_data(
    element: &mut TileElement,
    aux: &StciAppData,
    tile_locs: &[(i8, i8)],
    database_index: u16,
) {
    if aux.flags.contains(StciAppDataFlags::FULL_TILE) {
        element.full_tile = true;
    }
    if aux.flags.contains(StciAppDataFlags::ANIMATED_TILE) {
        element.current_frame = aux.current_frame;
        let first = database_index.wrapping_sub(u16::from(aux.current_frame));
        element.anim_frames = (0..u16::from(aux.number_of_frames))
            .map(|frame| first.wrapping_add(frame))
            .collect();
        element.flags |= TileElementFlags::ANIMATED_TILE;
    }
    element.wall_orientation = Orientation::from_bits_retain(aux.wall_orientation);
    if aux.number_of_tiles > 0 {
        let start = usize::from(aux.tile_location_index);
        let end = start + usize::from(aux.number_of_tiles);
        element.tile_locs = tile_locs.get(start..end).unwrap_or_default().to_vec();
    }
}

/// Animation of 5 frames starting at the tile, forwards or backwards (`SetAnimData`)
fn set_anim_data(element: &mut TileElement, database_index: u16, forwards: bool) {
    element.current_frame = 0;
    element.anim_frames = (0..SPECIFIC_ANIMATION_FRAMES)
        .map(|frame| {
            if forwards {
                database_index + frame
            } else {
                database_index - frame
            }
        })
        .collect();
}

/// Port of `SetSpecificDatabaseValues`, `elements` are the tiles before this one
fn set_specific_database_values(
    element: &mut TileElement,
    database_index: u16,
    raised_object: bool,
    elements: &[TileElement],
) {
    let tile_type = element.tile_type;
    let index = database_index;

    // Buddies of structures and shadows
    for buddies in &SHADOW_BUDDIES {
        let structure_start = tile_type_start_index(buddies.structure);
        let shadow_start = tile_type_start_index(buddies.shadow);
        if tile_type == buddies.shadow {
            let buddy = index - shadow_start + structure_start;
            element.buddy = Some(buddy);
            // Take the flags of the structure
            if let Some(structure) = elements.get(usize::from(buddy)) {
                element.flags |= structure.flags;
            }
            break;
        } else if tile_type == buddies.structure {
            element.buddy = Some(index - structure_start + shadow_start);
            element.flags |= TileElementFlags::HAS_SHADOW_BUDDY;
            break;
        }
    }

    // Door animations
    if (tile_index(FIRSTDOOR, 1)..=tile_index(FOURTHDOORSHADOW, 20)).contains(&index) {
        if OPEN_DOOR_LIST.contains(&index) || OPEN_DOOR_SHADOW_LIST.contains(&index) {
            set_anim_data(element, index, true);
        }
        if CLOSED_DOOR_LIST.contains(&index) || CLOSED_DOOR_SHADOW_LIST.contains(&index) {
            set_anim_data(element, index, false);
        }
    }

    if index == tile_index(FIRSTMISS, 1) {
        set_anim_data(element, index, true);
    }
    if (tile_index(FIRSTMISS, 1)..=tile_index(FIRSTMISS, 5)).contains(&index) {
        element.flags |= TileElementFlags::DYNAMIC_TILE;
    }

    if [1, 6, 11, 16]
        .map(|n| tile_index(WINDOWSHATTER, n))
        .contains(&index)
    {
        set_anim_data(element, index, true);
    }
    if (tile_index(WINDOWSHATTER, 1)..=tile_index(WINDOWSHATTER, 20)).contains(&index) {
        element.flags |= TileElementFlags::DYNAMIC_TILE;
    }

    if tile_type == BODYEXPLOSION {
        element.flags |= TileElementFlags::DYNAMIC_TILE;
    }

    // Object pieces which use proper z
    if raised_object {
        element.flags |= TileElementFlags::OBJECTLAYER_USEZHEIGHT;
    }

    // Full 3D tiles, and the dynamic flag for the foliage
    if (tile_index(FIRSTFULLSTRUCT, 1)..=tile_index(SECONDFULLSTRUCT, 12)).contains(&index) {
        match (index - tile_type_start_index(tile_type)) % 3 {
            0 => element.flags |= TileElementFlags::FULL3D_TILE,
            1 => element.flags |= TileElementFlags::DYNAMIC_TILE,
            _ => {}
        }
    }

    // Cliffs ignore the height, if the land is raised the cliff is not offset
    if (tile_index(FIRSTCLIFFHANG, 1)..=tile_index(FIRSTCLIFFSHADOW, 17)).contains(&index) {
        if tile_type == FIRSTCLIFFHANG {
            element.flags |= TileElementFlags::CLIFFHANG_TILE;
        }
        element.flags |= TileElementFlags::IGNORE_WORLD_HEIGHT;
    }

    if (tile_index(FIRSTWALL, 1)..=tile_index(FOURTHWALL, 65)).contains(&index) {
        element.flags |= TileElementFlags::WALL_TILE;
    }

    // A-frames are drawn higher (for roof pieces)
    if (tile_index(FIRSTWALL, 1)..=tile_index(FOURTHWALL, 47)).contains(&index) {
        let start = tile_type_start_index(tile_type);
        if (start + WALL_AFRAME_START..=start + WALL_AFRAME_END).contains(&index) {
            element.flags |= TileElementFlags::AFRAME_TILE;
        }
    }

    // UI elements are dynamic
    if (tile_index(FOOTPRINTS, 1)..=tile_index(THIRDPOINTERS, 2)).contains(&index) {
        element.flags |= TileElementFlags::DYNAMIC_TILE | TileElementFlags::OBJECTLAYER_USEZHEIGHT;
    }

    // UI elements, blood, items and explosion debris use the object z level
    if (FOOTPRINTS..=LAST_UI_ELEMENT).contains(&tile_type)
        || (HUMANBLOOD..=CREATUREBLOOD).contains(&tile_type)
        || (GUNS..=P2ITEMS).contains(&tile_type)
        || tile_type == P3ITEMS
        || (FIRSTEXPLDEBRIS..=SECONDEXPLDEBRIS).contains(&tile_type)
    {
        element.flags |= TileElementFlags::OBJECTLAYER_USEZHEIGHT;
    }

    if index == tile_index(MOCKFLOOR, 1) || tile_type == BULLETTILE {
        element.flags |= TileElementFlags::DYNAMIC_TILE;
    }

    // Floors are full tiles
    if (tile_index(FIRSTFLOOR, 1)..=tile_index(FOURTHFLOOR, 8)).contains(&index) {
        element.full_tile = true;
    }

    // Fullness of the land tiles
    if index <= tile_index(DEEPWATERTEXTURE, 10) {
        element.full_tile = FULL_BASE_TILE_VALUES[usize::from(index)] != 0;
    }

    // The game compares the tile index to the tile type `REGWATERTEXTURE` here
    if (tile_index(REGWATERTEXTURE, 18)..=tile_index(REGWATERTEXTURE, 50)).contains(&index)
        || index == REGWATERTEXTURE as u16
        || [12, 14, 16]
            .map(|n| tile_index(REGWATERTEXTURE, n))
            .contains(&index)
    {
        element.terrain = TerrainType::FlatGround;
    }

    if (FIRSTROOF..=SECONDSLANTROOF).contains(&tile_type)
        || tile_type == FIRSTHIGHROOF
        || tile_type == SECONDHIGHROOF
    {
        element.flags |= TileElementFlags::ROOF_TILE;
    }
}

/// Fixes incorrect double door flags, which only affects DOOR3 in the PALACE! tileset and DOOR1
/// in QUEEN'S TROPICAL in vanilla. `element` is the first tile of a door type.
fn fix_double_door(element: &mut TileElement, structure_file: &mut Ja2StructureFile) {
    let structures = &mut structure_file.structures;
    let Some(Some(first)) = structures.first() else {
        return;
    };
    if !first
        .flags
        .intersects(StructureFlags::DDOOR_RIGHT | StructureFlags::DDOOR_LEFT)
    {
        return;
    }
    // A door in an open state that takes up 1 tile and is flagged as outside oriented
    let open_door_tiles = structures
        .get(4)
        .and_then(Option::as_ref)
        .map(|structure| structure.tiles.len());
    if element.wall_orientation != Orientation::OUTSIDE_RIGHT || open_door_tiles != Some(1) {
        return;
    }
    // The game expects all these structures to exist
    if [0, 4, 5, 9, 10, 14, 15, 19]
        .iter()
        .any(|&i| !matches!(structures.get(i), Some(Some(_))))
    {
        return;
    }
    fn structure(structures: &mut [Option<DbStructure>], i: usize) -> &mut DbStructure {
        structures[i].as_mut().expect("checked above")
    }

    element.wall_orientation = Orientation::INSIDE_RIGHT;
    structure(structures, 0).wall_orientation = Orientation::INSIDE_RIGHT;
    structure(structures, 4).wall_orientation = Orientation::INSIDE_RIGHT;
    structure(structures, 4).armour = MATERIAL_PLYWOOD_WALL;
    structure(structures, 5).wall_orientation = Orientation::INSIDE_LEFT;
    structure(structures, 9).wall_orientation = Orientation::INSIDE_LEFT;
    // Between sub indices 0-4 and 10-14 there can only be 1 right part of a double door
    for i in [10, 14] {
        structure(structures, i)
            .flags
            .remove(StructureFlags::DDOOR_RIGHT);
        structure(structures, i)
            .flags
            .insert(StructureFlags::DDOOR_LEFT);
    }
    // Between sub indices 5-9 and 15-19 there can only be 1 left part of a double door
    for i in [15, 19] {
        structure(structures, i)
            .flags
            .remove(StructureFlags::DDOOR_LEFT);
        structure(structures, i)
            .flags
            .insert(StructureFlags::DDOOR_RIGHT);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::file_formats::{
        ja2_structure::tests::{aux_bytes, jsd_bytes, structure_bytes},
        ja2_tile_types::{
            FIRSTCLIFF, FIRSTOSTRUCT, FIRSTSHADOW, FIRSTTEXTURE, SECONDTEXTURE, THIRDTEXTURE,
            WIREFRAMES,
        },
    };

    fn sub_images(count: usize) -> Vec<SubImageInfo> {
        (0..count)
            .map(|_| SubImageInfo {
                offset: (0, 0),
                dimensions: (40, 20),
                app_data: None,
            })
            .collect()
    }

    pub(crate) fn surface(filename: &str, count: usize) -> TileSurfaceInfo {
        TileSurfaceInfo {
            tileset_id: TilesetId::new(0),
            filename: filename.to_owned(),
            sub_images: sub_images(count),
            structure: None,
            terrain: TerrainType::NoTerrain,
        }
    }

    /// All surfaces loaded, each type with exactly its number of tiles
    pub(crate) fn full_surfaces() -> Vec<Option<TileSurfaceInfo>> {
        (0..NUMBER_OF_TILE_TYPES)
            .map(|t| {
                Some(surface(
                    &format!("type{}.sti", t),
                    usize::from(NUM_TILES_PER_TYPE[t]),
                ))
            })
            .collect()
    }

    fn build(surfaces: Vec<Option<TileSurfaceInfo>>) -> Ja2TileDatabase {
        Ja2TileDatabase::build(surfaces).unwrap()
    }

    /// A database with every tile type loaded and every tile a 40×20 image
    pub(crate) fn test_database() -> Ja2TileDatabase {
        build(full_surfaces())
    }

    #[test]
    fn indices_are_fixed() {
        let mut surfaces = full_surfaces();
        // Too many and too few regions don't change the start indices
        surfaces[FIRSTTEXTURE] = Some(surface("a.sti", 50));
        surfaces[SECONDTEXTURE] = Some(surface("b.sti", 3));
        let db = build(surfaces);
        assert_eq!(db.index(FIRSTCLIFFHANG, 1), Some(348));
        assert_eq!(db.index(FIRSTDOOR, 1), Some(907));
        assert_eq!(db.index(WIREFRAMES, 15), Some(NUMBER_OF_TILES as u16 - 1));
        assert_eq!(db.index(WIREFRAMES, 16), None);
        assert_eq!(db.index(FIRSTTEXTURE, 0), None);
        assert_eq!(db.index(NUMBER_OF_TILE_TYPES, 1), None);
    }

    #[test]
    fn overflow_and_underflow() {
        let mut surfaces = full_surfaces();
        surfaces[FIRSTTEXTURE] = Some(surface("a.sti", 50));
        surfaces[SECONDTEXTURE] = Some(surface("b.sti", 3));
        let db = build(surfaces);

        // Overflow is cut off, the index spills into the next type
        let spilled = db.element(db.index(FIRSTTEXTURE, 36).unwrap()).unwrap();
        assert_eq!(spilled.tile_type, SECONDTEXTURE);
        assert_eq!(spilled.region_index, 0);
        assert!(!spilled.flags.contains(TileElementFlags::UNDERFLOW_FILLER));

        let last = db.element(db.index(SECONDTEXTURE, 3).unwrap()).unwrap();
        assert_eq!(last.region_index, 2);
        assert!(!last.flags.contains(TileElementFlags::UNDERFLOW_FILLER));
        let filler = db.element(db.index(SECONDTEXTURE, 4).unwrap()).unwrap();
        assert_eq!(filler.tile_type, SECONDTEXTURE);
        assert_eq!(filler.region_index, 0);
        assert_eq!(filler.flags, TileElementFlags::UNDERFLOW_FILLER);
    }

    #[test]
    fn missing_surfaces() {
        let mut surfaces = full_surfaces();
        surfaces[SECONDTEXTURE] = None;
        let db = build(surfaces);
        assert!(db.surface(SECONDTEXTURE).is_none());
        assert_eq!(db.index(SECONDTEXTURE, 1), None);
        assert_eq!(db.index(FIRSTTEXTURE, 36), None);
        assert_eq!(db.index(THIRDTEXTURE, 1), Some(70));

        assert!(Ja2TileDatabase::build(vec![]).is_err());
    }

    #[test]
    fn specific_flags() {
        let db = build(full_surfaces());
        let flags = |tile_type, sub_index| {
            db.element(db.index(tile_type, sub_index).unwrap())
                .unwrap()
                .flags
        };
        assert!(flags(FIRSTWALL, 1).contains(TileElementFlags::WALL_TILE));
        assert!(flags(FOURTHWALL, 65).contains(TileElementFlags::WALL_TILE));
        assert!(!flags(FIRSTWALL, 31).contains(TileElementFlags::AFRAME_TILE));
        assert!(flags(FIRSTWALL, 32).contains(TileElementFlags::AFRAME_TILE));
        assert!(flags(FIRSTWALL, 35).contains(TileElementFlags::AFRAME_TILE));
        assert!(!flags(FIRSTWALL, 36).contains(TileElementFlags::AFRAME_TILE));
        assert!(flags(FIRSTROOF, 1).contains(TileElementFlags::ROOF_TILE));
        assert!(flags(SECONDHIGHROOF, 1).contains(TileElementFlags::ROOF_TILE));
        assert!(flags(FIRSTFULLSTRUCT, 1).contains(TileElementFlags::FULL3D_TILE));
        assert!(flags(FIRSTFULLSTRUCT, 2).contains(TileElementFlags::DYNAMIC_TILE));
        assert!(!flags(FIRSTFULLSTRUCT, 3)
            .intersects(TileElementFlags::FULL3D_TILE | TileElementFlags::DYNAMIC_TILE));
        assert!(flags(FIRSTFULLSTRUCT, 4).contains(TileElementFlags::FULL3D_TILE));
        assert!(flags(FIRSTCLIFFHANG, 1)
            .contains(TileElementFlags::CLIFFHANG_TILE | TileElementFlags::IGNORE_WORLD_HEIGHT));
        assert!(flags(FIRSTCLIFF, 1).contains(TileElementFlags::IGNORE_WORLD_HEIGHT));
        assert!(!flags(FIRSTCLIFF, 1).contains(TileElementFlags::CLIFFHANG_TILE));
        assert!(flags(FOOTPRINTS, 1).contains(TileElementFlags::DYNAMIC_TILE));
        assert!(flags(GUNS, 1).contains(TileElementFlags::OBJECTLAYER_USEZHEIGHT));

        let element =
            |tile_type, sub_index| db.element(db.index(tile_type, sub_index).unwrap()).unwrap();
        assert!(element(FIRSTTEXTURE, 1).full_tile);
        assert!(!element(FIRSTTEXTURE, 11).full_tile);
        assert!(element(FIRSTFLOOR, 3).full_tile);
        assert_eq!(
            element(REGWATERTEXTURE, 20).terrain,
            TerrainType::FlatGround
        );

        let door = element(FIRSTDOOR, 1);
        assert_eq!(door.anim_frames, vec![907, 908, 909, 910, 911]);
        let closed = element(FIRSTDOOR, 5);
        assert_eq!(closed.anim_frames, vec![911, 910, 909, 908, 907]);
    }

    #[test]
    fn shadow_buddies() {
        let mut surfaces = full_surfaces();
        let structures = [structure_bytes(
            0,
            StructureFlags::GENERIC,
            0,
            &[(0, 0, 0), (1, 0, 0)],
        )];
        let mut ostruct = surface("ostruct.sti", 12);
        ostruct.sub_images[0].offset = (-30, 0);
        ostruct.sub_images[0].dimensions = (70, 50);
        ostruct.structure =
            Some(Ja2StructureFile::parse(&jsd_bytes(12, None, &structures)).unwrap());
        surfaces[FIRSTOSTRUCT] = Some(ostruct);
        let db = build(surfaces);

        let structure_index = db.index(FIRSTOSTRUCT, 1).unwrap();
        let shadow_index = db.index(FIRSTSHADOW, 1).unwrap();
        let structure = db.element(structure_index).unwrap();
        let shadow = db.element(shadow_index).unwrap();
        assert_eq!(structure.buddy, Some(shadow_index));
        assert_eq!(shadow.buddy, Some(structure_index));
        assert!(structure.flags.contains(TileElementFlags::HAS_SHADOW_BUDDY));
        assert!(structure.flags.contains(TileElementFlags::MULTI_Z_TILE));
        assert!(structure.z_strip.is_some());
        // The shadow takes the flags of the structure
        assert!(shadow
            .flags
            .contains(TileElementFlags::HAS_SHADOW_BUDDY | TileElementFlags::MULTI_Z_TILE));
        assert_eq!(structure.structure, Some(0));
        assert_eq!(db.structure(structure_index).unwrap().tiles.len(), 2);
        assert_eq!(db.element(structure_index + 1).unwrap().structure, None);
    }

    #[test]
    fn aux_data() {
        let mut surfaces = full_surfaces();
        let aux: Vec<_> = (0..12)
            .map(|i| aux_bytes(0, if i == 0 { 2 } else { 0 }, 1))
            .collect();
        let mut ostruct = surface("ostruct.sti", 12);
        ostruct.structure = Some(
            Ja2StructureFile::parse(&jsd_bytes(12, Some((&aux, &[(0, 0), (1, 0), (0, 1)])), &[]))
                .unwrap(),
        );
        surfaces[FIRSTOSTRUCT] = Some(ostruct);
        let db = build(surfaces);
        let index = db.index(FIRSTOSTRUCT, 2).unwrap();
        let element = db.element(index).unwrap();
        assert!(element.flags.contains(TileElementFlags::ANIMATED_TILE));
        // Current frame 1 of 3
        assert_eq!(element.anim_frames, vec![index - 1, index, index + 1]);
        let first = db.element(index - 1).unwrap();
        assert_eq!(first.tile_locs, vec![(1, 0), (0, 1)]);
    }

    #[test]
    fn structure_count_must_match() {
        let mut surfaces = full_surfaces();
        let mut ostruct = surface("ostruct.sti", 12);
        ostruct.structure = Some(Ja2StructureFile::parse(&jsd_bytes(11, None, &[])).unwrap());
        surfaces[FIRSTOSTRUCT] = Some(ostruct);
        assert!(Ja2TileDatabase::build(surfaces).is_err());
    }

    /// Builds the database of every tileset from the extracted game data in `JA2_DATA_DIR` (with
    /// lowercase paths: `binarydata/ja2set.dat`, `tilesets/<id>/*.sti|jsd`, `maps/*.dat`) and
    /// looks up every tile of every map
    #[test]
    fn build_from_data_dir() {
        use crate::file_formats::{
            ja2_map::Ja2Map, ja2_tile_types::FIRSTPOINTERS, ja2_tileset::Ja2Tilesets, stci::Stci,
        };
        use std::{collections::HashMap, fs, path::Path};

        let Ok(dir) = std::env::var("JA2_DATA_DIR") else {
            eprintln!("JA2_DATA_DIR not set, skipping");
            return;
        };
        let dir = Path::new(&dir);
        let tilesets =
            Ja2Tilesets::parse(&fs::read(dir.join("binarydata/ja2set.dat")).unwrap()).unwrap();

        let mut failures = vec![];
        let mut databases = HashMap::new();
        for tileset_id in tilesets.ids() {
            let tileset = tilesets.get(tileset_id.index()).unwrap();
            let surfaces = (0..NUMBER_OF_TILE_TYPES)
                .map(|tile_type| {
                    let resolved = tilesets.resolve_tile_surface(tileset_id, tile_type)?;
                    let file = dir
                        .join(format!("tilesets/{}", resolved.tileset_id))
                        .join(resolved.filename.to_lowercase());
                    let bytes = fs::read(&file)
                        .map_err(|e| failures.push(format!("{}: {}", file.display(), e)))
                        .ok()?;
                    let stci = Stci::from_input(&mut std::io::Cursor::new(bytes))
                        .map_err(|e| failures.push(format!("{}: {}", file.display(), e)))
                        .ok()?;
                    let jsd = file.with_extension("jsd");
                    let structure = fs::read(&jsd).ok().and_then(|bytes| {
                        Ja2StructureFile::parse(&bytes)
                            .map_err(|e| failures.push(format!("{}: {}", jsd.display(), e)))
                            .ok()
                    });
                    Some(TileSurfaceInfo {
                        tileset_id: resolved.tileset_id,
                        filename: resolved.filename,
                        sub_images: SubImageInfo::from_stci(&stci),
                        structure,
                        terrain: tileset.terrain_id(tile_type),
                    })
                })
                .collect();
            match Ja2TileDatabase::build(surfaces) {
                Ok(database) => {
                    databases.insert(tileset_id, database);
                }
                Err(e) => failures.push(format!("tileset {}: {}", tileset_id, e)),
            }
        }

        let mut missing = vec![];
        let mut maps = 0;
        let mut tiles = 0;
        let mut fillers = 0;
        let mut spilled = 0;
        for entry in fs::read_dir(dir.join("maps")).unwrap() {
            let path = entry.unwrap().path();
            let Ok(map) = Ja2Map::parse(&fs::read(&path).unwrap()) else {
                continue;
            };
            let Some(database) = databases.get(&TilesetId::new(map.tileset_id as u8)) else {
                failures.push(format!("{}: no database", path.display()));
                continue;
            };
            maps += 1;
            for element in &map.map_elements {
                let objects = element
                    .objects
                    .iter()
                    .filter(|o| usize::from(o.kind) < FIRSTPOINTERS);
                for layer in element.land.iter().chain(objects).chain(&element.structs) {
                    tiles += 1;
                    let Some(index) = database.index(layer.kind.into(), layer.sub_index) else {
                        missing.push(format!(
                            "{}: kind {} sub index {}",
                            path.display(),
                            layer.kind,
                            layer.sub_index
                        ));
                        continue;
                    };
                    let element = database.element(index).unwrap();
                    if element.flags.contains(TileElementFlags::UNDERFLOW_FILLER) {
                        fillers += 1;
                    }
                    if element.tile_type != usize::from(layer.kind) {
                        spilled += 1;
                    }
                }
            }
        }
        eprintln!(
            "built {} databases, looked up {} tiles of {} maps: {} underflow fillers, {} in \
             the next type, {} missing",
            databases.len(),
            tiles,
            maps,
            fillers,
            spilled,
            missing.len()
        );
        assert!(failures.is_empty(), "failures:\n{}", failures.join("\n"));
        assert!(missing.is_empty(), "missing tiles:\n{}", missing.join("\n"));
    }

    #[test]
    fn raised_objects() {
        assert!(is_raised_object(DEBRISWOOD, "TWIGS.STI"));
        assert!(is_raised_object(ANOTHERDEBRIS, "tilesets/0/bones.sti"));
        assert!(!is_raised_object(DEBRISWOOD, "twigs2.sti"));
        assert!(!is_raised_object(FIRSTOSTRUCT, "twigs.sti"));
    }
}
