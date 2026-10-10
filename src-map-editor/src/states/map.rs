use bevy::prelude::*;

use crate::{
    states::{AppState, DebugOverlay},
    systems::MapSystems,
};

pub struct MapPlugin;

impl Plugin for MapPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            toggle_tile_debug
                .in_set(MapSystems::Input)
                .run_if(in_state(AppState::Map)),
        );
    }
}

fn toggle_tile_debug(
    keys: Res<ButtonInput<KeyCode>>,
    debug_overlay: Res<State<DebugOverlay>>,
    mut next_debug_overlay: ResMut<NextState<DebugOverlay>>,
) {
    if keys.just_pressed(KeyCode::F3) {
        next_debug_overlay.set(match debug_overlay.get() {
            DebugOverlay::Off => DebugOverlay::On,
            DebugOverlay::On => DebugOverlay::Off,
        });
    }
}
