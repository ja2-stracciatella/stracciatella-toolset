//! Builds the tileset from the tile database of the game

use std::{collections::HashMap, sync::Arc};

use super::{
    BuildingPalette, BuildingPiece, BuildingTile, DoorTile, Orientation, RoofKind, RoofPiece,
    SlantPiece, SlantSide, StructureFlags, StructureTile, SurfaceId, Terrain, TerrainTile, Tile,
    TileAnimation, TileCategory, TileImage, TileIndex, TileKind, TileStructure, Tileset,
    TilesetSurface, ViewLayer, WallPiece, WallTile, ZStrip, NUM_ROAD_MACROS,
};
use crate::file_formats::{
    ja2_structure::{DbStructure, Ja2StructureFile, StructureTileFlags, ZStripInfo},
    ja2_tile_database::{Ja2TileDatabase, TileElement},
    ja2_tile_types::{
        self, tile_type_start_index, TerrainType, TileElementFlags, TileType, ANIOSTRUCT,
        ANOTHERDEBRIS, BODYEXPLOSION, DEBRIS2MISC, DEBRISGRASS, DEBRISMISC, DEBRISROCKS,
        DEBRISSAND, DEBRISWEEDS, DEBRISWOOD, DEEPWATERTEXTURE, EIGHTISTRUCT, EIGHTOSTRUCT,
        EIGTHWALLDECAL, FENCESTRUCT, FIFTHISTRUCT, FIFTHOSTRUCT, FIFTHWALLDECAL, FIRSTCISTRUCT,
        FIRSTCLIFF, FIRSTCLIFFHANG, FIRSTCLIFFSHADOW, FIRSTDEBRISSTRUCT, FIRSTDECORATIONS,
        FIRSTDOOR, FIRSTDOORSHADOW, FIRSTFLOOR, FIRSTFULLSTRUCT, FIRSTHIGHROOF, FIRSTISTRUCT,
        FIRSTLARGEEXPDEBRIS, FIRSTONROOF, FIRSTOSTRUCT, FIRSTROAD, FIRSTROOF, FIRSTSHADOW,
        FIRSTSLANTROOF, FIRSTSWITCHES, FIRSTTEXTURE, FIRSTVEHICLE, FIRSTWALL, FIRSTWALLDECAL,
        FOOTPRINTS, FOURTHDECORATIONS, FOURTHDOOR, FOURTHDOORSHADOW, FOURTHFLOOR, FOURTHFULLSHADOW,
        FOURTHFULLSTRUCT, FOURTHOSTRUCT, FOURTHROOF, FOURTHWALL, FOURTHWALLDECAL, FOURTHWINDOW,
        GUNS, NINTHOSTRUCT, NUMBER_OF_TILE_TYPES, NUM_TILES_PER_TYPE, P2ITEMS, P3ITEMS,
        REGWATERTEXTURE, REVEALEDSLANTROOFS, ROADPIECES, SECONDDEBRISSTRUCT, SECONDFULLSTRUCT,
        SECONDHIGHROOF, SECONDLARGEEXPDEBRIS, SECONDONROOF, SECONDOSTRUCT, SECONDREVEALEDHIGHROOFS,
        SECONDSLANTROOF, SECONDVEHICLE, SEVENTHOSTRUCT, SEVENTHTEXTURE, SHADOW_BUDDIES,
        SIXTHOSTRUCT, SLANTROOFCEILING, TENTHOSTRUCT, THIRDOSTRUCT, TILE_TYPE_NAMES, WINDOWSHATTER,
        WIREFRAMES,
    },
};

impl Tileset {
    /// Builds the tiles of a tile database
    pub fn from_database(database: Ja2TileDatabase) -> Self {
        let (database_surfaces, elements) = database.into_parts();

        let mut surfaces = Vec::new();
        let mut surface_slots = vec![None; NUMBER_OF_TILE_TYPES];
        let mut structure_files: Vec<Option<Ja2StructureFile>> =
            Vec::with_capacity(NUMBER_OF_TILE_TYPES);
        for (tile_type, surface) in database_surfaces.into_iter().enumerate() {
            let Some(surface) = surface else {
                structure_files.push(None);
                continue;
            };
            let id = SurfaceId(surfaces.len() as u16);
            surface_slots[tile_type] = Some(id);
            surfaces.push(Arc::new(TilesetSurface {
                id,
                tileset_id: surface.tileset_id,
                filename: surface.filename,
            }));
            structure_files.push(surface.structure);
        }
        let surface_of = |tile_type: TileType| {
            let id = surface_slots.get(tile_type).copied().flatten()?;
            Some(surfaces[id.index()].clone())
        };

        // Shadows first, so that the structures can point to them
        let mut elements: Vec<Option<TileElement>> = elements.into_iter().map(Some).collect();
        let mut built: Vec<Option<Arc<Tile>>> = vec![None; elements.len()];
        for shadows in [true, false] {
            for id in 0..elements.len() {
                let Some(element) = &elements[id] else {
                    continue;
                };
                if is_shadow(element.tile_type) != shadows {
                    continue;
                }
                let Some(surface) = surface_of(element.tile_type) else {
                    continue;
                };
                let element = elements[id].take().expect("checked above");
                let structure = element.structure.and_then(|structure| {
                    structure_files[element.tile_type]
                        .as_mut()?
                        .structures
                        .get_mut(usize::from(structure))?
                        .take()
                });
                let shadow_buddy = element
                    .buddy
                    .filter(|_| element.flags.contains(TileElementFlags::HAS_SHADOW_BUDDY))
                    .and_then(|buddy| built.get(usize::from(buddy))?.clone());
                built[id] = Some(Arc::new(tile(
                    id as TileIndex,
                    element,
                    surface,
                    structure,
                    shadow_buddy,
                )));
            }
        }

        let mut tiles = Vec::new();
        let mut tile_slots = Vec::with_capacity(built.len());
        for tile in built {
            tile_slots.push(tile.map(|tile| {
                tiles.push(tile);
                (tiles.len() - 1) as u32
            }));
        }
        let current_frames = tiles
            .iter()
            .map(|tile| {
                tile.animation
                    .as_ref()
                    .map_or(0, |animation| animation.initial_frame)
            })
            .collect();

        let mut tileset = Self {
            tiles,
            tile_slots,
            surfaces,
            surface_slots,
            current_frames,
            palettes: Default::default(),
            building_tiles: HashMap::new(),
            building_palettes: Default::default(),
            type_starts: TILE_TYPE_NAMES
                .iter()
                .enumerate()
                .map(|(tile_type, name)| (name.to_string(), tile_type_start_index(tile_type)))
                .collect(),
        };
        tileset.palettes = TileCategory::ALL.map(|category| palette(&tileset, category));
        for tile in &tileset.tiles {
            if let Some(building) = tile.building {
                tileset
                    .building_tiles
                    .entry(building)
                    .or_default()
                    .push(tile.clone());
            }
        }
        tileset.building_palettes =
            BuildingPalette::ALL.map(|palette| building_palette(&tileset, palette));
        tileset
    }
}

