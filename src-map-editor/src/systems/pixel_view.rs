//! Renders the world pixel-perfect into an off-screen texture and draws it to the screen
//!
//! The world camera renders at exactly 1 world px = 1 texel, without MSAA and from an integer
//! position, so every fragment of the map is at the center of a world pixel and the replay shader
//! (see `view_replay.rs`) computes exactly one game pixel for it. The screen camera then draws
//! that texture as a single, bilinear filtered sprite scaled by the zoom and shifted by the
//! sub-pixel part of the camera position. This keeps scrolling and zooming smooth.
//!
//! The texture is sized for the window at [`MIN_ZOOM`], and the world camera only renders into a
//! centered viewport that fits the current zoom. The texture therefore only changes size when the
//! window does. Replaying the map is expensive, so the world camera only renders when the view or
//! the map changed ([`MapRedraw`]).
//!
//! The gizmos and the previews of the editor change with the mouse, so they are not drawn into the
//! world texture but by the overlay camera ([`OVERLAY_LAYER`]), which looks at the world directly
//! on the screen.
//!
//! A texture of a new size is a new image. Bevy picks up a new render target one frame late, so
//! the screen keeps showing the old texture until the world camera has rendered into the new one.

use bevy::{
    camera::{visibility::RenderLayers, RenderTarget, Viewport},
    image::ImageSampler,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    ui::IsDefaultUiCamera,
    window::PrimaryWindow,
};

use crate::{
    components::camera::{OverlayCamera, ScreenCamera, TileCamera},
    systems::MapSystems,
    CLEAR_COLOR,
};

/// Render layer of the screen camera and the sprite showing the world texture
pub const SCREEN_LAYER: usize = 1;
/// Render layer of the overlay camera, the gizmos and the previews of the editor
pub const OVERLAY_LAYER: usize = 2;
/// Width of the gizmo lines in physical pixels at any zoom
///
/// A whole number: the view moves by fractions of a pixel, and a horizontal or vertical line of a
/// fractional width would cover one pixel more or less depending on where it lands.
const GIZMO_LINE_WIDTH: f32 = 2.;
/// The world texture grows and shrinks in steps of this many texels (must be even), so that it
/// isn't recreated on every frame while the window is resized
const TEXTURE_SIZE_STEP: u32 = 128;
/// How many frames the world camera renders after the map changed, more than one in case a
/// pipeline only becomes ready a frame later
const MAP_REDRAW_FRAMES: u32 = 2;

/// Current view onto the world
#[derive(Resource, Debug, Clone)]
pub struct PixelView {
    /// Texture shown on the screen, the world camera renders into it (or into its successor of
    /// a new size, see the module documentation)
    pub image: Handle<Image>,
    /// World position at the center of the screen
    pub position: Vec2,
    /// Physical screen pixels per world pixel
    pub zoom: f32,
    /// Zoom that [`PixelView::zoom`] eases towards
    pub target_zoom: f32,
    /// Whether the zoom has been initialized from the window scale factor
    zoom_initialized: bool,
}

impl PixelView {
    /// World position of the physical screen position `screen` (y down, like the cursor)
    pub fn screen_to_world(&self, window_physical_size: Vec2, screen: Vec2) -> Vec2 {
        screen_to_world(self.position, self.zoom, window_physical_size, screen)
    }

    /// Position on the screen camera (1 unit per physical pixel, origin at the center, y up) of
    /// the world position `world`
    pub fn world_to_screen(&self, world: Vec2) -> Vec2 {
        (world - self.position) * self.zoom
    }

    /// A view without a texture
    #[cfg(test)]
    pub fn new(position: Vec2, zoom: f32) -> Self {
        Self {
            image: Handle::default(),
            position,
            zoom,
            target_zoom: zoom,
            zoom_initialized: true,
        }
    }
}

/// When the world camera renders
///
/// It renders when its position, viewport or texture changed, or when the renderer asks for it
/// with `changed`. Otherwise the texture keeps showing the last frame.
#[derive(Resource, Debug, Clone, Default)]
pub struct MapRedraw {
    /// The map looks different, set by the renderer and reset once the camera renders
    pub changed: bool,
    /// What the camera rendered last
    last_view: Option<RenderedView>,
    frames_left: u32,
}

impl MapRedraw {
    /// Whether the world texture shows the current view and map
    #[cfg(feature = "bench")]
    pub fn is_idle(&self) -> bool {
        !self.changed && self.frames_left == 0
    }
}

/// What the world camera renders: its position, viewport and texture
#[derive(Debug, Clone, PartialEq)]
struct RenderedView {
    position: Vec2,
    viewport: Option<(UVec2, UVec2)>,
    target: Option<AssetId<Image>>,
}

/// Sprite showing the world texture on the screen
#[derive(Component)]
struct PixelCanvas;

