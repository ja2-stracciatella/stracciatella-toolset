use bevy::prelude::*;

/// Marks the camera that draws the rendered map to the window
#[derive(Component)]
pub struct ScreenCamera;

/// Marks the camera that draws the gizmos and the UI to the window, on top of the map
#[derive(Component)]
pub struct OverlayCamera;