/// The debris palette (`DEBRISROCKS` to `LASTDEBRIS`, then the other debris)
const DEBRIS_TYPES: [TileType; 8] = [
    DEBRISROCKS,
    DEBRISWOOD,
    DEBRISWEEDS,
    DEBRISGRASS,
    DEBRISSAND,
    DEBRISMISC,
    DEBRIS2MISC,
    ANOTHERDEBRIS,
];
/// Rocks & barrels, in the order of the original
const ROCK_TYPES: [TileType; 3] = [FOURTHOSTRUCT, FIRSTOSTRUCT, SECONDOSTRUCT];
/// Other junk, in the order of the original
const MISC_TYPES: [TileType; 12] = [
    THIRDOSTRUCT,
    FIFTHOSTRUCT,
    SEVENTHOSTRUCT,
    EIGHTOSTRUCT,
    FIRSTVEHICLE,
    SECONDVEHICLE,
    FIRSTDEBRISSTRUCT,
    SECONDDEBRISSTRUCT,
    FIRSTLARGEEXPDEBRIS,
    SECONDLARGEEXPDEBRIS,
    NINTHOSTRUCT,
    TENTHOSTRUCT,
];

/// What a tile type is to the editor
fn category_of(tile_type: TileType) -> Option<TileCategory> {
    match tile_type {
        FIRSTTEXTURE..=SEVENTHTEXTURE => Some(TileCategory::Ground),
        REGWATERTEXTURE => Some(TileCategory::Water),
        DEEPWATERTEXTURE => Some(TileCategory::DeepWater),
        FIRSTFLOOR..=FOURTHFLOOR => Some(TileCategory::Floor),
        FIRSTFULLSTRUCT | SECONDFULLSTRUCT | SIXTHOSTRUCT => Some(TileCategory::Trees),
        FIRSTROAD => Some(TileCategory::Roads),
        FIRSTCLIFF => Some(TileCategory::Cliffs),
        FIRSTISTRUCT..=FIRSTCISTRUCT => Some(TileCategory::Furniture),
        FIFTHISTRUCT..=EIGHTISTRUCT => Some(TileCategory::GenericFurniture),
        _ if DEBRIS_TYPES.contains(&tile_type) => Some(TileCategory::Debris),
        _ if ROCK_TYPES.contains(&tile_type) => Some(TileCategory::Rocks),
        _ if MISC_TYPES.contains(&tile_type) => Some(TileCategory::Misc),
        _ => None,
    }
}

/// The view layer of a tile type, shadows are in the layer of their structure
fn view_layer(tile_type: TileType, kind: &TileKind) -> Option<ViewLayer> {
    if let Some(buddies) = SHADOW_BUDDIES
        .iter()
        .find(|buddies| buddies.shadow == tile_type)
    {
        return Some(match buddies.structure {
            FIRSTCLIFF => ViewLayer::Terrain,
            FIRSTDOOR..=FOURTHDOOR => ViewLayer::Buildings,
            _ => ViewLayer::Objects,
        });
    }
    match kind {
        // Floors are in the land list, they stay when the buildings are hidden
        TileKind::Terrain(_) | TileKind::Cliff { .. } | TileKind::Floor => Some(ViewLayer::Terrain),
        TileKind::Object if matches!(tile_type, FIRSTROAD | ROADPIECES) => Some(ViewLayer::Terrain),
        TileKind::Wall(_)
        | TileKind::Door(_)
        | TileKind::Roof(_)
        | TileKind::OnRoof
        | TileKind::WallDecoration { .. } => Some(ViewLayer::Buildings),
        TileKind::Item | TileKind::Effect => None,
        TileKind::Structure | TileKind::Shadow | TileKind::Object => Some(ViewLayer::Objects),
    }
}

/// The tiles of a type that have an image of their own, in the order of the images
fn type_tiles(tileset: &Tileset, tile_type: TileType) -> impl Iterator<Item = &Arc<Tile>> + '_ {
    (1..=NUM_TILES_PER_TYPE[tile_type])
        .filter_map(move |sub_index| tileset.tile(tileset.index(tile_type, sub_index)?))
        .filter(move |tile| {
            tile.type_name == TILE_TYPE_NAMES[tile_type] && !tile.image.underflow_filler
        })
}

/// The palette of a category, in the order of the original selection window
/// (`InitJA2SelectionWindow`)
fn palette(tileset: &Tileset, category: TileCategory) -> Vec<Arc<Tile>> {
    let all = |types: &[TileType]| -> Vec<Arc<Tile>> {
        types
            .iter()
            .flat_map(|&tile_type| type_tiles(tileset, tile_type))
            .cloned()
            .collect()
    };
    match category {
        TileCategory::Ground | TileCategory::Water | TileCategory::DeepWater => (FIRSTTEXTURE
            ..=DEEPWATERTEXTURE)
            .filter(|&tile_type| category_of(tile_type) == Some(category))
            .filter_map(|tile_type| type_tiles(tileset, tile_type).next())
            .cloned()
            .collect(),
        TileCategory::Floor => building_palette(tileset, BuildingPalette::Floors),
        TileCategory::Debris => all(&DEBRIS_TYPES),
        TileCategory::Trees => {
            // Every third image of the full structures, the others are parts of their animation
            let mut tiles: Vec<Arc<Tile>> = [FIRSTFULLSTRUCT, SECONDFULLSTRUCT]
                .into_iter()
                .flat_map(|tile_type| type_tiles(tileset, tile_type))
                .filter(|tile| tile.image.sub_image % 3 == 0)
                .cloned()
                .collect();
            tiles.extend(all(&[SIXTHOSTRUCT]));
            tiles
        }
        TileCategory::Rocks => all(&ROCK_TYPES),
        TileCategory::Misc => all(&MISC_TYPES),
        TileCategory::Furniture => all(&(FIRSTISTRUCT..=FIRSTCISTRUCT).collect::<Vec<_>>()),
        TileCategory::GenericFurniture => all(&(FIFTHISTRUCT..=EIGHTISTRUCT).collect::<Vec<_>>()),
        TileCategory::Roads => type_tiles(tileset, FIRSTROAD)
            .filter(|tile| tile.sub_index <= NUM_ROAD_MACROS)
            .cloned()
            .collect(),
        TileCategory::Cliffs => all(&[FIRSTCLIFF]),
    }
}

/// The piece of a wall by its sub index (`gbWallTileLUT` and the offsets of `SelectWin.cc`)
fn wall_piece(sub_index: u16) -> Option<WallPiece> {
    Some(match sub_index {
        1..=3 | 18..=20 => WallPiece::ExteriorRight,
        4..=6 | 21..=23 => WallPiece::ExteriorLeft,
        7..=9 | 24..=26 => WallPiece::InteriorRight,
        10..=12 | 27..=29 => WallPiece::InteriorLeft,
        13 => WallPiece::ExteriorBottomEnd,
        14 => WallPiece::InteriorCorner,
        15 => WallPiece::InteriorBottomEnd,
        16 => WallPiece::InteriorExtended,
        17 => WallPiece::ExteriorExtendedBottomEnd,
        30 => WallPiece::ExteriorLeftShadow,
        31 => WallPiece::InteriorBottomEndShadow,
        // `VWALL_LEFT`, `VWALL_RIGHT`, `HWALL_RIGHT` and `HWALL_LEFT` of `BuildSlantRoof`
        32 => WallPiece::AFrame {
            vertical: true,
            left: true,
        },
        33 => WallPiece::AFrame {
            vertical: true,
            left: false,
        },
        34 => WallPiece::AFrame {
            vertical: false,
            left: false,
        },
        35 => WallPiece::AFrame {
            vertical: false,
            left: true,
        },
        // 3 window styles for each orientation
        36..=47 => WallPiece::Window {
            style: ((sub_index - 36) % 3) as u8,
        },
        // 2 broken walls for each orientation
        48..=55 => WallPiece::Broken {
            variant: ((sub_index - 48) % 2) as u8,
        },
        56 => WallPiece::InteriorExtendedBottomEnd,
        57 => WallPiece::ExteriorExtended,
        // A cracked and a smudged wall for each inside orientation, then for each outside one
        58 | 59 | 62 | 63 => WallPiece::Cracked,
        60 | 61 | 64 | 65 => WallPiece::Smudged,
        _ => return None,
    })
}

