use bevy::prelude::*;

pub mod camera;
pub mod cursor;
pub mod pixel_view;
pub mod radar;
pub mod view_replay;

/// Ordering of the systems that run every frame while a map is shown
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum MapSystems {
    Scene,
    Input,
    Camera,
    View,
    Cursor,
    Edit,
    Debug,
}

pub fn configure_map_systems(app: &mut App) {
    app.configure_sets(
        Update,
        (
            MapSystems::Scene,
            MapSystems::Input,
            MapSystems::Camera,
            MapSystems::View,
            MapSystems::Cursor,
            MapSystems::Edit,
            MapSystems::Debug,
        )
            .chain(),
    );
}
