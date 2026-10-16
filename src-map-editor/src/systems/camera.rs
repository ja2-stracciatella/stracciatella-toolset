use bevy::{
    input::mouse::{AccumulatedMouseScroll, MouseScrollPixelsPerLine, MouseScrollUnit},
    prelude::*,
    window::PrimaryWindow,
};

use crate::{
    components::map::game_rect_to_world,
    render::{movement_costs::scrollable_area, world::MapWorld},
    states::AppState,
    systems::{
        pixel_view::{zoom_around, PixelView, MAX_ZOOM, MIN_ZOOM},
        MapSystems,
    },
};

const CAMERA_MOUSE_SCROLL_BORDER: f32 = 60.;
/// Seconds the cursor has to stay in the border before the camera scrolls, so that moving the
/// mouse quickly over the edge does not scroll
const CAMERA_MOUSE_SCROLL_DELAY: f32 = 0.3;
const CAMERA_FRICTION_ACCEL: f32 = 10.;
const CAMERA_FRICTION_DECEL: f32 = 5.;
const MAX_CAMERA_SPEED: f32 = 25.;

/// Zoom factor per mouse wheel line
const ZOOM_STEP: f32 = 1.1;
/// Mouse wheel lines per second while a zoom key is held
const KEY_ZOOM_LINES_PER_SECOND: f32 = 10.;
/// How quickly the zoom eases towards the target zoom, per second
const ZOOM_EASING_RATE: f32 = 15.;
/// Zoom differences below this are snapped to the target zoom
const ZOOM_SNAP_EPSILON: f32 = 1e-3;
/// How far beyond the part of the map that the game can show the camera can scroll, in world
/// pixels, so that the map can be designed slightly outside of it
const CAMERA_SCROLL_MARGIN: f32 = 160.;

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (move_camera, zoom_camera, clamp_camera)
                .chain()
                .in_set(MapSystems::Camera)
                .run_if(in_state(AppState::Map)),
        );
    }
}

fn axis(keys: &ButtonInput<KeyCode>, pos: KeyCode, neg: KeyCode) -> f32 {
    let mut value = 0.;
    if keys.pressed(pos) {
        value += 1.;
    }
    if keys.pressed(neg) {
        value -= 1.;
    }
    value
}

fn move_camera(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut pixel_view: ResMut<PixelView>,
    mut camera_speed: Local<Vec2>,
    mut border_time: Local<f32>,
) {
    // Shortcuts like Ctrl+S or Cmd+S are meant for the host
    let shortcut = keys.any_pressed([
        KeyCode::ControlLeft,
        KeyCode::ControlRight,
        KeyCode::SuperLeft,
        KeyCode::SuperRight,
    ]);
    let axis_scroll_speed = if shortcut {
        Vec2::ZERO
    } else {
        Vec2::new(
            axis(&keys, KeyCode::KeyD, KeyCode::KeyA),
            axis(&keys, KeyCode::KeyW, KeyCode::KeyS),
        )
        .normalize_or_zero()
    };

    let mouse_scroll_speed = {
        let mut v = Vec2::ZERO;
        if let Some(mouse_position) = window.cursor_position() {
            if mouse_position.x < CAMERA_MOUSE_SCROLL_BORDER {
                v.x -= 1.;
            }
            if mouse_position.x > window.width() - CAMERA_MOUSE_SCROLL_BORDER {
                v.x += 1.;
            }
            if mouse_position.y < CAMERA_MOUSE_SCROLL_BORDER {
                v.y += 1.;
            }
            if mouse_position.y > window.height() - CAMERA_MOUSE_SCROLL_BORDER {
                v.y -= 1.;
            }
        }
        if v == Vec2::ZERO {
            *border_time = 0.;
        } else {
            *border_time += time.delta_secs();
        }
        if *border_time >= CAMERA_MOUSE_SCROLL_DELAY {
            v.normalize_or_zero()
        } else {
            Vec2::ZERO
        }
    };

    let wanted_speed = if axis_scroll_speed != Vec2::ZERO {
        axis_scroll_speed * MAX_CAMERA_SPEED
    } else {
        mouse_scroll_speed * MAX_CAMERA_SPEED
    };

    let friction = if wanted_speed == Vec2::ZERO {
        CAMERA_FRICTION_DECEL
    } else {
        CAMERA_FRICTION_ACCEL
    };
    let speed = *camera_speed;
    *camera_speed = speed + (wanted_speed - speed) / friction;

    if *camera_speed != Vec2::ZERO {
        // The speed is in logical screen pixels, so that it feels the same at every zoom level
        let world_speed = *camera_speed * window.scale_factor() / pixel_view.zoom;
        pixel_view.position += world_speed;
    }
}

