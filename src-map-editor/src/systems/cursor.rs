use bevy::{prelude::*, window::PrimaryWindow};

use crate::{
    components::map::{game_to_tile, raised_tile_at, world_to_game, GridPosition},
    map::{GridNo, Level},
    render::{soldiers::ROOF_HEIGHT_ADJUSTMENT, world::MapWorld, RenderOptions},
    states::AppState,
    systems::{pixel_view::PixelView, MapSystems},
};

/// Information about what is below the mouse cursor
#[derive(Resource, Debug, Clone, Default)]
pub struct CursorInfo {
    pub world_position: Option<Vec2>,
    /// Game pixel position
    pub game_position: Option<Vec2>,
    /// Fractional tile coordinates, ignoring the terrain height
    pub tile_position: Option<Vec2>,
    /// The tile below the cursor on the floor of the render options, raised by its terrain
    /// height like it is drawn and on the roof floor by the height of the roofs, `None` outside
    /// of the map
    pub tile: Option<GridPosition>,
}

pub struct CursorPlugin;

impl Plugin for CursorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CursorInfo>().add_systems(
            Update,
            update_cursor_info
                .in_set(MapSystems::Cursor)
                .run_if(in_state(AppState::Map)),
        );
    }
}

/// The tile at a game pixel position on a floor: the one whose diamond, raised by its terrain
/// height and on the roof by the height of the roofs, is there
pub fn tile_on_floor(
    game: Vec2,
    floor: Level,
    height: impl Fn(GridPosition) -> i32,
) -> Option<GridPosition> {
    let raise = match floor {
        Level::Ground => 0,
        Level::Roof => ROOF_HEIGHT_ADJUSTMENT,
    };
    raised_tile_at(game + Vec2::new(0., raise as f32), height)
}

fn update_cursor_info(
    window: Single<&Window, With<PrimaryWindow>>,
    pixel_view: Res<PixelView>,
    world: Option<Res<MapWorld>>,
    options: Option<Res<RenderOptions>>,
    mut cursor_info: ResMut<CursorInfo>,
) {
    let world_position = window
        .physical_cursor_position()
        .map(|position| pixel_view.screen_to_world(window.physical_size().as_vec2(), position));

    let Some(world_position) = world_position else {
        *cursor_info = CursorInfo::default();
        return;
    };

    let game_position = world_to_game(world_position);
    let tile_position = game_to_tile(game_position);
    let floor = options.map_or(Level::Ground, |options| options.floor);
    let tile = tile_on_floor(game_position, floor, |position| {
        world.as_ref().map_or(0, |world| {
            i32::from(world.tiles[GridNo::from(position)].height)
        })
    });
    *cursor_info = CursorInfo {
        world_position: Some(world_position),
        game_position: Some(game_position),
        tile_position: Some(tile_position),
        tile,
    };
}

/// Draws a diamond inside its bounding box, like an isometric tile
pub fn draw_diamond(gizmos: &mut Gizmos, center: Vec2, size: Vec2, color: Srgba) {
    let half = size / 2.;
    gizmos.rect_2d(
        Isometry2d::from_translation(center),
        size,
        color.with_alpha(32. / 255.),
    );
    gizmos.lineloop_2d(
        [
            center + Vec2::new(-half.x, 0.),
            center + Vec2::new(0., half.y),
            center + Vec2::new(half.x, 0.),
            center + Vec2::new(0., -half.y),
        ],
        color.with_alpha(0.5),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::map::{JA2_TILE_HEIGHT, JA2_TILE_WIDTH};

    #[test]
    fn the_roof_floor_picks_the_roof_tiles() {
        let tile = GridPosition::new(10, 10);
        let center = (tile.tile_box() + IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT) / 2).as_vec2();
        let flat = |_| 0;
        assert_eq!(tile_on_floor(center, Level::Ground, flat), Some(tile));
        // The roof of the tile is drawn higher, where a tile further up is on the ground
        let roof = center - Vec2::new(0., ROOF_HEIGHT_ADJUSTMENT as f32);
        assert_eq!(tile_on_floor(roof, Level::Roof, flat), Some(tile));
        assert_ne!(tile_on_floor(roof, Level::Ground, flat), Some(tile));
        // Raised by the terrain height too
        let hill = |position| if position == tile { 20 } else { 0 };
        let raised = roof - Vec2::new(0., 20.);
        assert_eq!(tile_on_floor(raised, Level::Roof, hill), Some(tile));
    }
}
