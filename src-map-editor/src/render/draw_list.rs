//! The draws of a map in the order the game renders them
//!
//! Port of the traversal and the per-node setup of `RenderTiles` in `RenderWorld.cc`, as called by
//! `RenderStaticWorld` and `RenderDynamicWorld`: the level lists are drawn row by row, and the
//! layers of one `RenderTiles` call are interleaved within each row. Each draw records the
//! position, sprite, palette, blitter and z value, the GPU then replays the draws per pixel.
//!
//! Not reproduced are things that only happen while playing (redundant tile culling, revealed and
//! hidden nodes, the glow of enemies) and the game drawing tiles of other rows when its view
//! reaches past the map border.

use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    components::map::{GridPosition, JA2_MAP_SIZE},
    map::{GridNo, Level},
    render::{
        soldiers::tile_merc_draws,
        world::{LevelNode, MapWorld, NodeFlags, WorldTile},
    },
    soldier::team::TeamId,
    tileset::{SurfaceId, Tile, TileKind, Tileset, ViewLayer, ZStrip},
};

/// Height of a wall and of the roof level in pixels (`WALL_HEIGHT`)
pub const WALL_HEIGHT: i32 = 50;

/// Z levels within a row (`ZLevels`)
pub(crate) const Z_SUBLAYERS: i32 = 8;
const LAND_Z_LEVEL: i32 = 0;
const OBJECT_Z_LEVEL: i32 = 1;
const SHADOW_Z_LEVEL: i32 = 2;
pub(crate) const MERC_Z_LEVEL: i32 = 3;
pub(crate) const STRUCT_Z_LEVEL: i32 = 4;
const ROOF_Z_LEVEL: i32 = 5;
pub(crate) const ONROOF_Z_LEVEL: i32 = 6;
const TOPMOST_Z_LEVEL: i32 = 32767;

/// Outline colour of items on the ground (`gusNormalItemOutlineColor`)
pub const ITEM_OUTLINE_COLOR: [u8; 3] = [255, 255, 255];
/// Outline colour of items on roofs (`gusYellowItemOutlineColor`)
pub const ON_ROOF_ITEM_OUTLINE_COLOR: [u8; 3] = [255, 255, 0];

/// The blitters of the game that static map content uses, with the depth rules of
/// `VObject_Blitters.cc` and `RenderWorld.cc`
///
/// The numbers are used by the replay shader.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Blitter {
    /// `Transparent`: no z test or write
    Transparent = 0,
    /// `TransZ`: drawn where the z buffer is <= z, writes z
    TransZ = 1,
    /// `TransZNB`: drawn where the z buffer is <= z
    TransZNB = 2,
    /// `TransZInc`: z changes per z strip, drawn where the z buffer is < z, writes z
    TransZInc = 3,
    /// `TransZIncSameZBurnsThrough`: like `TransZInc`, but drawn where the z buffer is <= z
    TransZIncSameZBurnsThrough = 4,
    /// `ShadowZ`: darkens where the z buffer is < z, writes z
    ShadowZ = 5,
    /// `Shadow`: darkens without a z test
    Shadow = 6,
    /// `OutlineZ`: drawn where the z buffer is <= z, writes z. Index 254 is drawn in the outline
    /// colour without writing z.
    OutlineZ = 7,
    /// `OutlineZPixelateObscured`: where the z buffer is < z drawn and z written, elsewhere drawn
    /// in a checker pattern. Index 254 is drawn in the outline colour.
    OutlineZPixelateObscured = 8,
    /// `TransShadowZNB`: drawn where the z buffer is < z, index 254 darkens
    TransShadowZNB = 9,
    /// `TransShadowZNBObscured`: index 254 darkens where the z buffer is < z, the others are drawn
    /// where it is <= z and elsewhere in a checker pattern
    TransShadowZNBObscured = 10,
}

impl Blitter {
    pub fn uses_z_strips(self) -> bool {
        matches!(
            self,
            Blitter::TransZInc | Blitter::TransZIncSameZBurnsThrough
        )
    }
}

/// An image that a draw shows
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SpriteKey {
    /// The sub image of a tile of the tile database
    Tile(u16),
    /// A frame of a soldier animation surface, `surface` is a `StandingSurfaceId` index
    Soldier { surface: u16, frame: u16 },
}

/// The palette that a draw uses
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PaletteKey {
    /// The palette of a tileset surface
    Tile(SurfaceId),
    /// The palette of a soldier
    Soldier(u16),
    /// The palette of a soldier without the light colour of the map
    SoldierUnbiased(u16),
}

/// The sprite of a draw
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DrawSprite {
    Fixed(SpriteKey),
    /// The current frame of an animated tile (`ANIMATED_TILE`), the tile database index of the
    /// animation
    Animated(u16),
}

/// Offset and size of a sprite (`sOffsetX`, `sOffsetY`, `usWidth`, `usHeight`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpriteGeometry {
    pub offset: IVec2,
    pub size: UVec2,
}

/// Z strips of a sprite in the form the shader uses (`ZStripInfo`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct PackedZStrip {
    pub first_width: u8,
    pub initial_change: i8,
    /// Change of each strip after the first as 2 bit two's complement values, the lowest bits
    /// are the first strip. The game has room for 16 changes.
    pub changes: u32,
}

impl From<&ZStrip> for PackedZStrip {
    fn from(info: &ZStrip) -> Self {
        let changes = info
            .z_changes
            .iter()
            .take(16)
            .enumerate()
            .fold(0, |bits, (i, &change)| {
                bits | ((change as u32 & 0b11) << (2 * i))
            });
        Self {
            first_width: info.first_z_strip_width,
            initial_change: info.initial_z_change,
            changes,
        }
    }
}

impl PackedZStrip {
    /// Z value of a sprite column, `z` is the z value of the draw
    pub fn column_z(&self, z: u16, column: u32) -> u16 {
        const HALF_TILE: u32 = 20;
        let first = u32::from(self.first_width);
        let strip = if column < first {
            0
        } else {
            1 + (column - first) / HALF_TILE
        };
        let mut level = i32::from(z) + i32::from(self.initial_change) * Z_SUBLAYERS * 10;
        for i in 0..strip.min(16) {
            let change = ((self.changes >> (2 * i)) & 0b11) as i32;
            // Sign extend the 2 bit value
            let change = if change >= 2 { change - 4 } else { change };
            level += change * Z_SUBLAYERS * 10;
        }
        level as u16
    }
}

