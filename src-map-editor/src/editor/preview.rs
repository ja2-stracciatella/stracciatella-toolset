//! Shows what a click in the objects mode, with the roads or cliffs tool of the terrain mode or
//! with a single tile tool of the buildings mode places or removes
//!
//! Objects are picked at random, so the preview runs the placement with a copy of the random
//! numbers of the document: the tinted sprites are exactly what the next click places. Roads show
//! the pieces of the road macro around the cursor, cliffs the piece and its overhanging edge. While erasing, what the click removes is
//! tinted red. The building tools are applied to the map and undone again (`dry_run`), and the
//! tiles they add and remove are shown.

use std::{collections::HashMap, sync::Arc};

use bevy::input::mouse::MouseButton;

use bevy::{
    asset::RenderAssetUsages,
    image::ImageSampler,
    prelude::*,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
    sprite::Anchor,
};

use crate::{
    components::map::{game_to_world, GridPosition},
    map::MapTile,
    map::{GridNo, Level, TacticalMap},
    render::{
        atlas::SpriteSheets,
        draw_list::{
            ground_draw_position, map_top_y, map_world_y, PaletteKey, SpriteKey, WALL_HEIGHT,
        },
        world::MapWorld,
        RenderOptions,
    },
    states::{loading::MapAssets, AppState},
    systems::cursor::CursorInfo,
    tileset::{StciSheet, Tile, TileIndex, Tileset},
};

use super::{
    brush::BrushArea,
    cliffs::{is_cliff, is_cliff_hang, is_cliff_shadow, planned_cliff},
    commands::CurrentMode,
    document::{EditContext, EditorDocument},
    input::EditorStroke,
    mode::{EditorMode, ObjectsMode, TerrainMode, TerrainTool},
    placement::{is_debris, is_erased_structure, is_erased_with_structures},
    roads::{erased_road_pieces, road_macro_pieces},
};

/// Alpha of the preview sprites
const PREVIEW_ALPHA: u8 = 153;

/// The colour the preview sprites are tinted towards
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Tint {
    /// Green, the click places the object
    Place,
    /// Red, the click removes the object
    Remove,
}

impl Tint {
    fn color(self) -> [u8; 3] {
        match self {
            Tint::Place => [0, 255, 0],
            Tint::Remove => [255, 0, 0],
        }
    }
}

/// The tinted images of the preview and what it shows
#[derive(Resource, Default)]
pub struct PreviewSprites {
    images: HashMap<(TileIndex, Tint), Handle<Image>>,
    /// The tiles of the preview that is shown and whether it erases, `None` if none is shown
    shown: Option<(BrushArea, bool)>,
}

/// A sprite of the preview
#[derive(Component)]
pub struct PreviewSprite;

/// The sprites of the last map are not valid for the next one
pub fn reset_preview(mut preview: ResMut<PreviewSprites>) {
    *preview = PreviewSprites::default();
}

/// Blends the colours of a sprite halfway towards the tint, index 0 is transparent
fn tint_pixels(pixels: &[u8], palette: &[[u8; 3]; 256], tint: Tint) -> Vec<u8> {
    let target = tint.color();
    pixels
        .iter()
        .flat_map(|&index| {
            if index == 0 {
                return [0; 4];
            }
            let [r, g, b] = palette[usize::from(index)];
            let blend = |c: u8, t: u8| ((u16::from(c) + u16::from(t)) / 2) as u8;
            [
                blend(r, target[0]),
                blend(g, target[1]),
                blend(b, target[2]),
                PREVIEW_ALPHA,
            ]
        })
        .collect()
}

/// The layer of a node of the preview, in the order the game draws the layers of a row
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreviewLayer {
    Land,
    Object,
    Shadow,
    Struct,
    /// Drawn at the height of the roofs
    Roof,
    OnRoof,
}

/// A node of the map that the preview shows
struct PreviewNode {
    grid_no: GridNo,
    tile: Arc<Tile>,
    layer: PreviewLayer,
    tint: Tint,
}