/// The piece of a flat roof by its sub index (the roof indices of `NewSmooth.cc`)
fn roof_piece(sub_index: u16) -> Option<RoofPiece> {
    Some(match sub_index {
        1 => RoofPiece::Left,
        2 => RoofPiece::Top,
        3 => RoofPiece::TopLeft,
        4 => RoofPiece::Bottom,
        5 => RoofPiece::Right,
        6 => RoofPiece::BottomRight,
        7 => RoofPiece::TopRight,
        8 => RoofPiece::BottomLeft,
        9..=11 => RoofPiece::Center,
        _ => return None,
    })
}

/// The piece of a slant roof by its sub index (the slant roof table of `NewSmooth.cc`)
fn slant_piece(sub_index: u16) -> Option<SlantPiece> {
    let (thick, side) = match sub_index {
        1 => (false, SlantSide::Bottom),
        2 => (false, SlantSide::Top),
        3 => (true, SlantSide::Bottom),
        4 => (true, SlantSide::Top),
        5 => (false, SlantSide::Left),
        6 => (false, SlantSide::Right),
        7 => (true, SlantSide::Left),
        8 => (true, SlantSide::Right),
        _ => return None,
    };
    Some(SlantPiece { thick, side })
}

/// What a tile of a building type is to the editor
fn building_of(tile_type: TileType, sub_index: u16) -> Option<BuildingTile> {
    let (first, piece) = match tile_type {
        FIRSTWALL..=FOURTHWALL => (FIRSTWALL, BuildingPiece::Wall(wall_piece(sub_index)?)),
        FIRSTDOOR..=FOURTHDOOR => {
            // 5 frames for each orientation, from closed to open
            let piece = match (sub_index - 1) % 5 {
                0 => BuildingPiece::Door { open: false },
                4 => BuildingPiece::Door { open: true },
                _ => BuildingPiece::DoorFrame,
            };
            (FIRSTDOOR, piece)
        }
        FIRSTDOORSHADOW..=FOURTHDOORSHADOW => (FIRSTDOORSHADOW, BuildingPiece::DoorShadow),
        FIRSTFLOOR..=FOURTHFLOOR => (FIRSTFLOOR, BuildingPiece::Floor),
        FIRSTROOF..=FOURTHROOF => (FIRSTROOF, BuildingPiece::Roof(roof_piece(sub_index))),
        FIRSTSLANTROOF..=SECONDSLANTROOF => (
            FIRSTSLANTROOF,
            BuildingPiece::SlantRoof(slant_piece(sub_index)),
        ),
        FIRSTONROOF..=SECONDONROOF => (FIRSTONROOF, BuildingPiece::OnRoof),
        FIRSTDECORATIONS..=FOURTHDECORATIONS => (FIRSTDECORATIONS, BuildingPiece::Decoration),
        FIRSTWALLDECAL..=FOURTHWALLDECAL => (FIRSTWALLDECAL, BuildingPiece::WallDecal),
        // Styles 4 to 7
        FIFTHWALLDECAL..=EIGTHWALLDECAL => (FIFTHWALLDECAL - 4, BuildingPiece::WallDecal),
        FIRSTSWITCHES => (FIRSTSWITCHES, BuildingPiece::Switch),
        FIRSTISTRUCT..=FIRSTCISTRUCT => (FIRSTISTRUCT, BuildingPiece::Furniture),
        FIFTHISTRUCT..=EIGHTISTRUCT => (FIFTHISTRUCT, BuildingPiece::GenericFurniture),
        _ => return None,
    };
    Some(BuildingTile {
        style: (tile_type - first) as u8,
        piece,
    })
}

/// Whether placing a building removes tiles of the type (`BulldozeNature`)
fn is_bulldozed(tile_type: TileType) -> bool {
    matches!(
        tile_type,
        FIRSTISTRUCT..=FIRSTCISTRUCT
            | FIRSTCLIFFSHADOW
            | FIRSTOSTRUCT..=SECONDFULLSTRUCT
            | FIRSTSHADOW..=FOURTHFULLSHADOW
            | FIRSTROAD
            | DEBRISROCKS..=DEBRISMISC
            | ANOTHERDEBRIS
    )
}

/// The wall, door, floor and roof types, which have a tile type for each style
const WALL_TYPES: [TileType; 4] = [FIRSTWALL, FIRSTWALL + 1, FIRSTWALL + 2, FOURTHWALL];
const DOOR_TYPES: [TileType; 4] = [FIRSTDOOR, FIRSTDOOR + 1, FIRSTDOOR + 2, FOURTHDOOR];
const FLOOR_TYPES: [TileType; 4] = [FIRSTFLOOR, FIRSTFLOOR + 1, FIRSTFLOOR + 2, FOURTHFLOOR];
const ROOF_TYPES: [TileType; 4] = [FIRSTROOF, FIRSTROOF + 1, FIRSTROOF + 2, FOURTHROOF];

/// The tiles of a type with the given sub indices, in their order
fn sub_tiles<'a>(
    tileset: &'a Tileset,
    tile_type: TileType,
    sub_indices: impl IntoIterator<Item = u16> + 'a,
) -> impl Iterator<Item = &'a Arc<Tile>> + 'a {
    let tiles: Vec<&Arc<Tile>> = type_tiles(tileset, tile_type).collect();
    sub_indices
        .into_iter()
        .filter_map(move |sub_index| tiles.iter().find(|t| t.sub_index == sub_index).copied())
}

/// The tiles of the types with the given sub indices, type by type
fn subs_of_types(
    tileset: &Tileset,
    types: impl IntoIterator<Item = TileType>,
    sub_indices: &[u16],
) -> Vec<Arc<Tile>> {
    types
        .into_iter()
        .flat_map(|tile_type| sub_tiles(tileset, tile_type, sub_indices.iter().copied()))
        .cloned()
        .collect()
}