/// One blit of the game
#[derive(Clone, Debug, PartialEq)]
pub struct Draw {
    /// Position that the sprite offset is relative to, in game pixels
    pub position: IVec2,
    pub sprite: DrawSprite,
    pub palette: PaletteKey,
    /// Shade level of the palette, 0 to 15
    pub shade: u8,
    pub blitter: Blitter,
    pub z: u16,
    /// Set for the blitters that use z strips
    pub z_strip: Option<PackedZStrip>,
    /// Colour of index 254 for the outline blitters
    pub outline: [u8; 3],
    /// Area the draw can cover, for animations the area of all frames. Min inclusive, max
    /// exclusive, in game pixels.
    pub bounds: IRect,
    /// Tile the draw belongs to
    pub grid_no: GridNo,
}

/// Which layers of the map are drawn
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LayerVisibility {
    /// What the terrain mode places: land textures, water, roads, cliffs and their shadows, and
    /// the floors of buildings
    pub terrain: bool,
    /// What the objects mode places: debris, trees, rocks, misc junk, furniture and their
    /// shadows
    pub objects: bool,
    /// What the buildings mode places but the floors: walls, doors and their shadows, wall
    /// decorations, roofs and what is on them
    pub buildings: bool,
    pub items: bool,
    pub soldiers: bool,
    /// Which teams of the soldiers layer are drawn, edited and selected
    pub soldier_teams: VisibleTeams,
    /// The room numbers of the tiles, which are not part of the draws
    pub room_numbers: bool,
}

impl LayerVisibility {
    /// The layers without those that are not drawn with the map, a change of which doesn't
    /// change the draws. The soldiers of the shown teams are brought up to date by
    /// `sync_soldier_scene` instead of a rebuild.
    pub fn drawn(self) -> Self {
        Self {
            room_numbers: false,
            soldier_teams: VisibleTeams::default(),
            ..self
        }
    }

    /// Whether the tiles of a view layer are drawn, tiles without one always are
    pub fn shows(self, layer: Option<ViewLayer>) -> bool {
        match layer {
            Some(ViewLayer::Terrain) => self.terrain,
            Some(ViewLayer::Objects) => self.objects,
            Some(ViewLayer::Buildings) => self.buildings,
            None => true,
        }
    }
}

impl Default for LayerVisibility {
    fn default() -> Self {
        Self {
            terrain: true,
            objects: true,
            buildings: true,
            items: true,
            soldiers: true,
            soldier_teams: VisibleTeams::default(),
            room_numbers: false,
        }
    }
}

/// Which teams of the editor have their soldiers shown (`SetMercTeamVisibility`)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct VisibleTeams {
    pub enemy: bool,
    pub creature: bool,
    pub rebel: bool,
    pub civilian: bool,
}

impl Default for VisibleTeams {
    fn default() -> Self {
        Self {
            enemy: true,
            creature: true,
            rebel: true,
            civilian: true,
        }
    }
}

impl VisibleTeams {
    /// Whether the soldiers of a team are shown, those of the player's teams always are
    pub fn shows(&self, team: TeamId) -> bool {
        if team == TeamId::ENEMY {
            self.enemy
        } else if team == TeamId::CREATURE {
            self.creature
        } else if team == TeamId::MILITIA {
            self.rebel
        } else if team == TeamId::CIV {
            self.civilian
        } else {
            true
        }
    }
}

/// Options of the draw list
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DrawListOptions {
    pub layers: LayerVisibility,
    /// The roofs, what is on them and the topmost tiles are only drawn on the roof floor
    pub floor: Level,
    /// Whether the static shadows are drawn (`RENDER_FLAG_SHADOWS`, off from dusk on)
    pub shadows: bool,
}

/// Soldier that is drawn on a tile, see `soldiers.rs`
#[derive(Clone, Debug, PartialEq)]
pub struct MercDraw {
    /// Position that the frame offset is relative to, in game pixels
    pub position: IVec2,
    pub sprite: SpriteKey,
    pub palette: PaletteKey,
    pub shade: u8,
    pub blitter: Blitter,
    pub z: u16,
    /// Whether the soldier is on a roof (`bLevel > 0`), drawn with the high mercs
    pub on_roof: bool,
}

/// The render layers of the game (`RenderLayerID`)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderLayer {
    StaticLand,
    StaticObjects,
    StaticShadows,
    StaticStructs,
    StaticRoof,
    StaticOnRoof,
    StaticTopmost,
    DynamicObjects,
    DynamicShadows,
    DynamicMercs,
    DynamicStructs,
    DynamicRoof,
    DynamicHighMercs,
    DynamicOnRoof,
    DynamicTopmost,
}

impl RenderLayer {
    fn is_dynamic(self) -> bool {
        !matches!(
            self,
            RenderLayer::StaticLand
                | RenderLayer::StaticObjects
                | RenderLayer::StaticShadows
                | RenderLayer::StaticStructs
                | RenderLayer::StaticRoof
                | RenderLayer::StaticOnRoof
                | RenderLayer::StaticTopmost
        )
    }
}

/// Whether a `RenderTiles` call draws the nodes that show through structures
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PassKind {
    Normal,
    /// `TILES_OBSCURED`
    Obscured,
}

/// One `RenderTiles` call of `RenderStaticWorld` or `RenderDynamicWorld`
struct Pass {
    kind: PassKind,
    layers: &'static [RenderLayer],
}