/// What a click in the objects mode places on the tiles of the brush
fn planned_nodes(
    area: &BrushArea,
    mode: &ObjectsMode,
    document: &EditorDocument,
    tileset: &Tileset,
    world: &MapWorld,
) -> Vec<PreviewNode> {
    let mut random = document.random.clone();
    mode.plan_objects(&mut random, tileset, area)
        .into_iter()
        .filter(|(grid_no, tile)| {
            // Structures overlapping others of the same plan are not checked
            is_debris(tile)
                || tile.structure.as_ref().is_none_or(|structure| {
                    world.okay_to_add_structure_to_world(*grid_no, Level::Ground, structure)
                })
        })
        .map(|(grid_no, tile)| PreviewNode {
            grid_no,
            layer: if is_debris(&tile) {
                PreviewLayer::Object
            } else {
                PreviewLayer::Struct
            },
            tile,
            tint: Tint::Place,
        })
        .collect()
}

/// The pieces of the road macro that a click with the roads tool places
fn planned_road_nodes(area: &BrushArea, mode: &TerrainMode, tileset: &Tileset) -> Vec<PreviewNode> {
    let Some(first) = area.first() else {
        return vec![];
    };
    road_macro_pieces(tileset, first, mode.road)
        .into_iter()
        .map(|(grid_no, tile)| PreviewNode {
            grid_no,
            tile,
            layer: PreviewLayer::Object,
            tint: Tint::Place,
        })
        .collect()
}

/// The cliff pieces that a click with the cliffs tool places, with their overhanging edges
fn planned_cliff_nodes(
    area: &BrushArea,
    mode: &TerrainMode,
    map: &TacticalMap,
    tileset: &Tileset,
) -> Vec<PreviewNode> {
    let mut nodes = vec![];
    for grid_no in area.tiles() {
        let Some((cliff, hang)) = planned_cliff(map, tileset, grid_no, mode.cliff) else {
            continue;
        };
        let node = |tile, layer| PreviewNode {
            grid_no,
            tile,
            layer,
            tint: Tint::Place,
        };
        nodes.push(node(cliff, PreviewLayer::Struct));
        nodes.extend(hang.map(|hang| node(hang, PreviewLayer::Object)));
    }
    nodes
}

/// What erasing with the cliffs tool removes from the tiles of the brush, like `erase_cliffs`
fn erased_cliff_nodes(area: &BrushArea, map: &TacticalMap) -> Vec<PreviewNode> {
    let mut nodes = vec![];
    for grid_no in area.tiles() {
        let tile = &map.tiles[grid_no];
        let layers = [
            (
                &tile.structs,
                PreviewLayer::Struct,
                is_cliff as fn(&Tile) -> bool,
            ),
            (&tile.shadows, PreviewLayer::Shadow, is_cliff_shadow),
            (&tile.objects, PreviewLayer::Object, is_cliff_hang),
        ];
        for (tiles, layer, matches) in layers {
            nodes.extend(tiles.iter().filter(|t| matches(t)).map(|t| PreviewNode {
                grid_no,
                tile: t.clone(),
                layer,
                tint: Tint::Remove,
            }));
        }
    }
    nodes
}

/// What erasing with the roads tool removes from the tiles of the brush, like `erase_roads`
fn erased_road_nodes(area: &BrushArea, map: &TacticalMap, tileset: &Tileset) -> Vec<PreviewNode> {
    area.tiles()
        .flat_map(|grid_no| erased_road_pieces(map, tileset, grid_no))
        .map(|(grid_no, tile)| PreviewNode {
            grid_no,
            tile,
            layer: PreviewLayer::Object,
            tint: Tint::Remove,
        })
        .collect()
}

/// What erasing in the objects mode removes from the tiles of the brush, like
/// `erase_debris` and `erase_structures`
fn erased_nodes(area: &BrushArea, map: &TacticalMap) -> Vec<PreviewNode> {
    let mut nodes = vec![];
    for grid_no in area.tiles() {
        let tile = &map.tiles[grid_no];
        let objects = tile
            .objects
            .iter()
            .filter(|t| is_debris(t) || is_erased_with_structures(t));
        nodes.extend(objects.map(|t| PreviewNode {
            grid_no,
            tile: t.clone(),
            layer: PreviewLayer::Object,
            tint: Tint::Remove,
        }));
        let structs = tile.structs.iter().filter(|t| is_erased_structure(t));
        nodes.extend(structs.map(|t| PreviewNode {
            grid_no,
            tile: t.clone(),
            layer: PreviewLayer::Struct,
            tint: Tint::Remove,
        }));
    }
    nodes
}