/// A palette of the buildings mode, in the order of the original selection window
/// (`InitJA2SelectionWindow`). The smart palettes have a tile for each value of the smart
/// method, taken from the first wall and door types that have them.
fn building_palette(tileset: &Tileset, palette: BuildingPalette) -> Vec<Arc<Tile>> {
    let all = |types: &[TileType]| -> Vec<Arc<Tile>> {
        types
            .iter()
            .flat_map(|&tile_type| type_tiles(tileset, tile_type))
            .cloned()
            .collect()
    };
    let range = |from: u16, to: u16| (from..=to).collect::<Vec<u16>>();
    // The first wall type that has all of the sub indices
    let first_wall_with = |sub_indices: &[u16]| -> Vec<Arc<Tile>> {
        WALL_TYPES
            .iter()
            .map(|&tile_type| subs_of_types(tileset, [tile_type], sub_indices))
            .find(|tiles| tiles.len() == sub_indices.len())
            .unwrap_or_default()
    };
    match palette {
        BuildingPalette::Room => {
            let mut types = WALL_TYPES.to_vec();
            types.extend(FLOOR_TYPES);
            types.extend(ROOF_TYPES);
            types.extend([FIRSTSLANTROOF, SECONDSLANTROOF]);
            subs_of_types(tileset, types, &[1])
        }
        BuildingPalette::Walls => WALL_TYPES
            .iter()
            .flat_map(|&tile_type| {
                let mut subs = range(1, 31);
                subs.extend([56, 57]);
                subs_of_types(tileset, [tile_type], &subs)
            })
            .collect(),
        BuildingPalette::Doors => {
            subs_of_types(tileset, DOOR_TYPES, &[1, 5, 6, 10, 11, 15, 16, 20])
        }
        BuildingPalette::Windows => subs_of_types(tileset, WALL_TYPES, &range(36, 47)),
        BuildingPalette::Roofs => {
            let mut tiles = all(&ROOF_TYPES);
            tiles.extend(all(&[FIRSTSLANTROOF, SECONDSLANTROOF]));
            tiles.extend(subs_of_types(tileset, WALL_TYPES, &range(32, 35)));
            tiles.extend(all(&[FIRSTONROOF, SECONDONROOF]));
            tiles
        }
        BuildingPalette::BrokenWalls => {
            let mut tiles = all(&(FIRSTDECORATIONS..=FOURTHDECORATIONS).collect::<Vec<_>>());
            tiles.extend(subs_of_types(tileset, WALL_TYPES, &range(48, 55)));
            tiles.extend(subs_of_types(tileset, WALL_TYPES, &range(58, 65)));
            tiles
        }
        BuildingPalette::Decals => {
            let mut types: Vec<TileType> = (FIRSTWALLDECAL..=FOURTHWALLDECAL).collect();
            types.extend(FIFTHWALLDECAL..=EIGTHWALLDECAL);
            types.push(FIRSTSWITCHES);
            all(&types)
        }
        BuildingPalette::Floors => subs_of_types(tileset, FLOOR_TYPES, &range(1, 8)),
        BuildingPalette::SmartWalls => subs_of_types(tileset, WALL_TYPES, &[1]),
        // Closed and open, like the values of the original (`CalcSmartDoorDefault`)
        BuildingPalette::SmartDoors => subs_of_types(tileset, DOOR_TYPES, &[1, 5]),
        // The outside windows (`CalcSmartWindowDefault`)
        BuildingPalette::SmartWindows => first_wall_with(&[45, 46, 47]),
        // The 2 broken walls, the cracked and the smudged wall, without the hole in the wall
        // (`CalcSmartBrokenWallDefault`)
        BuildingPalette::SmartBrokenWalls => first_wall_with(&[50, 51, 63, 65]),
        BuildingPalette::SmartRoofs => subs_of_types(tileset, ROOF_TYPES, &[10]),
    }
}

fn tile(
    id: TileIndex,
    element: TileElement,
    surface: Arc<TilesetSurface>,
    structure: Option<DbStructure>,
    shadow_buddy: Option<Arc<Tile>>,
) -> Tile {
    let flags = element.flags;
    let kind = kind(&element);
    let view_layer = view_layer(element.tile_type, &kind);
    let lit_as_wall = kind == TileKind::Structure
        && (FIRSTWALLDECAL..=EIGTHWALLDECAL).contains(&element.tile_type);
    Tile {
        id,
        type_name: TILE_TYPE_NAMES[element.tile_type].to_string(),
        sub_index: id - tile_type_start_index(element.tile_type) + 1,
        kind,
        category: category_of(element.tile_type),
        view_layer,
        erased_with_structures: matches!(element.tile_type, DEBRIS2MISC | ANOTHERDEBRIS),
        building: building_of(
            element.tile_type,
            id - tile_type_start_index(element.tile_type) + 1,
        ),
        bulldozed: is_bulldozed(element.tile_type),
        image: TileImage {
            surface,
            sub_image: element.region_index,
            underflow_filler: flags.contains(TileElementFlags::UNDERFLOW_FILLER),
        },
        terrain: element.terrain.into(),
        shadow_buddy,
        structure: structure.map(TileStructure::from),
        covered_tiles: element.tile_locs,
        z_strip: element.z_strip.map(ZStrip::from),
        // Shadows take the flag from their structure, they can have no frames of their own
        animation: flags
            .contains(TileElementFlags::ANIMATED_TILE)
            .then_some(TileAnimation {
                frames: element.anim_frames,
                initial_frame: element.current_frame,
            }),
        dynamic: flags.contains(TileElementFlags::DYNAMIC_TILE),
        ignores_world_height: flags.contains(TileElementFlags::IGNORE_WORLD_HEIGHT),
        uses_object_z_height: flags.contains(TileElementFlags::OBJECTLAYER_USEZHEIGHT),
        lit_as_wall,
    }
}

fn is_shadow(tile_type: TileType) -> bool {
    SHADOW_BUDDIES
        .iter()
        .any(|buddies| buddies.shadow == tile_type)
}

/// Classifies a tile by its type, with the ranges of `SetSpecificDatabaseValues`
fn kind(element: &TileElement) -> TileKind {
    let tile_type = element.tile_type;
    let orientation = element.wall_orientation.into();
    if is_shadow(tile_type) {
        return TileKind::Shadow;
    }
    match tile_type {
        FIRSTTEXTURE..=DEEPWATERTEXTURE => TileKind::Terrain(TerrainTile {
            full: element.full_tile,
        }),
        FIRSTCLIFFHANG => TileKind::Cliff { hang: true },
        FIRSTCLIFF => TileKind::Cliff { hang: false },
        FIRSTWALL..=FOURTHWALL => TileKind::Wall(WallTile {
            orientation,
            aframe: element.flags.contains(TileElementFlags::AFRAME_TILE),
        }),
        FIRSTDOOR..=FOURTHDOOR => TileKind::Door(DoorTile {
            orientation,
            animation: element.anim_frames.clone(),
        }),
        FIRSTFLOOR..=FOURTHFLOOR => TileKind::Floor,
        FIRSTROOF..=FOURTHROOF => TileKind::Roof(RoofKind::Flat),
        FIRSTSLANTROOF..=SECONDSLANTROOF | SLANTROOFCEILING => TileKind::Roof(RoofKind::Slant),
        FIRSTHIGHROOF..=SECONDHIGHROOF => TileKind::Roof(RoofKind::High),
        REVEALEDSLANTROOFS..=SECONDREVEALEDHIGHROOFS => TileKind::Roof(RoofKind::Revealed),
        FIRSTONROOF..=SECONDONROOF => TileKind::OnRoof,
        FOURTHWINDOW
        | FIRSTWALLDECAL..=FOURTHWALLDECAL
        | FIFTHWALLDECAL..=EIGTHWALLDECAL
        | FIRSTSWITCHES => TileKind::WallDecoration { orientation },
        FIRSTOSTRUCT..=FOURTHFULLSTRUCT
        | FIRSTISTRUCT..=FIRSTCISTRUCT
        | FIFTHISTRUCT..=EIGHTISTRUCT
        | ANIOSTRUCT
        | FENCESTRUCT
        | FIRSTVEHICLE
        | SECONDVEHICLE
        | FIRSTDEBRISSTRUCT
        | SECONDDEBRISSTRUCT
        | NINTHOSTRUCT
        | TENTHOSTRUCT
        | FIRSTLARGEEXPDEBRIS
        | SECONDLARGEEXPDEBRIS => TileKind::Structure,
        GUNS..=P2ITEMS | P3ITEMS => TileKind::Item,
        WINDOWSHATTER | BODYEXPLOSION | FOOTPRINTS..=WIREFRAMES => TileKind::Effect,
        // Roads, debris, decorations, blood, explosion debris, the mock floor and exits
        _ => TileKind::Object,
    }
}

impl From<DbStructure> for TileStructure {
    fn from(structure: DbStructure) -> Self {
        Self {
            armour: structure.armour,
            flags: StructureFlags::from_bits_retain(structure.flags.bits()),
            wall_orientation: structure.wall_orientation.into(),
            tiles: structure
                .tiles
                .into_iter()
                .map(|tile| StructureTile {
                    pos_rel_to_base: tile.pos_rel_to_base,
                    on_roof: tile.flags.contains(StructureTileFlags::ON_ROOF),
                    passable: tile.flags.contains(StructureTileFlags::PASSABLE),
                    shape: tile.shape,
                })
                .collect(),
        }
    }
}