/// The `RenderTiles` calls in the order of the game. Only drawn with shadows: the static shadows.
const PASSES: [Pass; 8] = {
    use RenderLayer::*;
    let normal = PassKind::Normal;
    [
        // RenderStaticWorld
        Pass {
            kind: normal,
            layers: &[StaticLand],
        },
        Pass {
            kind: normal,
            layers: &[StaticObjects],
        },
        Pass {
            kind: normal,
            layers: &[StaticShadows],
        },
        Pass {
            kind: normal,
            layers: &[StaticStructs, StaticRoof, StaticOnRoof, StaticTopmost],
        },
        Pass {
            kind: PassKind::Obscured,
            layers: &[StaticStructs, StaticOnRoof],
        },
        // RenderDynamicWorld. It starts with another obscured pass of the structs, which draws
        // nothing new because nothing was drawn since the one above.
        Pass {
            kind: normal,
            layers: &[DynamicObjects, DynamicShadows, DynamicMercs, DynamicStructs],
        },
        Pass {
            kind: normal,
            layers: &[DynamicRoof, DynamicHighMercs, DynamicOnRoof],
        },
        Pass {
            kind: normal,
            layers: &[DynamicTopmost],
        },
    ]
};
/// Index of the static shadow pass in `PASSES`
const STATIC_SHADOW_PASS: usize = 2;

/// Position of a draw in the draw list of the whole map
///
/// `RenderTiles` draws row by row, and within a row the layers of the pass one after the other,
/// from left to right. So the draws of a map are ordered by pass, row, layer, x and the position
/// in the node list of the tile, and the draws of each tile can be computed on their own.
fn draw_key(pass: usize, position: GridPosition, layer: usize, node: usize) -> u64 {
    let row = u64::from(position.x) + u64::from(position.y);
    ((pass as u64) << 48)
        | (row << 36)
        | ((layer as u64) << 32)
        | (u64::from(position.x) << 24)
        | node as u64
}

/// The draws of one tile with their positions in the draw list of the map
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TileDraws {
    /// In draw list order, with their `draw_key`
    pub draws: Vec<(u64, Draw)>,
    /// Union of the bounds of the draws, `None` without draws
    pub bounds: Option<IRect>,
}

/// Port of the per-tile part of `RenderTiles`
pub struct DrawContext<'a, S> {
    pub world: &'a MapWorld,
    pub tileset: &'a Tileset,
    /// Returns the geometry of a sprite, or `None` if it can't be drawn
    pub sprites: S,
    pub options: DrawListOptions,
}

impl<S: Fn(SpriteKey) -> Option<SpriteGeometry>> DrawContext<'_, S> {
    /// The draws of a tile in all passes
    pub fn tile_draws(&self, grid_no: GridNo) -> TileDraws {
        let position = grid_no.position();
        let tile = TileContext {
            world: self.world,
            tileset: self.tileset,
            sprites: &self.sprites,
            options: self.options,
            top_y: map_top_y(self.world.restricted_scroll),
            mercs: tile_merc_draws(self.world, grid_no),
        };
        let mut result = TileDraws::default();
        let mut draws = Vec::new();
        for (pass_index, pass) in PASSES.iter().enumerate() {
            if pass_index == STATIC_SHADOW_PASS && !self.options.shadows {
                continue;
            }
            for (layer_index, &layer) in pass.layers.iter().enumerate() {
                draws.clear();
                tile.render_tile(pass.kind, layer, position, &mut draws);
                for (node, draw) in draws.drain(..).enumerate() {
                    result.bounds = Some(match result.bounds {
                        Some(bounds) => bounds.union(draw.bounds),
                        None => draw.bounds,
                    });
                    let key = draw_key(pass_index, position, layer_index, node);
                    result.draws.push((key, draw));
                }
            }
        }
        result
    }
}

/// The draws of every tile of the map, the source of the draw list that is updated tile by tile
#[derive(Clone, Debug, Default)]
pub struct DrawCache {
    tiles: Vec<TileDraws>,
}

impl DrawCache {
    /// The draws of all tiles
    pub fn build<S: Fn(SpriteKey) -> Option<SpriteGeometry>>(context: &DrawContext<S>) -> Self {
        Self {
            tiles: GridNo::all()
                .map(|grid_no| context.tile_draws(grid_no))
                .collect(),
        }
    }

    /// Replaces the draws of a tile, returns whether they changed
    pub fn replace(&mut self, grid_no: GridNo, draws: TileDraws) -> bool {
        let old = &mut self.tiles[grid_no.index()];
        if *old == draws {
            return false;
        }
        *old = draws;
        true
    }

    /// Union of the bounds of all draws, empty without draws
    pub fn bounds(&self) -> IRect {
        self.tiles
            .iter()
            .filter_map(|tile| tile.bounds)
            .reduce(|a, b| a.union(b))
            .unwrap_or_default()
    }

    /// The draws of the map in draw list order
    pub fn all(&self) -> Vec<Draw> {
        let mut draws: Vec<&(u64, Draw)> = self.tiles.iter().flat_map(|tile| &tile.draws).collect();
        draws.sort_unstable_by_key(|(key, _)| *key);
        draws.into_iter().map(|(_, draw)| draw.clone()).collect()
    }
}

/// Builds the draw list of a map
///
/// `sprites` returns the geometry of a sprite, or `None` if it can't be drawn.
pub fn build_draw_list(
    world: &MapWorld,
    tileset: &Tileset,
    sprites: impl Fn(SpriteKey) -> Option<SpriteGeometry>,
    options: DrawListOptions,
) -> Vec<Draw> {
    DrawCache::build(&DrawContext {
        world,
        tileset,
        sprites,
        options,
    })
    .all()
}

/// What the per-node part of `RenderTiles` needs for one tile
struct TileContext<'a, S> {
    world: &'a MapWorld,
    tileset: &'a Tileset,
    sprites: &'a S,
    options: DrawListOptions,
    /// Top edge of the map in screen coordinates (`gsTopY`)
    top_y: i32,
    /// The soldiers of the tile in the order of the merc list
    mercs: Vec<MercDraw>,
}

/// Where a tile of the ground levels (land, objects, shadows and structs) is drawn: its tile box
/// raised by the terrain height, the sprite offset is relative to this
pub(crate) fn ground_draw_position(
    position: GridPosition,
    world_tile: &WorldTile,
    tile: &Tile,
) -> IVec2 {
    let mut draw_position = position.tile_box();
    let height = i32::from(world_tile.height);
    if tile.ignores_world_height {
        draw_position.y -= ((height / 80 - 1) * 80).max(0);
    } else {
        draw_position.y -= height;
    }
    draw_position
}