/// The layers of a map tile with the preview layers of their tiles
fn layers(tile: &MapTile) -> [(PreviewLayer, &[Arc<Tile>]); 6] {
    [
        (PreviewLayer::Land, &tile.land),
        (PreviewLayer::Object, &tile.objects),
        (PreviewLayer::Shadow, &tile.shadows),
        (PreviewLayer::Struct, &tile.structs),
        (PreviewLayer::Roof, &tile.roofs),
        (PreviewLayer::OnRoof, &tile.on_roofs),
    ]
}

/// The tiles of `from` that are not in `other`, as often as they are missing
fn missing(from: &[Arc<Tile>], other: &[Arc<Tile>]) -> Vec<Arc<Tile>> {
    let mut other: Vec<TileIndex> = other.iter().map(|tile| tile.id).collect();
    from.iter()
        .filter(|tile| match other.iter().position(|&id| id == tile.id) {
            Some(index) => {
                other.swap_remove(index);
                false
            }
            None => true,
        })
        .cloned()
        .collect()
}

/// The nodes an edit added to and removed from a tile
fn changed_nodes(grid_no: GridNo, before: &MapTile, after: &MapTile) -> Vec<PreviewNode> {
    let mut nodes = vec![];
    for ((layer, before), (_, after)) in layers(before).into_iter().zip(layers(after)) {
        for (tint, tiles) in [
            (Tint::Place, missing(after, before)),
            (Tint::Remove, missing(before, after)),
        ] {
            nodes.extend(tiles.into_iter().map(|tile| PreviewNode {
                grid_no,
                tile,
                layer,
                tint,
            }));
        }
    }
    nodes
}

/// What an edit adds and removes: it is applied to the map and undone again
///
/// The map is restored from the undo record of the edit, the world from a copy, and the random
/// numbers and the undo stack of the document are kept, so nothing changes.
fn dry_run(
    map: &mut TacticalMap,
    world: &mut MapWorld,
    tileset: &Tileset,
    document: &mut EditorDocument,
    edit: impl FnOnce(&mut EditContext),
) -> Vec<PreviewNode> {
    let saved_world = world.clone();
    let saved_random = document.random.clone();
    let saved_background = document.background.clone();
    let saved_undo = std::mem::take(&mut document.undo);
    let saved_edge_points_dirty = document.edge_points_dirty;
    let mut context = EditContext::new(map, world, tileset, document);
    edit(&mut context);
    drop(context);
    let command = document.undo.pop().unwrap_or_default();
    let mut nodes = vec![];
    for (grid_no, before) in command.tiles {
        nodes.extend(changed_nodes(grid_no, &before, &map.tiles[grid_no]));
        map.tiles[grid_no] = before;
    }
    *world = saved_world;
    document.random = saved_random;
    document.background = saved_background;
    document.undo = saved_undo;
    document.edge_points_dirty = saved_edge_points_dirty;
    nodes
}

