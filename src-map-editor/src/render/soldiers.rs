//! Soldier placements of the map, drawn as standing soldiers
//!
//! Soldiers are shown like the editor creates them from the placements (`AddPlacementToWorld`,
//! `TacticalCreateSoldier`), in the first frame of their standing animation and without their
//! animation. The body type, the standing animation and the palette of a placement were chosen
//! when the map was converted (`map::placement`), with a random generator seeded with the
//! placement, so the choices of the game are the same every time. Soldiers without a detailed
//! placement have no equipment, so they don't hold a gun. Profiles of named characters are not
//! read.

use std::{collections::BTreeSet, sync::Arc};

use bevy::prelude::*;

use crate::{
    map::{GridNo, Level, SoldierPlacement},
    render::{
        draw_list::{
            map_top_y, map_world_y, Blitter, MercDraw, PaletteKey, SpriteKey, MERC_Z_LEVEL,
            ONROOF_Z_LEVEL, STRUCT_Z_LEVEL, WALL_HEIGHT, Z_SUBLAYERS,
        },
        palette::DEFAULT_SHADE_LEVEL,
        world::{MapWorld, MercNode, NodeLight},
        MapSoldiers,
    },
    soldier::{
        palette::ResolvedSoldierPalette,
        palette_files::{ColPalette, PaletteReplacements},
        standing_surface::StandingSurface,
    },
    tileset::StciSheet,
};

/// How much higher soldiers on roofs are drawn (`SECOND_LEVEL_Z_OFFSET`), also the height of the
/// roof floor for the cursor
pub const ROOF_HEIGHT_ADJUSTMENT: i32 = 58;

/// A soldier to draw
#[derive(Clone, Debug, PartialEq)]
pub struct Soldier {
    /// Index of its palette in `MapSoldiers`, also the key of its palettes in the atlas
    pub id: u16,
    pub grid_no: GridNo,
    pub surface: Arc<StandingSurface>,
    pub frame: u16,
    pub level: Level,
    /// Enemies and creatures glow instead of being lit (`bNeutral`, `bSide`)
    pub hostile: bool,
    pub vehicle: bool,
}

/// The loaded files that soldiers need
pub struct SoldierFiles<'a> {
    /// The sheet of each standing surface by id, `None` if it could not be loaded
    pub surfaces: Vec<Option<&'a StciSheet>>,
    pub replacements: Option<&'a PaletteReplacements>,
    /// The palette files of the body types by file name, see `SoldierBodyTypes::palette_files`
    pub monster_palettes: Vec<(&'static str, Option<&'a ColPalette>)>,
}

/// Creates the soldiers of the placements that are shown and adds them to the merc lists of
/// their tiles, their palettes are added to `soldiers`
///
/// The editor adds the soldiers team by team, in the order of the team ids.
pub fn add_soldiers(
    world: &mut MapWorld,
    placements: &[SoldierPlacement],
    shown: impl Fn(&SoldierPlacement) -> bool,
    files: &SoldierFiles,
    soldiers: &mut MapSoldiers,
) {
    let mut placements: Vec<_> = placements
        .iter()
        .filter(|placement| placement.team.placed_by_editor && shown(placement))
        .collect();
    placements.sort_by_key(|placement| placement.team.id);
    for placement in placements {
        let Some(soldier) = create_soldier(placement, world, files, soldiers) else {
            continue;
        };
        let grid_no = soldier.grid_no;
        // `AddMercToHead`
        world.tiles[grid_no].mercs.insert(
            0,
            MercNode {
                soldier: Arc::new(soldier),
                light: NodeLight::default(),
            },
        );
    }
}

/// Replaces the soldiers of the world with the ones of the placements that are shown, returns
/// the tiles whose soldiers changed
pub fn sync_soldiers(
    world: &mut MapWorld,
    placements: &[SoldierPlacement],
    shown: impl Fn(&SoldierPlacement) -> bool,
    files: &SoldierFiles,
    soldiers: &mut MapSoldiers,
) -> BTreeSet<GridNo> {
    let mut touched: BTreeSet<GridNo> = world
        .tiles
        .iter()
        .filter(|(_, tile)| !tile.mercs.is_empty())
        .map(|(grid_no, _)| grid_no)
        .collect();
    for &grid_no in &touched {
        world.tiles[grid_no].mercs.clear();
    }
    add_soldiers(world, placements, shown, files, soldiers);
    for (grid_no, tile) in world.tiles.iter() {
        if !tile.mercs.is_empty() {
            touched.insert(grid_no);
        }
    }
    touched
}