/// Top edge of the map in screen coordinates (`gsTopY` after `InitRenderParams`)
pub(crate) fn map_top_y(restricted_scroll: bool) -> i32 {
    const CELL_SIZE: i32 = 10;
    const ROOF_LEVEL_HEIGHT: i32 = 50;
    let rows = i32::from(JA2_MAP_SIZE);
    let top_left = if restricted_scroll {
        // Basement level 1
        (CELL_SIZE * rows * 3 / 10, CELL_SIZE * rows / 2)
    } else {
        (CELL_SIZE, CELL_SIZE * rows / 2)
    };
    // FromCellToScreenCoordinates
    top_left.0 + top_left.1 + ROOF_LEVEL_HEIGHT
}

/// The screen y of a tile that the z values are based on (`GetMapXYWorldY`)
pub(crate) fn map_world_y(x: i32, y: i32, top_y: i32) -> i32 {
    const CELL_SIZE: i32 = 10;
    const CENTER_WORLD: i32 = 800;
    const SCREEN_CENTER_Y: i32 = 1625;
    let world_y = (x * CELL_SIZE - CENTER_WORLD) + (y * CELL_SIZE - CENTER_WORLD);
    // The game computes this in INT16
    (world_y + SCREEN_CENTER_Y - top_y) as i16 as i32
}

/// The z value the game passes to the blitters, an INT16 converted to UINT16
fn z_value(level: i32) -> u16 {
    level as i16 as u16
}