/// Spawns tinted sprites of the objects or roads the next click places or removes
#[allow(clippy::too_many_arguments)]
pub fn update_preview(
    mut commands: Commands,
    mode: Res<CurrentMode>,
    cursor: Res<CursorInfo>,
    stroke: Res<EditorStroke>,
    mut map: ResMut<TacticalMap>,
    mut world: ResMut<MapWorld>,
    tileset: Res<Tileset>,
    mut document: ResMut<EditorDocument>,
    (map_assets, sheets, options): (Res<MapAssets>, Res<Assets<StciSheet>>, Res<RenderOptions>),
    mut images: ResMut<Assets<Image>>,
    mut preview: ResMut<PreviewSprites>,
    sprites: Query<Entity, With<PreviewSprite>>,
) {
    let previewed = match &mode.0 {
        EditorMode::Terrain(TerrainMode {
            tool: TerrainTool::Roads | TerrainTool::Cliffs,
            ..
        })
        | EditorMode::Objects(_) => {
            // They don't edit the roof floor
            (options.floor != Level::Roof).then_some(&mode.0)
        }
        EditorMode::Buildings(buildings) if buildings.tool.previews() => Some(&mode.0),
        _ => None,
    };
    let smoothing = map.info.editor_smoothing;
    let wanted = previewed.zip(cursor.tile).and_then(|(previewed, tile)| {
        let brush = previewed.brush(&tileset, smoothing)?;
        let on_release = brush.applies_on_release();
        let erase = previewed.erases();
        // What was just painted is already on the map
        if stroke.active && !on_release && !erase {
            return None;
        }
        let anchor = stroke.anchor.filter(|_| stroke.active && on_release);
        Some((brush.region(anchor, tile), erase))
    });
    let changed =
        mode.is_changed() || options.is_changed() || map.is_changed() || document.is_changed();
    if !changed && wanted == preview.shown {
        return;
    }
    for entity in &sprites {
        commands.entity(entity).despawn();
    }
    preview.shown = wanted.clone();
    let (Some(previewed), Some((area, erase))) = (previewed, wanted) else {
        return;
    };

    let mut nodes = match previewed {
        EditorMode::Terrain(terrain) if terrain.tool == TerrainTool::Roads => {
            if erase {
                erased_road_nodes(&area, &map, &tileset)
            } else {
                planned_road_nodes(&area, terrain, &tileset)
            }
        }
        EditorMode::Terrain(terrain) if terrain.tool == TerrainTool::Cliffs => {
            if erase {
                erased_cliff_nodes(&area, &map)
            } else {
                planned_cliff_nodes(&area, terrain, &map, &tileset)
            }
        }
        EditorMode::Objects(_) if erase => erased_nodes(&area, &map),
        EditorMode::Objects(objects) => planned_nodes(&area, objects, &document, &tileset, &world),
        EditorMode::Buildings(buildings) => dry_run(
            // The preview doesn't change the map, nothing has to catch up with it
            map.bypass_change_detection(),
            world.bypass_change_detection(),
            &tileset,
            document.bypass_change_detection(),
            |context| buildings.apply(context, &area, MouseButton::Left),
        ),
        EditorMode::View
        | EditorMode::Terrain(_)
        | EditorMode::Soldiers(_)
        | EditorMode::Items(_)
        | EditorMode::Lights(_)
        | EditorMode::Traversal(_) => vec![],
    };
    let top_y = map_top_y(world.restricted_scroll);
    let order = |node: &PreviewNode| {
        let GridPosition { x, y } = node.grid_no.position();
        (map_world_y(i32::from(x), i32::from(y), top_y), node.layer)
    };
    nodes.sort_by_key(order);

    let sheets = SpriteSheets {
        tileset: &tileset,
        surface_sheets: map_assets
            .surface_sheets
            .iter()
            .map(|sheet| sheets.get(sheet))
            .collect(),
        soldier_sheets: vec![],
        soldier_palettes: vec![],
    };
    for (index, node) in nodes.iter().enumerate() {
        let key = SpriteKey::Tile(node.tile.id);
        let Some(geometry) = sheets.geometry(key) else {
            continue;
        };
        let tint = node.tint;
        let image = match preview.images.get(&(node.tile.id, tint)) {
            Some(image) => image.clone(),
            None => {
                let (Some(pixels), Some(palette)) = (
                    sheets.pixels(key),
                    sheets.palette(PaletteKey::Tile(node.tile.image.surface.id)),
                ) else {
                    continue;
                };
                let mut image = Image::new(
                    Extent3d {
                        width: geometry.size.x,
                        height: geometry.size.y,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    tint_pixels(pixels, palette, tint),
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                );
                image.sampler = ImageSampler::nearest();
                let image = images.add(image);
                preview.images.insert((node.tile.id, tint), image.clone());
                image
            }
        };
        let position = node.grid_no.position();
        let mut draw_position =
            ground_draw_position(position, &world.tiles[node.grid_no], &node.tile);
        if matches!(node.layer, PreviewLayer::Roof | PreviewLayer::OnRoof) {
            draw_position.y -= WALL_HEIGHT;
        }
        let top_left = game_to_world(draw_position + geometry.offset);
        // Above the regions of the map at z 0, in the order the game draws them
        let z = 1. + index as f32 * 0.001;
        commands.spawn((
            PreviewSprite,
            DespawnOnExit(AppState::Map),
            Sprite::from_image(image),
            Anchor::TOP_LEFT,
            Transform::from_translation(top_left.extend(z)),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        components::map::GridPosition,
        editor::{
            brush::Brush,
            document::tests::{grid, Fixture},
            mode::EditorMode,
            placement::{erase_debris, erase_structures, paste_debris, paste_structure},
            roads::is_road_piece,
        },
        file_formats::ja2_tile_types::{DEBRISROCKS, FIRSTFULLSTRUCT},
        tileset::tests::structure_tileset,
    };

    #[test]
    fn dry_runs_show_the_changes_and_keep_the_map() {
        use crate::editor::{
            building::tests::{add_room, building_fixture, rect},
            smart::paste_smart_door,
        };
        let mut fixture = building_fixture();
        add_room(&mut fixture, rect(50, 50, 52, 52));
        let before: Vec<TileIndex> = fixture.map.tiles[grid(52, 51)]
            .structs
            .iter()
            .map(|t| t.id)
            .collect();
        let random = fixture.document.random.clone();
        let depth = fixture.document.undo.depth();
        let nodes = dry_run(
            &mut fixture.map,
            &mut fixture.world,
            &fixture.tileset,
            &mut fixture.document,
            |context| paste_smart_door(context, grid(52, 51), 0, false),
        );
        let mut shown: Vec<_> = nodes
            .iter()
            .map(|n| (n.tile.type_name.as_str(), n.layer, n.tint))
            .collect();
        shown.sort_by_key(|&(name, _, _)| name);
        assert_eq!(
            shown,
            [
                ("DOOR1", PreviewLayer::Struct, Tint::Place),
                ("WALL1", PreviewLayer::Struct, Tint::Remove)
            ]
        );
        let after: Vec<TileIndex> = fixture.map.tiles[grid(52, 51)]
            .structs
            .iter()
            .map(|t| t.id)
            .collect();
        assert_eq!(after, before);
        assert_eq!(fixture.world.tiles[grid(52, 51)].structs.len(), 1);
        assert_eq!(fixture.document.undo.depth(), depth);
        assert_eq!(
            fixture.document.random.clone().next(1000),
            random.clone().next(1000)
        );
    }

    #[test]
    fn tints_blend_towards_the_colour() {
        let mut palette = [[0; 3]; 256];
        palette[1] = [100, 100, 100];
        let pixels = tint_pixels(&[0, 1], &palette, Tint::Place);
        assert_eq!(pixels, [0, 0, 0, 0, 50, 177, 50, PREVIEW_ALPHA]);
        let pixels = tint_pixels(&[1], &palette, Tint::Remove);
        assert_eq!(pixels, [177, 50, 50, PREVIEW_ALPHA]);
    }

    #[test]
    fn the_erase_preview_shows_what_erasing_removes() {
        let mut fixture = Fixture::with_tileset(structure_tileset());
        let rock = fixture.tileset.index(DEBRISROCKS, 1).unwrap();
        let rock = fixture.tileset.tile(rock).unwrap().clone();
        let tree = fixture.tileset.index(FIRSTFULLSTRUCT, 1).unwrap();
        let tree = fixture.tileset.tile(tree).unwrap().clone();
        let mut context = fixture.context();
        paste_debris(&mut context, grid(50, 50), rock);
        paste_structure(&mut context, grid(51, 50), tree);
        let road = context.tile("ROADPIECES", 1).unwrap();
        let junk = context.tile("MISC2", 1).unwrap();
        context.add_object_to_head(grid(50, 50), road);
        context.add_object_to_tail(grid(51, 50), junk);
        context.finish();

        let area = Brush::Square { size: 3 }.region(None, GridPosition::new(51, 51));
        let names = |nodes: Vec<PreviewNode>| {
            let mut names: Vec<_> = nodes
                .iter()
                .map(|n| {
                    let is_struct = n.layer == PreviewLayer::Struct;
                    (n.grid_no, n.tile.type_name.clone(), is_struct)
                })
                .collect();
            names.sort();
            names
        };
        let shown = names(erased_nodes(&area, &fixture.map));
        assert_eq!(shown.len(), 3, "{shown:?}");
        assert!(shown.iter().all(|(_, name, _)| name != "ROADPIECES"));
        assert!(shown
            .iter()
            .any(|(g, _, is_struct)| *g == grid(51, 50) && *is_struct));

        // Erasing removes exactly these
        let mut context = fixture.context();
        for grid_no in area.tiles() {
            erase_debris(&mut context, grid_no);
            erase_structures(&mut context, grid_no);
        }
        context.finish();
        assert!(erased_nodes(&area, &fixture.map).is_empty());
        assert_eq!(fixture.map.tiles[grid(50, 50)].objects.len(), 1);
    }

    #[test]
    fn the_cliff_preview_shows_the_piece_and_its_hang() {
        let mut fixture = Fixture::new();
        let json = r#"{"mode":"terrain","tool":"cliffs","cliff":4}"#;
        let EditorMode::Terrain(mode) = serde_json::from_str(json).unwrap() else {
            unreachable!()
        };
        let area = mode.brush().region(None, GridPosition::new(50, 50));
        let nodes = |nodes: Vec<PreviewNode>| -> Vec<_> {
            nodes
                .into_iter()
                .map(|n| (n.grid_no, n.tile.type_name.clone(), n.layer, n.tint))
                .collect()
        };
        let g = grid(50, 50);
        let planned = vec![
            (
                g,
                "FIRSTCLIFF".to_owned(),
                PreviewLayer::Struct,
                Tint::Place,
            ),
            (
                g,
                "FIRSTCLIFFHANG".to_owned(),
                PreviewLayer::Object,
                Tint::Place,
            ),
        ];
        assert_eq!(
            nodes(planned_cliff_nodes(
                &area,
                &mode,
                &fixture.map,
                &fixture.tileset
            )),
            planned
        );

        let mut context = fixture.context();
        mode.apply(&mut context, &area);
        context.finish();
        // The piece is already there
        assert!(planned_cliff_nodes(&area, &mode, &fixture.map, &fixture.tileset).is_empty());
        let erased = vec![
            (
                g,
                "FIRSTCLIFF".to_owned(),
                PreviewLayer::Struct,
                Tint::Remove,
            ),
            (
                g,
                "FIRSTCLIFFHANG".to_owned(),
                PreviewLayer::Object,
                Tint::Remove,
            ),
        ];
        assert_eq!(nodes(erased_cliff_nodes(&area, &fixture.map)), erased);
    }

    fn roads_mode(road: u16, erase: bool) -> TerrainMode {
        let json = format!(r#"{{"mode":"terrain","tool":"roads","road":{road},"erase":{erase}}}"#);
        match serde_json::from_str(&json).unwrap() {
            EditorMode::Terrain(mode) => mode,
            _ => unreachable!(),
        }
    }

    #[test]
    fn the_road_preview_shows_the_pieces_of_the_macro() {
        let mut fixture = Fixture::new();
        let mode = roads_mode(5, false);
        let area = mode.brush().region(None, GridPosition::new(50, 50));
        let mut preview: Vec<_> = planned_road_nodes(&area, &mode, &fixture.tileset)
            .iter()
            .map(|n| (n.grid_no, n.tile.id))
            .collect();
        assert!(preview.len() > 1);

        let mut context = fixture.context();
        mode.apply(&mut context, &area);
        context.finish();
        let mut placed: Vec<_> = GridNo::all()
            .flat_map(|g| {
                let objects = fixture.map.tiles[g].objects.iter();
                objects.filter(|t| is_road_piece(t)).map(move |t| (g, t.id))
            })
            .collect();
        preview.sort();
        placed.sort();
        assert_eq!(preview, placed);

        // Erasing shows all pieces of the macro
        let erase = roads_mode(5, true);
        let (piece, _) = placed[0];
        let area = erase.brush().region(None, piece.position());
        let mut erased: Vec<_> = erased_road_nodes(&area, &fixture.map, &fixture.tileset)
            .iter()
            .map(|n| (n.grid_no, n.tile.id))
            .collect();
        erased.sort();
        assert_eq!(erased, placed);
        // Nothing without road pieces
        assert!(erased_road_nodes(
            &Brush::default().region(None, GridPosition::new(10, 10)),
            &fixture.map,
            &fixture.tileset
        )
        .is_empty());
    }
}