impl From<ja2_tile_types::Orientation> for Orientation {
    fn from(orientation: ja2_tile_types::Orientation) -> Self {
        Self::from_bits_retain(orientation.bits())
    }
}

impl From<TerrainType> for Terrain {
    fn from(terrain: TerrainType) -> Self {
        match terrain {
            TerrainType::NoTerrain => Self::NoTerrain,
            TerrainType::FlatGround => Self::FlatGround,
            TerrainType::FlatFloor => Self::FlatFloor,
            TerrainType::PavedRoad => Self::PavedRoad,
            TerrainType::DirtRoad => Self::DirtRoad,
            TerrainType::LowGrass => Self::LowGrass,
            TerrainType::HighGrass => Self::HighGrass,
            TerrainType::TrainTracks => Self::TrainTracks,
            TerrainType::LowWater => Self::LowWater,
            TerrainType::MedWater => Self::MedWater,
            TerrainType::DeepWater => Self::DeepWater,
        }
    }
}

impl From<ZStripInfo> for ZStrip {
    fn from(info: ZStripInfo) -> Self {
        Self {
            initial_z_change: info.initial_z_change,
            first_z_strip_width: info.first_z_strip_width,
            z_changes: info.z_changes,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        file_formats::{
            ja2_structure::{
                tests::{aux_bytes, jsd_bytes, structure_bytes},
                StructureFlags,
            },
            ja2_tile_database::{
                tests::{full_surfaces, surface},
                TileSurfaceInfo,
            },
            ja2_tile_types::{
                FIRSTFULLSHADOW, ROADPIECES, SECONDISTRUCT, SECONDTEXTURE, THIRDFULLSTRUCT,
                THIRDTEXTURE,
            },
        },
        tileset::tests::test_tileset,
    };

    fn build(surfaces: Vec<Option<TileSurfaceInfo>>) -> Tileset {
        Tileset::from_database(Ja2TileDatabase::build(surfaces).unwrap())
    }

    fn tile(tileset: &Tileset, tile_type: TileType, sub_index: u16) -> &Arc<Tile> {
        tileset
            .tile(tileset.index(tile_type, sub_index).unwrap())
            .unwrap()
    }

    #[test]
    fn classification() {
        let tileset = test_tileset();
        let kind = |tile_type, sub_index| tile(&tileset, tile_type, sub_index).kind.clone();
        assert_eq!(
            kind(FIRSTWALL, 32),
            TileKind::Wall(WallTile {
                orientation: Orientation::empty(),
                aframe: true,
            })
        );
        assert_eq!(
            kind(FIRSTWALL, 1),
            TileKind::Wall(WallTile {
                orientation: Orientation::empty(),
                aframe: false,
            })
        );
        assert_eq!(
            kind(FIRSTDOOR, 1),
            TileKind::Door(DoorTile {
                orientation: Orientation::empty(),
                animation: vec![907, 908, 909, 910, 911],
            })
        );
        assert_eq!(kind(FIRSTSHADOW, 1), TileKind::Shadow);
        assert_eq!(kind(FIRSTOSTRUCT, 1), TileKind::Structure);
        assert_eq!(kind(GUNS, 1), TileKind::Item);
        assert_eq!(kind(FIRSTCLIFFHANG, 1), TileKind::Cliff { hang: true });
        assert_eq!(kind(FIRSTCLIFF, 1), TileKind::Cliff { hang: false });
        assert!(tile(&tileset, FIRSTFLOOR, 3).is_floor());
        assert!(tile(&tileset, FIRSTFLOOR, 3).is_full_land());
        assert!(tile(&tileset, FIRSTROOF, 1).is_flat_roof());
        assert!(tile(&tileset, FOURTHROOF, 1).is_flat_roof());
        assert!(!tile(&tileset, FIRSTSLANTROOF, 1).is_flat_roof());
        assert!(!tile(&tileset, FIRSTHIGHROOF, 1).is_flat_roof());

        assert!(tile(&tileset, FIRSTTEXTURE, 1).is_full_land());
        let door = tile(&tileset, FIRSTDOOR, 7);
        assert_eq!((door.type_name.as_str(), door.sub_index), ("DOOR1", 7));
        assert!(!tile(&tileset, FIRSTTEXTURE, 11).is_full_land());
        assert!(tile(&tileset, FIRSTCLIFFHANG, 1).ignores_world_height);
        assert!(tile(&tileset, FIRSTFULLSTRUCT, 2).dynamic);
        assert!(tile(&tileset, GUNS, 1).uses_object_z_height);
        assert!(tile(&tileset, FIRSTISTRUCT, 16).lit_as_wall);
        assert!(tile(&tileset, SECONDISTRUCT, 1).lit_as_wall);
        assert!(tile(&tileset, FIRSTVEHICLE, 1).lit_as_wall);
        assert!(tile(&tileset, TENTHOSTRUCT, 1).lit_as_wall);
        assert!(!tile(&tileset, FIRSTOSTRUCT, 1).lit_as_wall);
        assert!(!tile(&tileset, FIRSTWALLDECAL, 1).lit_as_wall);
        assert!(!tile(&tileset, FIRSTFLOOR, 1).lit_as_wall);
    }

    #[test]
    fn water() {
        let mut surfaces = full_surfaces();
        for tile_type in [REGWATERTEXTURE, DEEPWATERTEXTURE] {
            let terrain = if tile_type == REGWATERTEXTURE {
                TerrainType::LowWater
            } else {
                TerrainType::DeepWater
            };
            surfaces[tile_type].as_mut().unwrap().terrain = terrain;
        }
        let tileset = build(surfaces);
        let category = |tile_type, sub_index| tile(&tileset, tile_type, sub_index).category;
        // Shores are water to the editor, but flat ground to move on
        assert_eq!(category(REGWATERTEXTURE, 20), Some(TileCategory::Water));
        assert_eq!(
            tile(&tileset, REGWATERTEXTURE, 20).terrain,
            Terrain::FlatGround
        );
        assert_eq!(category(REGWATERTEXTURE, 1), Some(TileCategory::Water));
        assert_eq!(
            tile(&tileset, REGWATERTEXTURE, 1).terrain,
            Terrain::LowWater
        );
        assert_eq!(category(DEEPWATERTEXTURE, 1), Some(TileCategory::DeepWater));
        assert_eq!(category(FIRSTTEXTURE, 1), Some(TileCategory::Ground));
        assert_eq!(category(SEVENTHTEXTURE, 1), Some(TileCategory::Ground));
    }

    #[test]
    fn editor_categories() {
        let tileset = test_tileset();
        let category = |tile_type, sub_index| tile(&tileset, tile_type, sub_index).category;
        assert_eq!(category(FIRSTFLOOR, 1), Some(TileCategory::Floor));
        assert_eq!(category(FOURTHFLOOR, 1), Some(TileCategory::Floor));
        assert_eq!(category(ANOTHERDEBRIS, 1), Some(TileCategory::Debris));
        assert_eq!(category(DEBRISSAND, 1), Some(TileCategory::Debris));
        assert_eq!(category(FOURTHOSTRUCT, 1), Some(TileCategory::Rocks));
        assert_eq!(category(FIRSTVEHICLE, 1), Some(TileCategory::Misc));
        assert_eq!(category(FIRSTROAD, 1), Some(TileCategory::Roads));
        assert_eq!(category(FIRSTCLIFF, 1), Some(TileCategory::Cliffs));
        assert_eq!(category(FIRSTCLIFFHANG, 1), None);
        assert_eq!(category(FIRSTISTRUCT, 1), Some(TileCategory::Furniture));
        assert_eq!(category(FIRSTCISTRUCT, 1), Some(TileCategory::Furniture));
        assert_eq!(
            category(FIFTHISTRUCT, 1),
            Some(TileCategory::GenericFurniture)
        );
        assert_eq!(
            category(EIGHTISTRUCT, 1),
            Some(TileCategory::GenericFurniture)
        );
        assert_eq!(category(FIRSTWALL, 1), None);
        assert_eq!(category(FIRSTROOF, 1), None);
        assert_eq!(category(ROADPIECES, 1), None);
        assert_eq!(category(THIRDFULLSTRUCT, 1), None);

        // An animation frame of a tree is a tree, but not in the palette
        let frame = tile(&tileset, FIRSTFULLSTRUCT, 2);
        assert_eq!(frame.category, Some(TileCategory::Trees));
        assert!(!tileset
            .palette(TileCategory::Trees)
            .iter()
            .any(|tile| Arc::ptr_eq(tile, frame)));

        assert!(tile(&tileset, DEBRIS2MISC, 1).erased_with_structures);
        assert!(tile(&tileset, ANOTHERDEBRIS, 1).erased_with_structures);
        assert!(!tile(&tileset, DEBRISROCKS, 1).erased_with_structures);
        assert!(!tile(&tileset, FIRSTOSTRUCT, 1).erased_with_structures);
    }

    #[test]
    fn view_layers() {
        let tileset = test_tileset();
        let layer = |tile_type| tile(&tileset, tile_type, 1).view_layer;
        assert_eq!(layer(FIRSTTEXTURE), Some(ViewLayer::Terrain));
        assert_eq!(layer(DEEPWATERTEXTURE), Some(ViewLayer::Terrain));
        assert_eq!(layer(FIRSTROAD), Some(ViewLayer::Terrain));
        assert_eq!(layer(ROADPIECES), Some(ViewLayer::Terrain));
        assert_eq!(layer(FIRSTCLIFF), Some(ViewLayer::Terrain));
        assert_eq!(layer(FIRSTCLIFFHANG), Some(ViewLayer::Terrain));
        assert_eq!(layer(FIRSTCLIFFSHADOW), Some(ViewLayer::Terrain));
        assert_eq!(layer(FIRSTFULLSTRUCT), Some(ViewLayer::Objects));
        assert_eq!(layer(FIRSTFULLSHADOW), Some(ViewLayer::Objects));
        assert_eq!(layer(FIRSTISTRUCT), Some(ViewLayer::Objects));
        assert_eq!(layer(DEBRISROCKS), Some(ViewLayer::Objects));
        assert_eq!(layer(FIRSTWALL), Some(ViewLayer::Buildings));
        assert_eq!(layer(FIRSTDOOR), Some(ViewLayer::Buildings));
        assert_eq!(layer(FIRSTDOORSHADOW), Some(ViewLayer::Buildings));
        assert_eq!(layer(FIRSTFLOOR), Some(ViewLayer::Terrain));
        assert_eq!(layer(FIRSTROOF), Some(ViewLayer::Buildings));
        assert_eq!(layer(FIRSTONROOF), Some(ViewLayer::Buildings));
        assert_eq!(layer(FIRSTWALLDECAL), Some(ViewLayer::Buildings));
        assert_eq!(layer(FOURTHWINDOW), Some(ViewLayer::Buildings));
        assert_eq!(layer(FIRSTSWITCHES), Some(ViewLayer::Buildings));
        assert_eq!(layer(GUNS), None);
        assert_eq!(layer(WIREFRAMES), None);
    }

    fn types(tiles: &[Arc<Tile>]) -> Vec<&str> {
        let mut types: Vec<&str> = tiles.iter().map(|tile| tile.type_name.as_str()).collect();
        types.dedup();
        types
    }

    fn names(types: &[TileType]) -> Vec<&'static str> {
        types
            .iter()
            .map(|&tile_type| TILE_TYPE_NAMES[tile_type])
            .collect()
    }