pub struct PixelViewPlugin;

impl Plugin for PixelViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapRedraw>()
            .add_systems(Startup, setup)
            // Keep the view fitted to the window in every state
            .add_systems(
                Update,
                (fit_pixel_view, fit_overlay)
                    .chain()
                    .in_set(MapSystems::View),
            )
            // After the renderer updated its material
            .add_systems(PostUpdate, redraw_on_change);
    }
}

/// A world texture of `size` texels
fn world_texture(size: UVec2) -> Image {
    let mut image = Image::new_target_texture(size.x, size.y, TextureFormat::Rgba8UnormSrgb, None);
    image.sampler = ImageSampler::linear();
    // The benchmark reads it back
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    // The world camera clears the whole texture whenever it renders, so there is no need to
    // allocate and upload the texels. A texture of a new size is a new image, there is nothing to
    // copy over.
    image.data = None;
    image.copy_on_resize = false;
    image
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut gizmos: ResMut<GizmoConfigStore>,
) {
    let size = fit_texture_size(UVec2::ZERO, required_texture_size(UVec2::ZERO, MIN_ZOOM));
    let image = images.add(world_texture(size));

    commands.insert_resource(PixelView {
        image: image.clone(),
        position: Vec2::ZERO,
        zoom: 1.,
        target_zoom: 1.,
        zoom_initialized: false,
    });

    // Renders the map on the default render layer 0
    commands.spawn((
        Camera2d,
        TileCamera,
        Camera {
            order: -1,
            clear_color: ClearColorConfig::Custom(CLEAR_COLOR),
            ..default()
        },
        RenderTarget::Image(image.clone().into()),
        Msaa::Off,
    ));
    commands.spawn((
        Camera2d,
        ScreenCamera,
        Msaa::Off,
        RenderLayers::layer(SCREEN_LAYER),
    ));
    commands.spawn((
        Sprite::from_image(image),
        PixelCanvas,
        RenderLayers::layer(SCREEN_LAYER),
    ));
    commands.spawn((
        Camera2d,
        OverlayCamera,
        Camera {
            order: 1,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        Msaa::Off,
        IsDefaultUiCamera,
        RenderLayers::layer(OVERLAY_LAYER),
    ));
    let (config, _) = gizmos.config_mut::<DefaultGizmoConfigGroup>();
    config.render_layers = RenderLayers::layer(OVERLAY_LAYER);
    config.line.width = GIZMO_LINE_WIDTH;
}

/// The image of an image render target
fn target_image(target: &RenderTarget) -> Option<&Handle<Image>> {
    match target {
        RenderTarget::Image(image) => Some(&image.handle),
        _ => None,
    }
}

fn fit_pixel_view(
    window: Single<&Window, With<PrimaryWindow>>,
    mut pixel_view: ResMut<PixelView>,
    mut images: ResMut<Assets<Image>>,
    mut world_camera: Single<
        (&mut Camera, &mut RenderTarget, &mut Transform),
        (With<TileCamera>, Without<PixelCanvas>),
    >,
    mut screen_camera: Single<&mut Projection, With<ScreenCamera>>,
    mut canvas: Single<(&mut Sprite, &mut Transform), (With<PixelCanvas>, Without<TileCamera>)>,
) {
    if !pixel_view.zoom_initialized {
        // The scale factor is only known once the window exists
        let zoom = window.scale_factor().clamp(MIN_ZOOM, MAX_ZOOM);
        pixel_view.zoom = zoom;
        pixel_view.target_zoom = zoom;
        pixel_view.zoom_initialized = true;
    }
    let zoom = pixel_view.zoom;
    let position = pixel_view.position;
    let (camera, target, camera_transform) = &mut *world_camera;

    let Some(target_handle) = target_image(target).cloned() else {
        return;
    };
    let Some(current_size) = images.get(&target_handle).map(|image| image.size()) else {
        return;
    };
    let size = fit_texture_size(
        current_size,
        required_texture_size(window.physical_size(), MIN_ZOOM),
    );
    let target_handle = if size != current_size {
        let image = images.add(world_texture(size));
        **target = RenderTarget::Image(image.clone().into());
        image
    } else {
        target_handle
    };
    // The camera reports the size of a new target once it renders into it
    if pixel_view.image != target_handle && camera.physical_target_size() == Some(size) {
        pixel_view.image = target_handle.clone();
        canvas.0.image = target_handle.clone();
    }

    // 1 unit of the screen camera = 1 physical pixel. With `ScalingMode::WindowSize` the camera
    // shows `logical size * scale` units, and the logical size is `physical size / scale factor`.
    if let Projection::Orthographic(projection) = &mut **screen_camera {
        let scale = window.scale_factor();
        if projection.scale != scale {
            projection.scale = scale;
        }
    }

    // The camera only sees the size of a new texture in the next frame, stay within the size it
    // knows so that the viewport is valid
    let known_size = camera.physical_target_size().unwrap_or(size).min(size);
    let (view_position, view_size) = view_rect(
        known_size,
        required_texture_size(window.physical_size(), zoom),
    );
    let viewport_changed = camera.viewport.as_ref().is_none_or(|viewport| {
        viewport.physical_position != view_position || viewport.physical_size != view_size
    });
    if viewport_changed {
        camera.viewport = Some(Viewport {
            physical_position: view_position,
            physical_size: view_size,
            ..default()
        });
    }

    let rounded = position.round();
    if camera_transform.translation.truncate() != rounded {
        camera_transform.translation = rounded.extend(camera_transform.translation.z);
    }

    // The screen keeps showing the old texture as it was until the new one is rendered
    if pixel_view.image != target_handle {
        return;
    }
    let (sprite, transform) = &mut *canvas;
    let rect = Some(Rect::from_corners(
        view_position.as_vec2(),
        (view_position + view_size).as_vec2(),
    ));
    if sprite.rect != rect {
        sprite.rect = rect;
    }
    let custom_size = Some(view_size.as_vec2() * zoom);
    if sprite.custom_size != custom_size {
        sprite.custom_size = custom_size;
    }
    // Both cameras have y pointing up, so the sub-pixel offset needs no flip
    transform.translation = (-(position - rounded) * zoom).extend(transform.translation.z);
}

/// Moves the overlay camera to the view, unrounded like the map on the screen
fn fit_overlay(
    window: Single<&Window, With<PrimaryWindow>>,
    pixel_view: Res<PixelView>,
    mut overlay_camera: Single<(&mut Projection, &mut Transform), With<OverlayCamera>>,
) {
    let (projection, transform) = &mut *overlay_camera;
    // `ScalingMode::WindowSize` shows `logical size * scale` units, see `fit_pixel_view`
    if let Projection::Orthographic(projection) = &mut **projection {
        let scale = window.scale_factor() / pixel_view.zoom;
        if projection.scale != scale {
            projection.scale = scale;
        }
    }
    if transform.translation.truncate() != pixel_view.position {
        transform.translation = pixel_view.position.extend(transform.translation.z);
    }
}

/// Switches the world camera on only when its texture would look different
fn redraw_on_change(
    mut redraw: ResMut<MapRedraw>,
    mut world_camera: Single<(&mut Camera, &RenderTarget, &Transform), With<TileCamera>>,
) {
    let (camera, target, transform) = &mut *world_camera;
    let view = RenderedView {
        position: transform.translation.truncate(),
        viewport: camera
            .viewport
            .as_ref()
            .map(|viewport| (viewport.physical_position, viewport.physical_size)),
        target: target_image(target).map(Handle::id),
    };
    let redraw = &mut *redraw;
    if redraw.changed {
        redraw.changed = false;
        redraw.frames_left = MAP_REDRAW_FRAMES;
    }
    if redraw.last_view.as_ref() != Some(&view) {
        redraw.frames_left = redraw.frames_left.max(1);
    }
    let active = redraw.frames_left > 0;
    if active {
        redraw.frames_left -= 1;
        redraw.last_view = Some(view);
    }
    if camera.is_active != active {
        camera.is_active = active;
    }
}

/// Smallest zoom (physical px per world px)
pub const MIN_ZOOM: f32 = 0.5;
/// Largest zoom (physical px per world px)
pub const MAX_ZOOM: f32 = 4.;

/// Minimum size of the world texture in texels for a window of `window_physical_size` at `zoom`
///
/// It has a margin of 1 texel on each side for the sub-pixel offset and the bilinear border.
pub fn required_texture_size(window_physical_size: UVec2, zoom: f32) -> UVec2 {
    (window_physical_size.as_vec2() / zoom).ceil().as_uvec2() + UVec2::splat(2)
}

/// Size of the world texture, given its `current` size and the `required` size
///
/// The current size is kept while it is at least the required size and not more than two steps
/// larger. Otherwise the required size is rounded up to a multiple of [`TEXTURE_SIZE_STEP`]. The
/// size is always even, so that the texel edges of the integer-positioned world camera land on
/// whole world pixels.
pub fn fit_texture_size(current: UVec2, required: UVec2) -> UVec2 {
    let fit = |current: u32, required: u32| {
        if current.is_multiple_of(2)
            && current >= required
            && current <= required + 2 * TEXTURE_SIZE_STEP
        {
            current
        } else {
            required.div_ceil(TEXTURE_SIZE_STEP).max(1) * TEXTURE_SIZE_STEP
        }
    };
    UVec2::new(fit(current.x, required.x), fit(current.y, required.y))
}

/// Position and size of the viewport in a texture of `texture_size` for a view that needs
/// `required` texels
///
/// The viewport is centered and its size is even, so that its center (the camera position) is a
/// texel corner. It is clamped to the texture size.
pub fn view_rect(texture_size: UVec2, required: UVec2) -> (UVec2, UVec2) {
    let even_texture_size = texture_size / 2 * 2;
    let size = ((required + UVec2::ONE) / 2 * 2).min(even_texture_size);
    ((even_texture_size - size) / 2, size)
}

/// World position of the physical screen position `screen` (y down, like the cursor)
pub fn screen_to_world(
    position: Vec2,
    zoom: f32,
    window_physical_size: Vec2,
    screen: Vec2,
) -> Vec2 {
    position + (screen - window_physical_size / 2.) / zoom * Vec2::new(1., -1.)
}

/// Camera position after zooming from `old_zoom` to `new_zoom`, keeping the world point below
/// the physical screen position `anchor` in place
pub fn zoom_around(
    position: Vec2,
    old_zoom: f32,
    new_zoom: f32,
    window_physical_size: Vec2,
    anchor: Vec2,
) -> Vec2 {
    let before = screen_to_world(position, old_zoom, window_physical_size, anchor);
    let after = screen_to_world(position, new_zoom, window_physical_size, anchor);
    position + before - after
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_texture_size_has_margin() {
        assert_eq!(
            required_texture_size(UVec2::new(800, 600), 1.),
            UVec2::new(802, 602)
        );
        assert_eq!(
            required_texture_size(UVec2::new(801, 601), 2.),
            UVec2::new(403, 303)
        );
        assert_eq!(
            required_texture_size(UVec2::new(100, 100), 0.5),
            UVec2::new(202, 202)
        );
        assert_eq!(required_texture_size(UVec2::ZERO, 1.), UVec2::new(2, 2));
    }

    #[test]
    fn fit_texture_size_rounds_up_to_steps() {
        assert_eq!(
            fit_texture_size(UVec2::ZERO, UVec2::new(802, 2)),
            UVec2::new(896, 128)
        );
        assert_eq!(
            fit_texture_size(UVec2::ZERO, UVec2::new(768, 128)),
            UVec2::new(768, 128)
        );
    }

    #[test]
    fn fit_texture_size_keeps_current_size_within_slack() {
        let current = UVec2::new(896, 640);
        assert_eq!(fit_texture_size(current, UVec2::new(802, 602)), current);
        assert_eq!(fit_texture_size(current, UVec2::new(640, 384)), current);
        // Too small
        assert_eq!(
            fit_texture_size(current, UVec2::new(897, 602)),
            UVec2::new(1024, 640)
        );
        // Too large
        assert_eq!(
            fit_texture_size(current, UVec2::new(639, 383)),
            UVec2::new(640, 384)
        );
    }

    #[test]
    fn view_rect_is_centered_and_even() {
        assert_eq!(
            view_rect(UVec2::new(1024, 768), UVec2::new(802, 603)),
            (UVec2::new(111, 82), UVec2::new(802, 604))
        );
        assert_eq!(
            view_rect(UVec2::new(1024, 768), UVec2::new(1024, 768)),
            (UVec2::ZERO, UVec2::new(1024, 768))
        );
    }

    #[test]
    fn view_rect_is_clamped_to_texture() {
        assert_eq!(
            view_rect(UVec2::new(256, 128), UVec2::new(300, 200)),
            (UVec2::ZERO, UVec2::new(256, 128))
        );
    }

    #[test]
    fn screen_center_is_camera_position() {
        let position = Vec2::new(12.5, -3.25);
        let size = Vec2::new(800., 600.);
        assert_eq!(screen_to_world(position, 2., size, size / 2.), position);
    }

    #[test]
    fn screen_to_world_flips_y_and_scales() {
        let size = Vec2::new(800., 600.);
        let world = screen_to_world(Vec2::new(100., 100.), 2., size, Vec2::new(500., 200.));
        assert_eq!(world, Vec2::new(150., 150.));
    }

    #[test]
    fn zoom_around_keeps_anchor_in_place() {
        let size = Vec2::new(800., 600.);
        let anchor = Vec2::new(123., 456.);
        let position = Vec2::new(-40., 70.);
        let before = screen_to_world(position, 1., size, anchor);
        let new_position = zoom_around(position, 1., 2.5, size, anchor);
        let after = screen_to_world(new_position, 2.5, size, anchor);
        assert!(before.distance(after) < 1e-3);
    }

    #[test]
    fn zoom_around_center_keeps_position() {
        let size = Vec2::new(800., 600.);
        let position = Vec2::new(-40., 70.);
        assert_eq!(zoom_around(position, 1., 3., size, size / 2.), position);
    }
}
