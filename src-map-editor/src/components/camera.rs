use bevy::prelude::*;

/// Marks the camera that is used to look at the map
#[derive(Component)]
pub struct TileCamera;

/// Marks the camera that draws the rendered map and the UI to the window
#[derive(Component)]
pub struct ScreenCamera;