    #[test]
    fn palettes_follow_the_selection_window() {
        let tileset = test_tileset();
        let trees = tileset.palette(TileCategory::Trees);
        assert_eq!(types(trees), ["OFSTRUCT1", "OFSTRUCT2", "OSTRUCT6"]);
        let first: Vec<u16> = trees
            .iter()
            .filter(|tile| tile.type_name == "OFSTRUCT1")
            .map(|tile| tile.sub_index)
            .collect();
        assert_eq!(first, [1, 4, 7, 10]);
        assert_eq!(
            types(tileset.palette(TileCategory::Rocks)),
            ["OSTRUCT4", "OSTRUCT1", "OSTRUCT2"]
        );
        assert_eq!(
            types(tileset.palette(TileCategory::Misc)),
            names(&MISC_TYPES)
        );
        let debris = types(tileset.palette(TileCategory::Debris));
        assert_eq!(debris.first(), Some(&"ROCKS"));
        assert_eq!(debris[debris.len() - 2..], ["MISC2", "ANOTHERDEBRIS"]);
        assert_eq!(tileset.palette(TileCategory::Furniture).len(), 5 * 24);
        assert_eq!(
            tileset.palette(TileCategory::GenericFurniture).len(),
            4 * 24
        );
        let roads = tileset.palette(TileCategory::Roads);
        assert_eq!(roads.len(), usize::from(NUM_ROAD_MACROS));
        assert_eq!(roads[5].sub_index, 6);
        let cliffs = tileset.palette(TileCategory::Cliffs);
        assert_eq!(cliffs.len(), usize::from(NUM_TILES_PER_TYPE[FIRSTCLIFF]));
        assert_eq!(cliffs[2].sub_index, 3);

        // The first tile of each texture
        let ground = tileset.palette(TileCategory::Ground);
        assert_eq!(
            types(ground),
            names(&(FIRSTTEXTURE..=SEVENTHTEXTURE).collect::<Vec<_>>())
        );
        assert!(ground.iter().all(|tile| tile.sub_index == 1));
        let water = tileset.palette(TileCategory::Water);
        assert_eq!((water.len(), water[0].type_name.as_str()), (1, "WATER1"));
        let deep = tileset.palette(TileCategory::DeepWater);
        assert_eq!((deep.len(), deep[0].type_name.as_str()), (1, "DEEP WATER"));
        let floors = tileset.palette(TileCategory::Floor);
        let ids = |tiles: &[Arc<Tile>]| tiles.iter().map(|t| t.id).collect::<Vec<_>>();
        assert_eq!(
            ids(floors),
            ids(tileset.building_palette(BuildingPalette::Floors))
        );
        assert_eq!(floors.len(), 4 * 8);
    }