impl<S: Fn(SpriteKey) -> Option<SpriteGeometry>> TileContext<'_, S> {
    /// Whether the nodes of a layer can be drawn, `item` for the item nodes of the structs and
    /// the tiles on the roofs, which only depend on the items and, on the roofs, the floor
    ///
    /// The roofs, what is on them and the topmost tiles are only drawn on the roof floor. Which
    /// tiles of the other layers are drawn depends on their view layer.
    fn layer_visible(&self, layer: RenderLayer, item: bool) -> bool {
        let layers = &self.options.layers;
        let roof = self.options.floor == Level::Roof;
        if item {
            return layers.items
                && match layer {
                    RenderLayer::StaticStructs | RenderLayer::DynamicStructs => true,
                    RenderLayer::StaticOnRoof | RenderLayer::DynamicOnRoof => roof,
                    _ => false,
                };
        }
        match layer {
            RenderLayer::StaticRoof
            | RenderLayer::DynamicRoof
            | RenderLayer::StaticOnRoof
            | RenderLayer::DynamicOnRoof
            | RenderLayer::StaticTopmost
            | RenderLayer::DynamicTopmost => roof,
            RenderLayer::DynamicMercs | RenderLayer::DynamicHighMercs => layers.soldiers,
            _ => true,
        }
    }

    fn render_tile(
        &self,
        kind: PassKind,
        layer: RenderLayer,
        position: GridPosition,
        out: &mut Vec<Draw>,
    ) {
        // Item nodes can be shown in a hidden layer
        if !self.layer_visible(layer, false) && !self.layer_visible(layer, true) {
            return;
        }
        let world = self.world;
        let tile = world.tile(position);
        match layer {
            RenderLayer::DynamicMercs | RenderLayer::DynamicHighMercs => {
                self.render_mercs(layer, position, out);
            }
            // The land list is drawn from its start towards the head
            RenderLayer::StaticLand => {
                if let Some(start) = tile.land_start {
                    for node in tile.land[..=start].iter().rev() {
                        self.render_node(kind, layer, position, tile, node, out);
                    }
                }
            }
            _ => {
                let nodes = match layer {
                    RenderLayer::StaticObjects | RenderLayer::DynamicObjects => &tile.objects,
                    RenderLayer::StaticShadows | RenderLayer::DynamicShadows => &tile.shadows,
                    RenderLayer::StaticStructs | RenderLayer::DynamicStructs => &tile.structs,
                    RenderLayer::StaticRoof | RenderLayer::DynamicRoof => &tile.roofs,
                    RenderLayer::StaticOnRoof | RenderLayer::DynamicOnRoof => &tile.on_roofs,
                    _ => &tile.topmost,
                };
                for node in nodes {
                    self.render_node(kind, layer, position, tile, node, out);
                }
            }
        }
    }

    fn render_mercs(&self, layer: RenderLayer, position: GridPosition, out: &mut Vec<Draw>) {
        let grid_no = GridNo::from(position);
        for merc in &self.mercs {
            // Soldiers on roofs are drawn with the high mercs, after the roofs
            if merc.on_roof != (layer == RenderLayer::DynamicHighMercs) {
                continue;
            }
            let Some(geometry) = (self.sprites)(merc.sprite) else {
                continue;
            };
            out.push(Draw {
                position: merc.position,
                sprite: DrawSprite::Fixed(merc.sprite),
                palette: merc.palette,
                shade: merc.shade,
                blitter: merc.blitter,
                z: merc.z,
                z_strip: None,
                outline: [0; 3],
                bounds: sprite_bounds(merc.position, geometry),
                grid_no,
            });
        }
    }

    /// Port of the per-node part of `RenderTiles` for tiles
    fn render_node(
        &self,
        kind: PassKind,
        layer: RenderLayer,
        position: GridPosition,
        world_tile: &WorldTile,
        node: &LevelNode,
        out: &mut Vec<Draw>,
    ) {
        let tileset = self.tileset;
        let dynamic = layer.is_dynamic();
        let is_item = node.flags.contains(NodeFlags::ITEM);
        if !self.layer_visible(layer, is_item) {
            return;
        }
        // Tiles are hidden with the editing mode that places them
        if !is_item && !self.options.layers.shows(node.tile.view_layer) {
            return;
        }
        // Items are dynamic (glowing items) and show through structures
        // (`HandleItemObscuredFlag`)
        let node_dynamic = is_item;
        let show_through = is_item;

        let mut tile = &node.tile;
        // The animation of an animated tile, its current frame is drawn
        let mut animation = None;
        if dynamic {
            if !node_dynamic && !tile.dynamic {
                let Some(frame) = tileset.current_frame(tile.id) else {
                    return;
                };
                let Some(frame_tile) = tileset.tile(frame) else {
                    return;
                };
                animation = node.tile.animation.as_ref();
                tile = frame_tile;
            }
        } else if tile.animation.is_some()
            || ((tile.dynamic || node_dynamic) && kind != PassKind::Obscured)
        {
            return;
        }

        // Only the struct and on roof layers look at the obscured flag
        let mut obscured_blitter = false;
        if matches!(
            layer,
            RenderLayer::StaticStructs | RenderLayer::StaticOnRoof
        ) {
            match (kind, show_through) {
                (PassKind::Obscured, true) => obscured_blitter = true,
                (PassKind::Obscured, false) | (PassKind::Normal, true) => return,
                (PassKind::Normal, false) => {}
            }
        }

        let (x, y) = (i32::from(position.x), i32::from(position.y));
        let world_y = map_world_y(x, y, self.top_y);
        let mut draw_position = ground_draw_position(position, world_tile, tile);
        if is_item && node.item_height > 0 {
            draw_position.y -= i32::from(node.item_height);
        }

        let shadow_blitter = matches!(
            layer,
            RenderLayer::StaticShadows | RenderLayer::DynamicShadows
        );
        // `fZWrite` and `fZBlitter` of the layer (`RenderFX`)
        let z_write = !dynamic && layer != RenderLayer::StaticLand;
        let z_blitter = !matches!(layer, RenderLayer::StaticLand | RenderLayer::DynamicShadows);
        let mut multi_z = false;
        let mut wall = false;
        let level = match layer {
            RenderLayer::StaticLand => LAND_Z_LEVEL,
            RenderLayer::StaticObjects | RenderLayer::DynamicObjects => {
                if tile.kind == (TileKind::Cliff { hang: true }) {
                    LAND_Z_LEVEL
                } else if tile.uses_object_z_height {
                    world_y * Z_SUBLAYERS + LAND_Z_LEVEL
                } else {
                    OBJECT_Z_LEVEL
                }
            }
            RenderLayer::StaticShadows | RenderLayer::DynamicShadows => {
                ((world_y - 80) * Z_SUBLAYERS + SHADOW_Z_LEVEL).max(0)
            }
            RenderLayer::StaticStructs | RenderLayer::DynamicStructs => {
                if layer == RenderLayer::StaticStructs {
                    multi_z = tile.z_strip.is_some();
                    wall = matches!(tile.kind, TileKind::Wall(_));
                }
                let level = if is_item {
                    if node.item_height > 0 {
                        STRUCT_Z_LEVEL + i32::from(node.item_height)
                    } else {
                        OBJECT_Z_LEVEL
                    }
                } else {
                    STRUCT_Z_LEVEL
                };
                // `sZLevel` is an INT16
                i32::from((level + world_y * Z_SUBLAYERS) as i16)
            }
            RenderLayer::StaticRoof | RenderLayer::DynamicRoof => {
                draw_position.y -= WALL_HEIGHT;
                (WALL_HEIGHT + world_y) * Z_SUBLAYERS + ROOF_Z_LEVEL
            }
            RenderLayer::StaticOnRoof | RenderLayer::DynamicOnRoof => {
                draw_position.y -= WALL_HEIGHT;
                (world_y + WALL_HEIGHT) * Z_SUBLAYERS + ONROOF_Z_LEVEL
            }
            RenderLayer::StaticTopmost | RenderLayer::DynamicTopmost => TOPMOST_Z_LEVEL,
            RenderLayer::DynamicMercs | RenderLayer::DynamicHighMercs => unreachable!(),
        };

        let blitter = if is_item {
            if obscured_blitter {
                Blitter::OutlineZPixelateObscured
            } else {
                Blitter::OutlineZ
            }
        } else if multi_z {
            if wall {
                Blitter::TransZIncSameZBurnsThrough
            } else {
                Blitter::TransZInc
            }
        } else if shadow_blitter {
            match (z_blitter, z_write) {
                (true, true) => Blitter::ShadowZ,
                // `ShadowZNB` is only used for physics objects
                (true, false) => return,
                (false, _) => Blitter::Shadow,
            }
        } else if z_blitter {
            if z_write {
                Blitter::TransZ
            } else {
                Blitter::TransZNB
            }
        } else {
            Blitter::Transparent
        };

        let z_strip = if blitter.uses_z_strips() {
            match &tile.z_strip {
                Some(z_strip) => Some(PackedZStrip::from(z_strip)),
                // The game warns and draws nothing
                None => return,
            }
        } else {
            None
        };

        let (sprite, bounds) = match animation {
            Some(animation) => {
                let bounds = animation
                    .frames
                    .iter()
                    .filter_map(|&frame| (self.sprites)(SpriteKey::Tile(frame)))
                    .map(|geometry| sprite_bounds(draw_position, geometry))
                    .reduce(|a, b| a.union(b));
                (DrawSprite::Animated(node.tile.id), bounds)
            }
            None => {
                let key = SpriteKey::Tile(tile.id);
                let bounds =
                    (self.sprites)(key).map(|geometry| sprite_bounds(draw_position, geometry));
                (DrawSprite::Fixed(key), bounds)
            }
        };
        let Some(bounds) = bounds else {
            return;
        };
        // `RenderTiles` picks the outline colour of items by layer
        let outline = if matches!(
            layer,
            RenderLayer::StaticOnRoof | RenderLayer::DynamicOnRoof
        ) {
            ON_ROOF_ITEM_OUTLINE_COLOR
        } else {
            ITEM_OUTLINE_COLOR
        };
        out.push(Draw {
            position: draw_position,
            sprite,
            palette: PaletteKey::Tile(tile.image.surface.id),
            shade: node.light.shade,
            blitter,
            z: z_value(level),
            z_strip,
            outline,
            bounds,
            grid_no: GridNo::from(position),
        });
    }
}

