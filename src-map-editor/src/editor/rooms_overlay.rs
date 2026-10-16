//! The room numbers of the tiles (`RenderRoomInfo`), shown with the `roomNumbers` layer
//!
//! The numbers are text on the screen camera, so they stay sharp at every zoom. Only the tiles
//! in view get a label, and the labels follow the view.

use std::collections::HashMap;

use bevy::{camera::visibility::RenderLayers, prelude::*, window::PrimaryWindow};

use crate::{
    components::map::{game_to_world, JA2_TILE_HEIGHT, JA2_TILE_WIDTH},
    map::{GridNo, RoomNo, TacticalMap},
    render::{world::MapWorld, RenderOptions},
    states::{AppState, UiFont},
    systems::pixel_view::{PixelView, SCREEN_LAYER},
};

/// Size of the numbers in logical pixels
const FONT_SIZE: f32 = 11.;
/// Labels of tiles this far outside of the view are kept, in world pixels
const VIEW_MARGIN: f32 = 40.;

/// The labels that are shown, by their tiles
#[derive(Resource, Default)]
pub struct RoomLabels {
    shown: HashMap<GridNo, (RoomNo, Entity)>,
}

/// The text of a room number
#[derive(Component)]
pub struct RoomLabel {
    /// Where the number is drawn, in world coordinates
    position: Vec2,
}

/// The colour of a room number, like the original picks it by the number modulo 5
pub fn room_color(room: RoomNo) -> Color {
    match room.number() % 5 {
        0 => Color::srgb(0.63, 0.63, 0.63),
        1 => Color::srgb(1., 1., 0.),
        2 => Color::srgb(1., 0.4, 0.4),
        3 => Color::srgb(0.45, 0.65, 1.),
        _ => Color::srgb(0.45, 1., 0.45),
    }
}

/// The labels of a new map are not shown yet
pub fn reset_room_labels(mut labels: ResMut<RoomLabels>) {
    *labels = RoomLabels::default();
}

/// The center of a tile in world coordinates, raised by its height
fn tile_center(world: &MapWorld, grid_no: GridNo) -> Vec2 {
    let mut game = grid_no.position().tile_box() + IVec2::new(JA2_TILE_WIDTH, JA2_TILE_HEIGHT) / 2;
    game.y -= i32::from(world.tiles[grid_no].height);
    game_to_world(game)
}

/// Spawns and removes the labels of the tiles in view, and moves them with the view
pub fn update_room_labels(
    mut commands: Commands,
    options: Res<RenderOptions>,
    map: Res<TacticalMap>,
    world: Res<MapWorld>,
    view: Res<PixelView>,
    window: Single<&Window, With<PrimaryWindow>>,
    font: Res<UiFont>,
    mut labels: ResMut<RoomLabels>,
    mut transforms: Query<(&RoomLabel, &mut Transform)>,
) {
    if !options.layers.room_numbers {
        for (_, (_, entity)) in labels.shown.drain() {
            commands.entity(entity).despawn();
        }
        return;
    }
    let size = window.physical_size().as_vec2();
    if map.is_changed() || view.is_changed() || options.is_changed() {
        let corners = [Vec2::ZERO, size].map(|corner| view.screen_to_world(size, corner));
        let area = Rect::from_corners(corners[0], corners[1]).inflate(VIEW_MARGIN);
        let wanted: HashMap<GridNo, RoomNo> = map
            .tiles
            .iter()
            .filter_map(|(grid_no, tile)| Some((grid_no, tile.room?)))
            .filter(|&(grid_no, _)| area.contains(tile_center(&world, grid_no)))
            .collect();
        labels.shown.retain(|grid_no, (room, entity)| {
            let keep = wanted.get(grid_no) == Some(room);
            if !keep {
                commands.entity(*entity).despawn();
            }
            keep
        });
        let text_font = font.text_font(FONT_SIZE * window.scale_factor());
        for (grid_no, room) in wanted {
            if labels.shown.contains_key(&grid_no) {
                continue;
            }
            let position = tile_center(&world, grid_no);
            let entity = commands
                .spawn((
                    RoomLabel { position },
                    Text2d::new(room.number().to_string()),
                    text_font.clone(),
                    TextColor(room_color(room)),
                    RenderLayers::layer(SCREEN_LAYER),
                    Transform::from_translation(view.world_to_screen(position).extend(10.)),
                    DespawnOnExit(AppState::Map),
                ))
                .id();
            labels.shown.insert(grid_no, (room, entity));
        }
    }
    // 1 unit of the screen camera is 1 physical pixel, above the map
    for (label, mut transform) in &mut transforms {
        let translation = view.world_to_screen(label.position).extend(10.);
        if transform.translation != translation {
            transform.translation = translation;
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy::ecs::system::RunSystemOnce;

    use super::*;
    use crate::{editor::document::tests::grid, tileset::TilesetId};

    /// New labels must not show up at the center of the screen for a frame while scrolling
    #[test]
    fn labels_are_spawned_at_their_tiles() {
        let grid_no = grid(40, 40);
        let mut map = TacticalMap::empty(TilesetId::new(0));
        map.tiles[grid_no].room = RoomNo::new(3);
        let world = MapWorld::new(false);
        let position = tile_center(&world, grid_no);
        let view = PixelView::new(position + Vec2::new(20., 10.), 2.);
        let mut options = RenderOptions::default();
        options.layers.room_numbers = true;

        let mut app = World::new();
        app.init_resource::<Assets<Font>>();
        app.init_resource::<UiFont>();
        app.init_resource::<RoomLabels>();
        app.insert_resource(map);
        app.insert_resource(world);
        app.insert_resource(view);
        app.insert_resource(options);
        app.spawn((Window::default(), PrimaryWindow));
        app.run_system_once(update_room_labels).unwrap();

        let transforms: Vec<Vec3> = app
            .query_filtered::<&Transform, With<RoomLabel>>()
            .iter(&app)
            .map(|transform| transform.translation)
            .collect();
        assert_eq!(transforms, [Vec3::new(-40., -20., 10.)]);
    }
}