    #[test]
    fn building_pieces_follow_the_tables_of_the_original() {
        let tileset = test_tileset();
        let building = |tile_type, sub_index| tile(&tileset, tile_type, sub_index).building;
        let wall = |style, piece| {
            Some(BuildingTile {
                style,
                piece: BuildingPiece::Wall(piece),
            })
        };
        use crate::file_formats::ja2_tile_types::{
            FIFTHWALLDECAL, FIRSTDOORSHADOW, FOURTHISTRUCT, SECONDWALL, THIRDDOOR,
        };
        assert_eq!(building(FIRSTWALL, 1), wall(0, WallPiece::ExteriorRight));
        assert_eq!(building(SECONDWALL, 20), wall(1, WallPiece::ExteriorRight));
        assert_eq!(building(FIRSTWALL, 29), wall(0, WallPiece::InteriorLeft));
        assert_eq!(
            building(FIRSTWALL, 56),
            wall(0, WallPiece::InteriorExtendedBottomEnd)
        );
        assert_eq!(
            building(FIRSTWALL, 57),
            wall(0, WallPiece::ExteriorExtended)
        );
        assert_eq!(
            building(FOURTHWALL, 30),
            wall(3, WallPiece::ExteriorLeftShadow)
        );
        assert_eq!(
            building(FIRSTWALL, 35),
            wall(
                0,
                WallPiece::AFrame {
                    vertical: false,
                    left: true
                }
            )
        );
        assert_eq!(
            building(FIRSTWALL, 46),
            wall(0, WallPiece::Window { style: 1 })
        );
        assert_eq!(
            building(FIRSTWALL, 51),
            wall(0, WallPiece::Broken { variant: 1 })
        );
        assert_eq!(building(FIRSTWALL, 63), wall(0, WallPiece::Cracked));
        assert_eq!(building(FIRSTWALL, 64), wall(0, WallPiece::Smudged));
        let piece = |tile_type, sub_index| building(tile_type, sub_index).map(|b| b.piece);
        assert_eq!(
            piece(THIRDDOOR, 6),
            Some(BuildingPiece::Door { open: false })
        );
        assert_eq!(
            piece(THIRDDOOR, 10),
            Some(BuildingPiece::Door { open: true })
        );
        assert_eq!(piece(THIRDDOOR, 8), Some(BuildingPiece::DoorFrame));
        assert_eq!(building(THIRDDOOR, 1).unwrap().style, 2);
        assert_eq!(piece(FIRSTDOORSHADOW, 1), Some(BuildingPiece::DoorShadow));
        assert_eq!(
            piece(FIRSTROOF, 3),
            Some(BuildingPiece::Roof(Some(RoofPiece::TopLeft)))
        );
        assert_eq!(
            piece(FIRSTROOF, 11),
            Some(BuildingPiece::Roof(Some(RoofPiece::Center)))
        );
        assert_eq!(piece(FIRSTROOF, 12), Some(BuildingPiece::Roof(None)));
        assert_eq!(
            piece(SECONDSLANTROOF, 7),
            Some(BuildingPiece::SlantRoof(Some(SlantPiece {
                thick: true,
                side: SlantSide::Left
            })))
        );
        assert_eq!(building(FIFTHWALLDECAL, 1).unwrap().style, 4);
        assert_eq!(piece(FIRSTSWITCHES, 1), Some(BuildingPiece::Switch));
        assert_eq!(piece(FIRSTCISTRUCT, 1), Some(BuildingPiece::Furniture));
        assert_eq!(building(FIRSTCISTRUCT, 1).unwrap().style, 4);
        assert_eq!(
            piece(EIGHTISTRUCT, 1),
            Some(BuildingPiece::GenericFurniture)
        );
        assert_eq!(piece(FOURTHISTRUCT, 1), Some(BuildingPiece::Furniture));
        assert_eq!(building(FIRSTTEXTURE, 1), None);

        // The variants of a piece
        let variants = tileset.building_tiles(0, BuildingPiece::Wall(WallPiece::InteriorLeft));
        let subs: Vec<u16> = variants.iter().map(|t| t.sub_index).collect();
        assert_eq!(subs, [10, 11, 12, 27, 28, 29]);
        assert_eq!(
            tileset
                .building_tiles(1, BuildingPiece::Roof(Some(RoofPiece::Center)))
                .len(),
            3
        );
        assert!(tileset.building_tiles(7, BuildingPiece::Floor).is_empty());

        // Doors have their shadows, walls none
        let door = tile(&tileset, FIRSTDOOR, 7);
        let shadow = door.door_shadow().unwrap();
        assert_eq!(
            (shadow.type_name.as_str(), shadow.sub_index),
            ("DOORSH1", 7)
        );
        assert!(tile(&tileset, FIRSTWALL, 1).door_shadow().is_none());

        // Nature is bulldozed, buildings and roads pieces are not
        assert!(tile(&tileset, FIRSTOSTRUCT, 1).bulldozed);
        assert!(tile(&tileset, FIRSTSHADOW, 1).bulldozed);
        assert!(tile(&tileset, SECONDFULLSTRUCT, 1).bulldozed);
        assert!(tile(&tileset, DEBRISROCKS, 1).bulldozed);
        assert!(tile(&tileset, FIRSTISTRUCT, 1).bulldozed);
        assert!(!tile(&tileset, THIRDFULLSTRUCT, 1).bulldozed);
        assert!(!tile(&tileset, DEBRIS2MISC, 1).bulldozed);
        assert!(!tile(&tileset, FIRSTWALL, 1).bulldozed);
        assert!(!tile(&tileset, ROADPIECES, 1).bulldozed);
    }

    #[test]
    fn building_palettes_follow_the_selection_window() {
        let tileset = test_tileset();
        let entries = |palette| -> Vec<(String, u16)> {
            tileset
                .building_palette(palette)
                .iter()
                .map(|t| (t.type_name.clone(), t.sub_index))
                .collect()
        };
        let room = entries(BuildingPalette::Room);
        assert_eq!(room.len(), 4 + 4 + 4 + 2);
        assert_eq!(room[0], ("WALL1".to_owned(), 1));
        assert_eq!(room[13], ("SROOF2".to_owned(), 1));
        let walls = entries(BuildingPalette::Walls);
        assert_eq!(walls.len(), 4 * 33);
        assert_eq!(
            walls[30..33],
            [
                ("WALL1".to_owned(), 31),
                ("WALL1".to_owned(), 56),
                ("WALL1".to_owned(), 57)
            ]
        );
        let doors = entries(BuildingPalette::Doors);
        let subs: Vec<u16> = doors[..8].iter().map(|(_, s)| *s).collect();
        assert_eq!(subs, [1, 5, 6, 10, 11, 15, 16, 20]);
        assert_eq!(entries(BuildingPalette::Windows).len(), 4 * 12);
        let roofs = entries(BuildingPalette::Roofs);
        assert_eq!(roofs.len(), 4 * 14 + 2 * 20 + 4 * 4 + 2 * 12);
        assert_eq!(roofs[96], ("WALL1".to_owned(), 32));
        assert_eq!(
            entries(BuildingPalette::SmartRoofs)[3],
            ("ROOF4".to_owned(), 10)
        );
        let broken = entries(BuildingPalette::BrokenWalls);
        assert_eq!(broken.len(), 4 * 10 + 4 * 8 + 4 * 8);
        assert_eq!(broken[40], ("WALL1".to_owned(), 48));
        assert_eq!(broken.last(), Some(&("WALL4".to_owned(), 65)));
        let decals = entries(BuildingPalette::Decals);
        assert_eq!(decals.len(), 8 * 10 + 21);
        assert_eq!(decals.last().unwrap().0, "FIRSTSWITCHES");
        assert_eq!(entries(BuildingPalette::SmartWalls).len(), 4);
        let smart_doors = entries(BuildingPalette::SmartDoors);
        assert_eq!(
            smart_doors[..2],
            [("DOOR1".to_owned(), 1), ("DOOR1".to_owned(), 5)]
        );
        let windows = entries(BuildingPalette::SmartWindows);
        assert_eq!(
            windows.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
            [45, 46, 47]
        );
        let smart_broken = entries(BuildingPalette::SmartBrokenWalls);
        assert_eq!(
            smart_broken.iter().map(|(_, s)| *s).collect::<Vec<_>>(),
            [50, 51, 63, 65]
        );
    }