/// Area of a sprite drawn at `position`
pub fn sprite_bounds(position: IVec2, geometry: SpriteGeometry) -> IRect {
    let min = position + geometry.offset;
    IRect::from_corners(min, min + geometry.size.as_ivec2())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tileset::tests::{
        test_node, test_tileset, FIRSTFULLSTRUCT, FIRSTOSTRUCT, FIRSTROOF, FIRSTSHADOW,
        FIRSTTEXTURE, GUNS,
    };

    fn tile_sprites(_: SpriteKey) -> Option<SpriteGeometry> {
        Some(SpriteGeometry {
            offset: IVec2::ZERO,
            size: UVec2::new(40, 20),
        })
    }

    fn grid_no(x: u8, y: u8) -> GridNo {
        GridNo::from(GridPosition::new(x, y))
    }

    /// The options that draw everything, on the roof floor
    fn options(shadows: bool) -> DrawListOptions {
        DrawListOptions {
            layers: LayerVisibility::default(),
            floor: Level::Roof,
            shadows,
        }
    }

    #[test]
    fn rows_interleave_the_layers_of_a_pass() {
        let tileset = test_tileset();
        let node = |tile_type, sub_index| test_node(&tileset, tile_type, sub_index);
        let mut world = MapWorld::new(false);
        let land = &mut world.tiles[grid_no(2, 2)];
        land.land = vec![node(FIRSTTEXTURE, 1)];
        land.land_start = Some(0);
        land.objects.push(node(FIRSTOSTRUCT, 3));
        world.tiles[grid_no(0, 0)].roofs.push(node(FIRSTROOF, 1));
        world.tiles[grid_no(0, 1)]
            .structs
            .push(node(FIRSTOSTRUCT, 1));
        world.tiles[grid_no(0, 1)].roofs.push(node(FIRSTROOF, 2));
        world.tiles[grid_no(1, 0)].height = 30;
        world.tiles[grid_no(1, 0)]
            .structs
            .push(node(FIRSTOSTRUCT, 2));

        let draws = build_draw_list(&world, &tileset, tile_sprites, options(true));
        let order: Vec<_> = draws
            .iter()
            .map(|draw| (draw.grid_no, draw.blitter))
            .collect();
        assert_eq!(
            order,
            vec![
                (grid_no(2, 2), Blitter::Transparent),
                (grid_no(2, 2), Blitter::TransZ),
                // Row 0: the roof of (0, 0)
                (grid_no(0, 0), Blitter::TransZ),
                // Row 1: first the structs from left to right, then the roofs
                (grid_no(0, 1), Blitter::TransZ),
                (grid_no(1, 0), Blitter::TransZ),
                (grid_no(0, 1), Blitter::TransZ),
            ]
        );

        // Roofs are drawn a wall height higher, with their own z level
        let roof = &draws[2];
        assert_eq!(roof.position, IVec2::new(0, -WALL_HEIGHT));
        assert_eq!(
            roof.z,
            z_value((WALL_HEIGHT + 25 - 860) * Z_SUBLAYERS + ROOF_Z_LEVEL)
        );
        // The terrain height lifts the struct
        let raised = &draws[4];
        assert_eq!(raised.position, IVec2::new(20, 10 - 30));
        assert_eq!(
            raised.z,
            z_value((10 + 25 - 860) * Z_SUBLAYERS + STRUCT_Z_LEVEL)
        );
        assert_eq!(raised.bounds, IRect::new(20, -20, 60, 0));
    }

    #[test]
    fn dynamic_tiles_items_and_shadows() {
        let tileset = test_tileset();
        let node = |tile_type, sub_index| test_node(&tileset, tile_type, sub_index);
        let mut world = MapWorld::new(false);
        let tile = &mut world.tiles[grid_no(3, 3)];
        // Foliage is dynamic and drawn after the static layers, without writing z
        tile.structs.push(node(FIRSTFULLSTRUCT, 2));
        let mut item = node(GUNS, 1);
        item.flags |= NodeFlags::ITEM;
        item.item_height = 10;
        tile.structs.push(item);
        world.tiles[grid_no(4, 4)]
            .shadows
            .push(node(FIRSTSHADOW, 1));

        let draws = build_draw_list(&world, &tileset, tile_sprites, options(true));
        let blitters: Vec<_> = draws.iter().map(|draw| draw.blitter).collect();
        assert_eq!(
            blitters,
            vec![
                Blitter::ShadowZ,
                // Items show through structures in the obscured pass
                Blitter::OutlineZPixelateObscured,
                Blitter::TransZNB,
                Blitter::OutlineZ,
            ]
        );
        let item_z = z_value(STRUCT_Z_LEVEL + 10 + (60 + 25 - 860) * Z_SUBLAYERS);
        assert_eq!(draws[1].z, item_z);
        assert_eq!(draws[1].position, IVec2::new(0, 60 - 10));

        // Shadows are off from dusk on, and items can be hidden
        let layers = LayerVisibility {
            items: false,
            ..default()
        };
        let draws = build_draw_list(
            &world,
            &tileset,
            tile_sprites,
            DrawListOptions {
                layers,
                floor: Level::Roof,
                shadows: false,
            },
        );
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].blitter, Blitter::TransZNB);

        // Items stay without the objects, the tree and its shadow go with them
        let layers = LayerVisibility {
            objects: false,
            ..Default::default()
        };
        let options = DrawListOptions {
            layers,
            ..options(true)
        };
        let blitters: Vec<_> = build_draw_list(&world, &tileset, tile_sprites, options)
            .iter()
            .map(|draw| draw.blitter)
            .collect();
        assert_eq!(
            blitters,
            vec![Blitter::OutlineZPixelateObscured, Blitter::OutlineZ,]
        );
    }

    #[test]
    fn buildings_can_be_hidden() {
        use crate::file_formats::ja2_tile_types::{FIRSTFLOOR, FIRSTWALL};
        let tileset = crate::tileset::tests::building_tileset();
        let mut world = MapWorld::new(false);
        let g = grid_no(3, 3);
        let wall = test_node(&tileset, FIRSTWALL, 1).tile;
        let wall = world.add_node_to_world(g, &wall, Level::Ground).unwrap();
        world.tiles[g].structs.push(wall);
        // Also the walls without structure data and the wall shadows
        world.tiles[g]
            .structs
            .push(test_node(&tileset, FIRSTWALL, 30));
        world.tiles[g]
            .shadows
            .push(test_node(&tileset, FIRSTWALL, 30));
        // An outside structure in the structs and the floors stay
        let kept = &mut world.tiles[grid_no(5, 5)];
        kept.structs.push(test_node(&tileset, FIRSTOSTRUCT, 1));
        kept.land = vec![test_node(&tileset, FIRSTFLOOR, 1)];
        kept.land_start = Some(0);
        let draws = |buildings| {
            let layers = LayerVisibility {
                buildings,
                ..Default::default()
            };
            let options = DrawListOptions {
                layers,
                floor: Level::Roof,
                shadows: true,
            };
            build_draw_list(&world, &tileset, tile_sprites, options)
                .iter()
                .map(|draw| draw.grid_no)
                .collect::<Vec<_>>()
        };
        assert_eq!(draws(true).len(), 5);
        assert_eq!(draws(false), vec![grid_no(5, 5), grid_no(5, 5)]);
    }

    #[test]
    fn terrain_can_be_hidden() {
        use crate::file_formats::ja2_tile_types::{DEBRISROCKS, FIRSTROAD};
        let tileset = test_tileset();
        let node = |tile_type, sub_index| test_node(&tileset, tile_type, sub_index);
        let mut world = MapWorld::new(false);
        let tile = &mut world.tiles[grid_no(2, 2)];
        tile.land = vec![node(FIRSTTEXTURE, 1)];
        tile.land_start = Some(0);
        tile.objects.push(node(FIRSTROAD, 1));
        world.tiles[grid_no(4, 4)]
            .objects
            .push(node(DEBRISROCKS, 1));
        let tiles = |terrain| {
            let layers = LayerVisibility {
                terrain,
                ..Default::default()
            };
            let options = DrawListOptions {
                layers,
                ..options(true)
            };
            build_draw_list(&world, &tileset, tile_sprites, options)
                .iter()
                .map(|draw| draw.grid_no)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            tiles(true),
            vec![grid_no(2, 2), grid_no(2, 2), grid_no(4, 4)]
        );
        // The debris in the objects list stays
        assert_eq!(tiles(false), vec![grid_no(4, 4)]);
    }

    #[test]
    fn the_roof_floor_shows_the_roofs() {
        let tileset = test_tileset();
        let node = |tile_type, sub_index| test_node(&tileset, tile_type, sub_index);
        let mut world = MapWorld::new(false);
        world.tiles[grid_no(2, 2)]
            .structs
            .push(node(FIRSTOSTRUCT, 1));
        world.tiles[grid_no(2, 2)].roofs.push(node(FIRSTROOF, 1));
        let mut item = node(GUNS, 1);
        item.flags |= NodeFlags::ITEM;
        world.tiles[grid_no(3, 3)].on_roofs.push(item);
        let tiles_with = |floor, layers| {
            let options = DrawListOptions {
                floor,
                layers,
                ..options(true)
            };
            build_draw_list(&world, &tileset, tile_sprites, options)
                .iter()
                .map(|draw| draw.grid_no)
                .collect::<Vec<_>>()
        };
        let tiles = |floor| tiles_with(floor, LayerVisibility::default());
        assert_eq!(tiles(Level::Ground), vec![grid_no(2, 2)]);
        let roof = tiles(Level::Roof);
        assert_eq!(roof.iter().filter(|&&g| g == grid_no(2, 2)).count(), 2);
        assert!(roof.contains(&grid_no(3, 3)));

        // The roofs are buildings
        let no_buildings = LayerVisibility {
            buildings: false,
            ..Default::default()
        };
        let roof = tiles_with(Level::Roof, no_buildings);
        assert_eq!(roof.iter().filter(|&&g| g == grid_no(2, 2)).count(), 1);
        assert!(roof.contains(&grid_no(3, 3)));
        let no_objects = LayerVisibility {
            objects: false,
            ..Default::default()
        };
        let roof = tiles_with(Level::Roof, no_objects);
        assert_eq!(roof.iter().filter(|&&g| g == grid_no(2, 2)).count(), 1);
        // Items don't depend on the objects or buildings
        assert!(roof.contains(&grid_no(3, 3)));
        let no_items = LayerVisibility {
            items: false,
            ..no_objects
        };
        assert!(!tiles_with(Level::Roof, no_items).contains(&grid_no(3, 3)));
        // But on the ground they are hidden with the roofs
        assert!(!tiles(Level::Ground).contains(&grid_no(3, 3)));
    }

    #[test]
    fn soldiers_only_depend_on_their_layer() {
        use crate::render::{
            soldiers::Soldier,
            world::{MercNode, NodeLight},
        };
        use std::sync::Arc;
        let tileset = test_tileset();
        let registries = crate::soldier::Registries::build();
        let surface = registries.surfaces.iter().next().unwrap().clone();
        let mut world = MapWorld::new(false);
        for (grid_no, level) in [(grid_no(3, 3), Level::Ground), (grid_no(5, 5), Level::Roof)] {
            world.tiles[grid_no].mercs.push(MercNode {
                soldier: Arc::new(Soldier {
                    id: 0,
                    grid_no,
                    surface: surface.clone(),
                    frame: 0,
                    level,
                    hostile: false,
                    vehicle: false,
                }),
                light: NodeLight::default(),
            });
        }
        let only_soldiers = LayerVisibility {
            terrain: false,
            objects: false,
            buildings: false,
            items: false,
            soldiers: true,
            soldier_teams: VisibleTeams::default(),
            room_numbers: false,
        };
        let tiles = |layers, floor| {
            let options = DrawListOptions {
                layers,
                floor,
                shadows: true,
            };
            build_draw_list(&world, &tileset, tile_sprites, options)
                .iter()
                .map(|draw| draw.grid_no)
                .collect::<Vec<_>>()
        };
        // Both soldiers are drawn on both floors, whatever else is hidden
        for floor in [Level::Ground, Level::Roof] {
            assert_eq!(
                tiles(only_soldiers, floor),
                vec![grid_no(3, 3), grid_no(5, 5)]
            );
            let hidden = LayerVisibility {
                soldiers: false,
                ..Default::default()
            };
            assert!(tiles(hidden, floor).is_empty());
        }
    }

    #[test]
    fn items_on_roofs_have_a_yellow_outline() {
        let tileset = test_tileset();
        let item = || {
            let mut item = test_node(&tileset, GUNS, 1);
            item.flags |= NodeFlags::ITEM;
            item
        };
        let mut world = MapWorld::new(false);
        world.tiles[grid_no(3, 3)].structs.push(item());
        world.tiles[grid_no(5, 5)].on_roofs.push(item());

        let draws = build_draw_list(&world, &tileset, tile_sprites, options(true));
        let outlines: Vec<_> = draws
            .iter()
            .map(|draw| (draw.grid_no, draw.blitter, draw.outline))
            .collect();
        assert_eq!(
            outlines,
            vec![
                (
                    grid_no(3, 3),
                    Blitter::OutlineZPixelateObscured,
                    ITEM_OUTLINE_COLOR
                ),
                (
                    grid_no(5, 5),
                    Blitter::OutlineZPixelateObscured,
                    ON_ROOF_ITEM_OUTLINE_COLOR
                ),
                (grid_no(3, 3), Blitter::OutlineZ, ITEM_OUTLINE_COLOR),
                (grid_no(5, 5), Blitter::OutlineZ, ON_ROOF_ITEM_OUTLINE_COLOR),
            ]
        );
    }

    /// A world with land everywhere near the origin and structs, shadows and roofs in between
    fn busy_world(tileset: &Tileset) -> MapWorld {
        let node = |tile_type, sub_index| test_node(tileset, tile_type, sub_index);
        let mut world = MapWorld::new(false);
        for y in 0..12u8 {
            for x in 0..12u8 {
                let tile = &mut world.tiles[grid_no(x, y)];
                tile.land = vec![node(FIRSTTEXTURE, 1 + u16::from(x % 3))];
                tile.land_start = Some(0);
                if (x + y) % 3 == 0 {
                    tile.structs.push(node(FIRSTOSTRUCT, 1 + u16::from(y % 4)));
                    tile.shadows.push(node(FIRSTSHADOW, 1));
                }
                if x % 4 == 1 {
                    tile.roofs.push(node(FIRSTROOF, 1));
                    tile.height = 10;
                }
            }
        }
        world
    }

    #[test]
    fn cached_draws_are_the_draw_list() {
        let tileset = test_tileset();
        let world = busy_world(&tileset);
        let cache = DrawCache::build(&DrawContext {
            world: &world,
            tileset: &tileset,
            sprites: tile_sprites,
            options: options(true),
        });
        let all = cache.all();
        assert_eq!(
            all,
            build_draw_list(&world, &tileset, tile_sprites, options(true))
        );
        assert_eq!(
            cache.bounds(),
            all.iter()
                .map(|d| d.bounds)
                .reduce(|a, b| a.union(b))
                .unwrap()
        );
    }

    #[test]
    fn replacing_tile_draws_reports_changes() {
        let tileset = test_tileset();
        let mut world = busy_world(&tileset);
        let mut cache = DrawCache::build(&DrawContext {
            world: &world,
            tileset: &tileset,
            sprites: tile_sprites,
            options: options(true),
        });
        let changed = grid_no(3, 3);
        let tile_draws = |world: &MapWorld| {
            DrawContext {
                world,
                tileset: &tileset,
                sprites: tile_sprites,
                options: options(true),
            }
            .tile_draws(changed)
        };
        let same = tile_draws(&world);
        assert!(!cache.replace(changed, same));

        // Raising the tile moves its draws up
        world.tiles[changed].height = 30;
        let raised = tile_draws(&world);
        assert!(cache.replace(changed, raised));
        assert_eq!(
            cache.all(),
            build_draw_list(&world, &tileset, tile_sprites, options(true))
        );

        // Removing everything
        world.tiles[changed] = WorldTile::default();
        let empty = tile_draws(&world);
        assert_eq!(empty.bounds, None);
        assert!(cache.replace(changed, empty));
        assert_eq!(
            cache.all(),
            build_draw_list(&world, &tileset, tile_sprites, options(true))
        );
    }

    #[test]
    fn top_y() {
        assert_eq!(map_top_y(false), 860);
        assert_eq!(map_top_y(true), 1330);
    }

    #[test]
    fn world_y() {
        assert_eq!(map_world_y(0, 0, 860), 25 - 860);
        assert_eq!(map_world_y(80, 80, 860), 1600 + 25 - 860);
        assert_eq!(map_world_y(159, 159, 1330), 3180 + 25 - 1330);
    }

    #[test]
    fn z_values_wrap_like_int16() {
        assert_eq!(z_value(5), 5);
        assert_eq!(z_value(-8), 65528);
        assert_eq!(z_value(TOPMOST_Z_LEVEL), 32767);
    }

    #[test]
    fn z_strip_columns() {
        // Increasing by two strips, then stable, then decreasing
        let info = ZStrip {
            initial_z_change: -2,
            first_z_strip_width: 7,
            z_changes: vec![1, 1, 0, -1],
        };
        let packed = PackedZStrip::from(&info);
        assert_eq!(packed.changes, 0b11_00_01_01);
        let z = 1000;
        assert_eq!(packed.column_z(z, 0), 1000 - 160);
        assert_eq!(packed.column_z(z, 6), 1000 - 160);
        assert_eq!(packed.column_z(z, 7), 1000 - 80);
        assert_eq!(packed.column_z(z, 26), 1000 - 80);
        assert_eq!(packed.column_z(z, 27), 1000);
        assert_eq!(packed.column_z(z, 47), 1000);
        assert_eq!(packed.column_z(z, 66), 1000);
        assert_eq!(packed.column_z(z, 67), 1000 - 80);
        // Past the last change nothing changes any more
        assert_eq!(packed.column_z(z, 500), 1000 - 80);
    }

    #[test]
    fn soldier_teams_are_read_with_the_layers() {
        let options: crate::render::RenderOptions =
            serde_json::from_str(r#"{"layers":{"soldierTeams":{"enemy":false}}}"#).unwrap();
        let teams = options.layers.soldier_teams;
        assert!(!teams.enemy && teams.creature && teams.rebel && teams.civilian);
        assert!(!teams.shows(TeamId::ENEMY));
        assert!(teams.shows(TeamId::CIV));
        // The player's soldiers are always shown
        assert!(teams.shows(TeamId::OUR));
        // The shown teams don't change the draws
        assert_eq!(options.layers.drawn(), LayerVisibility::default().drawn());
    }
}