fn create_soldier(
    placement: &SoldierPlacement,
    world: &MapWorld,
    files: &SoldierFiles,
    soldiers: &mut MapSoldiers,
) -> Option<Soldier> {
    let grid_no = placement.grid_no;
    let surface = &placement.surface;
    let sheet = (*files.surfaces.get(surface.id.index())?)?;
    let indexed = sheet.indexed.as_ref()?;

    // The first frame of the direction (`ConvertAniCodeToAniFrame`), vehicles only have one
    // frame per direction (`ANIM_DATA_FLAG_NOFRAMES`)
    let direction = u32::from(placement.direction as u8);
    let sprite_direction = match surface.directions {
        1 => 0,
        32 => (direction * 4 + 4) % 32,
        _ => (direction + 1) % 8,
    };
    let frames_per_direction = u32::from(sheet.frames_per_direction);
    let mut frame = frames_per_direction * sprite_direction;
    if frame as usize >= sheet.sub_images.len() {
        frame = 0;
    }

    // `CreateSoldierPalettes`
    let mut palette = *indexed.palette;
    match &placement.palette {
        ResolvedSoldierPalette::Own => {}
        ResolvedSoldierPalette::Substitution(file) => {
            let col = files
                .monster_palettes
                .iter()
                .find(|(name, _)| name == file)
                .and_then(|(_, col)| *col);
            if let Some(col) = col {
                palette = *col.0;
            }
        }
        ResolvedSoldierPalette::Generated(replacements) => {
            if let Some(files) = files.replacements {
                for &id in replacements {
                    files.apply(&mut palette, id);
                }
            }
        }
    }

    // Soldiers on roofs without a roof are put on the ground
    let flat_roof = flat_roof_above(world, grid_no);
    Some(Soldier {
        id: soldiers.palette_id(&palette),
        grid_no,
        surface: surface.clone(),
        frame: frame as u16,
        level: if flat_roof {
            placement.level
        } else {
            Level::Ground
        },
        hostile: placement.team.hostile,
        vehicle: placement.body.vehicle,
    })
}

/// Whether a tile has a flat roof that soldiers can stand on (`FlatRoofAboveGridNo`)
pub fn flat_roof_above(world: &MapWorld, grid_no: GridNo) -> bool {
    world.tiles[grid_no]
        .roofs
        .iter()
        .any(|node| node.tile.is_flat_roof())
}

/// The soldiers of a tile as draws, in the order of the merc list, after the world was lit
pub fn tile_merc_draws(world: &MapWorld, grid_no: GridNo) -> Vec<MercDraw> {
    let top_y = map_top_y(world.restricted_scroll);
    let tile = &world.tiles[grid_no];
    let position = grid_no.position();
    tile.mercs
        .iter()
        .map(|node| {
            let soldier = &node.soldier;
            let (x, y) = (i32::from(position.x), i32::from(position.y));
            let world_y = map_world_y(x, y, top_y);
            let height_adjustment = if soldier.level == Level::Roof {
                ROOF_HEIGHT_ADJUSTMENT
            } else {
                0
            };
            // The center of the tile
            let draw_position = position.tile_box() + IVec2::new(20, 10)
                - IVec2::new(0, i32::from(tile.height) + height_adjustment);
            let mut z = if soldier.vehicle {
                world_y * Z_SUBLAYERS + STRUCT_Z_LEVEL
            } else if height_adjustment > 0 {
                (world_y + WALL_HEIGHT + 20) * Z_SUBLAYERS + ONROOF_Z_LEVEL
            } else {
                world_y * Z_SUBLAYERS + MERC_Z_LEVEL
            };
            // Soldiers burn through structs, unless the tile says otherwise
            let blitter = if tile.no_burn_struct {
                z += 2;
                Blitter::TransShadowZNB
            } else {
                Blitter::TransShadowZNBObscured
            };
            // Hostile soldiers use their unshaded glow palette, the others are always
            // lighter than the scene
            let (palette, shade) = if soldier.hostile {
                (PaletteKey::SoldierUnbiased(soldier.id), DEFAULT_SHADE_LEVEL)
            } else {
                let shade = (node.light.shade & 0x0f)
                    .saturating_sub(2)
                    .max(DEFAULT_SHADE_LEVEL);
                (PaletteKey::Soldier(soldier.id), shade)
            };
            MercDraw {
                position: draw_position,
                sprite: SpriteKey::Soldier {
                    surface: soldier.surface.id.index() as u16,
                    frame: soldier.frame,
                },
                palette,
                shade,
                blitter,
                z: z as i16 as u16,
                on_roof: soldier.level == Level::Roof,
            }
        })
        .collect()
}
