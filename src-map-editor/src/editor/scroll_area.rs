//! The part of the map that the game can scroll over (`ubRestrictedScrollID`): it is drawn as a
//! white box, and the world is synced with it when it changes

use bevy::prelude::*;

use crate::{
    components::map::game_rect_to_world,
    map::TacticalMap,
    render::{
        movement_costs::{scrollable_area, MovementCosts},
        world::MapWorld,
        PendingSceneUpdate,
    },
};

use super::document::EditorDocument;

/// The world rectangle that the game can show of a map
pub fn scrollable_world_rect(restricted_scroll: bool) -> Rect {
    let (min, max) = scrollable_area(restricted_scroll);
    game_rect_to_world(min, max)
}

/// Rebuilds the scene after the scroll restriction changed: the visible tiles, the movement
/// costs and the z values depend on it
pub fn sync_restricted_scroll(
    mut document: ResMut<EditorDocument>,
    map: Res<TacticalMap>,
    mut world: ResMut<MapWorld>,
    mut costs: ResMut<MovementCosts>,
    mut pending: ResMut<PendingSceneUpdate>,
) {
    if !document.restricted_scroll_dirty {
        return;
    }
    document.restricted_scroll_dirty = false;
    if world.restricted_scroll == map.info.restricted_scroll {
        return;
    }
    world.restricted_scroll = map.info.restricted_scroll;
    *costs = MovementCosts::compile(&mut world, &map);
    pending.full = true;
}

/// Draws the part of the map that the game can show as a white box
pub fn draw_scrollable_area(mut gizmos: Gizmos, world: Res<MapWorld>) {
    let rect = scrollable_world_rect(world.restricted_scroll);
    gizmos.rect_2d(
        Isometry2d::from_translation(rect.center()),
        rect.size(),
        Color::WHITE,
    );
}