fn zoom_camera(
    keys: Res<ButtonInput<KeyCode>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    pixels_per_line: Res<MouseScrollPixelsPerLine>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut pixel_view: ResMut<PixelView>,
) {
    let dt = time.delta_secs();
    let mut lines = match mouse_scroll.unit {
        MouseScrollUnit::Line => mouse_scroll.delta.y,
        // Pixel deltas are physical pixels, so wheel zoom would be faster on HiDPI screens
        MouseScrollUnit::Pixel => mouse_scroll.delta.y / window.scale_factor() / *pixels_per_line,
    };
    lines += KEY_ZOOM_LINES_PER_SECOND
        * dt
        * (axis(&keys, KeyCode::Equal, KeyCode::Minus)
            + axis(&keys, KeyCode::NumpadAdd, KeyCode::NumpadSubtract));
    if lines != 0. {
        pixel_view.target_zoom =
            (pixel_view.target_zoom * ZOOM_STEP.powf(lines)).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    let old_zoom = pixel_view.zoom;
    let target_zoom = pixel_view.target_zoom;
    if old_zoom == target_zoom {
        return;
    }
    // Ease exponentially in log space, so zooming in and out feels the same
    let t = 1. - (-ZOOM_EASING_RATE * dt).exp();
    let new_zoom = if (target_zoom / old_zoom).ln().abs() < ZOOM_SNAP_EPSILON {
        target_zoom
    } else {
        (old_zoom.ln() + (target_zoom.ln() - old_zoom.ln()) * t).exp()
    };

    let window_size = window.physical_size().as_vec2();
    // Zoom around the cursor, or around the center if it is outside of the window
    let anchor = window
        .physical_cursor_position()
        .unwrap_or(window_size / 2.);
    pixel_view.position = zoom_around(pixel_view.position, old_zoom, new_zoom, window_size, anchor);
    pixel_view.zoom = new_zoom;
}

/// Keeps the view inside of the part of the map that the game can show plus a margin
fn clamp_camera(
    world: Option<Res<MapWorld>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut pixel_view: ResMut<PixelView>,
) {
    let Some(world) = world else {
        return;
    };
    let (min, max) = scrollable_area(world.restricted_scroll);
    let bounds = game_rect_to_world(min, max).inflate(CAMERA_SCROLL_MARGIN);
    let half_extent = window.physical_size().as_vec2() / pixel_view.zoom / 2.;
    let position = clamp_view(pixel_view.position, half_extent, bounds);
    if position != pixel_view.position {
        pixel_view.position = position;
    }
}

/// The position closest to `position` at which a view of `half_extent` stays inside of `bounds`,
/// centered on an axis where the view is larger than the bounds
fn clamp_view(position: Vec2, half_extent: Vec2, bounds: Rect) -> Vec2 {
    let axis = |position: f32, half_extent: f32, min: f32, max: f32| {
        if max - min <= 2. * half_extent {
            (min + max) / 2.
        } else {
            position.clamp(min + half_extent, max - half_extent)
        }
    };
    Vec2::new(
        axis(position.x, half_extent.x, bounds.min.x, bounds.max.x),
        axis(position.y, half_extent.y, bounds.min.y, bounds.max.y),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn views_stay_inside_of_the_bounds() {
        let bounds = Rect::new(-100., -50., 100., 50.);
        let half_extent = Vec2::new(20., 10.);
        // Inside
        assert_eq!(
            clamp_view(Vec2::new(10., 5.), half_extent, bounds),
            Vec2::new(10., 5.)
        );
        // Beyond the edges
        assert_eq!(
            clamp_view(Vec2::new(-500., 45.), half_extent, bounds),
            Vec2::new(-80., 40.)
        );
        // Larger than the bounds on one axis
        assert_eq!(
            clamp_view(Vec2::new(90., 30.), Vec2::new(150., 10.), bounds),
            Vec2::new(0., 30.)
        );
    }
}