    #[test]
    fn the_building_fixture_has_wall_orientations() {
        let tileset = crate::tileset::tests::building_tileset();
        let orientation =
            |tile_type, sub_index| tile(&tileset, tile_type, sub_index).wall_orientation();
        assert_eq!(orientation(FIRSTWALL, 1), Orientation::OUTSIDE_RIGHT);
        assert_eq!(orientation(FIRSTWALL, 4), Orientation::OUTSIDE_LEFT);
        assert_eq!(orientation(FIRSTWALL, 7), Orientation::INSIDE_RIGHT);
        assert_eq!(orientation(FIRSTWALL, 10), Orientation::INSIDE_LEFT);
        assert!(orientation(FIRSTWALL, 10).is_horizontal());
        assert!(orientation(FIRSTWALL, 7).is_vertical());
        assert_eq!(orientation(FIRSTDOOR, 16), Orientation::INSIDE_LEFT);
        let wall = tile(&tileset, FOURTHWALL, 36);
        let structure = wall.structure.as_ref().unwrap();
        assert!(structure
            .flags
            .contains(crate::tileset::StructureFlags::WALLNWINDOW));
        assert_eq!(structure.wall_orientation, Orientation::INSIDE_LEFT);
        assert!(wall.animation.is_none());
        let open = tile(&tileset, FIRSTDOOR, 5).structure.as_ref().unwrap();
        assert!(open
            .flags
            .contains(crate::tileset::StructureFlags::OPEN | crate::tileset::StructureFlags::DOOR));
        assert!(tile(&tileset, FIRSTWALL, 30).structure.is_none());
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
        let aux: Vec<_> = (0..12).map(|_| aux_bytes(0, 0, 0)).collect();
        let mut ostruct = surface("ostruct.sti", 12);
        ostruct.structure =
            Some(Ja2StructureFile::parse(&jsd_bytes(12, Some((&aux, &[])), &structures)).unwrap());
        surfaces[FIRSTOSTRUCT] = Some(ostruct);
        let tileset = build(surfaces);

        let structure_id = tileset.index(FIRSTOSTRUCT, 2).unwrap();
        let shadow_id = tileset.index(FIRSTSHADOW, 2).unwrap();
        let structure = tileset.tile(structure_id).unwrap();
        let shadow = tileset.tile(shadow_id).unwrap();
        let buddy = structure.shadow_buddy.as_ref().unwrap();
        assert!(Arc::ptr_eq(buddy, shadow));
        assert_eq!(buddy.id, shadow_id);
        assert!(shadow.shadow_buddy.is_none());
        assert_eq!(
            tileset
                .structure(tileset.index(FIRSTOSTRUCT, 1).unwrap())
                .unwrap()
                .tiles
                .len(),
            2
        );
        assert!(tileset.structure(structure_id).is_none());

        // The shadow is animated like its structure, without frames of its own
        let animation = structure.animation.as_ref().unwrap();
        assert_eq!(
            animation.frames,
            vec![structure_id - 1, structure_id, structure_id + 1]
        );
        assert_eq!(shadow.animation, Some(TileAnimation::default()));
        assert_eq!(tileset.current_frame(shadow_id), None);

        // Foliage is dynamic, and so is its shadow
        assert!(tile(&tileset, FIRSTFULLSHADOW, 2).dynamic);
        assert!(!tile(&tileset, FIRSTFULLSHADOW, 3).dynamic);
        // Walls have no buddy
        assert!(tile(&tileset, FIRSTWALL, 1).shadow_buddy.is_none());
    }

    #[test]
    fn wall_orientation_from_aux_data() {
        let mut surfaces = full_surfaces();
        let aux: Vec<_> = (0..65).map(|i| aux_bytes(i % 2 + 1, 0, 0)).collect();
        let mut wall = surface("wall.sti", 65);
        wall.structure =
            Some(Ja2StructureFile::parse(&jsd_bytes(65, Some((&aux, &[])), &[])).unwrap());
        surfaces[FIRSTWALL] = Some(wall);
        let tileset = build(surfaces);
        let orientation = |sub_index| match &tile(&tileset, FIRSTWALL, sub_index).kind {
            TileKind::Wall(wall) => wall.orientation,
            kind => panic!("not a wall: {:?}", kind),
        };
        assert_eq!(orientation(1), Orientation::INSIDE_LEFT);
        assert_eq!(orientation(2), Orientation::INSIDE_RIGHT);
    }

    #[test]
    fn missing_surfaces() {
        let mut surfaces = full_surfaces();
        surfaces[SECONDTEXTURE] = None;
        let tileset = build(surfaces);
        assert!(tileset.surface_of_type(SECONDTEXTURE).is_none());
        assert_eq!(tileset.surface_slots[SECONDTEXTURE], None);
        assert_eq!(tileset.index(SECONDTEXTURE, 1), None);
        assert_eq!(tileset.index(FIRSTTEXTURE, 36), None);
        assert!(tileset.tile(35).is_none());
        assert_eq!(tileset.tile_slots[35], None);
        assert_eq!(tileset.index(THIRDTEXTURE, 1), Some(70));
        assert_eq!(tileset.tile(70).unwrap().id, 70);
        let third = tileset.surface_of_type(THIRDTEXTURE).unwrap();
        assert!(Arc::ptr_eq(tileset.surface(third.id).unwrap(), third));
        assert!(Arc::ptr_eq(
            &tile(&tileset, THIRDTEXTURE, 1).image.surface,
            third
        ));
        assert_eq!(tileset.surfaces.len(), NUMBER_OF_TILE_TYPES - 1);
        assert_eq!(tileset.tiles.len(), 3125 - 35);
    }

    #[test]
    fn underflow_fillers() {
        let mut surfaces = full_surfaces();
        surfaces[SECONDTEXTURE] = Some(surface("b.sti", 3));
        let tileset = build(surfaces);
        let last = tile(&tileset, SECONDTEXTURE, 3);
        assert_eq!(last.image.sub_image, 2);
        assert!(!last.image.underflow_filler);
        let filler = tile(&tileset, SECONDTEXTURE, 4);
        assert_eq!(filler.image.sub_image, 0);
        assert!(filler.image.underflow_filler);
        assert_eq!(filler.image.surface.filename, "b.sti");
    }

    #[test]
    fn animations() {
        let mut surfaces = full_surfaces();
        let aux: Vec<_> = (0..12).map(|_| aux_bytes(0, 0, 0)).collect();
        let mut ostruct = surface("ostruct.sti", 12);
        ostruct.structure =
            Some(Ja2StructureFile::parse(&jsd_bytes(12, Some((&aux, &[])), &[])).unwrap());
        surfaces[FIRSTOSTRUCT] = Some(ostruct);
        let mut tileset = build(surfaces);

        // Frame 1 of 3 at the start
        let id = tileset.index(FIRSTOSTRUCT, 2).unwrap();
        assert_eq!(tileset.current_frame_number(id), 1);
        assert_eq!(tileset.current_frame(id), Some(id));
        tileset.advance_animations();
        assert_eq!(tileset.current_frame_number(id), 2);
        assert_eq!(tileset.current_frame(id), Some(id + 1));
        tileset.advance_animations();
        assert_eq!(tileset.current_frame_number(id), 0);
        assert_eq!(tileset.current_frame(id), Some(id - 1));

        // Doors don't animate on their own
        let door = tileset.index(FIRSTDOOR, 1).unwrap();
        assert_eq!(tileset.current_frame(door), None);
        assert_eq!(tileset.current_frame_number(door), 0);
    }
}
