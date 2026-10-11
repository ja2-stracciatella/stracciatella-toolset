//! Compares the rendered map with the CPU replay of the draw list (`render/reference.rs`)
//!
//! After the warmup, the edits of the scenario pause and the view steps at zoom 1 over a grid
//! of positions covering the map. At each position the world texture is read back and every
//! texel within the map bounds is compared with the replay of the whole map. The gizmos of the
//! editor, which the world camera draws on top of the map, are switched off meanwhile.

use std::sync::Arc;

use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::gpu_readback::{Readback, ReadbackComplete},
    window::PrimaryWindow,
};
use serde_json::{json, Value};

use super::{report::Report, Bench};
use crate::{
    components::{
        camera::TileCamera,
        map::{game_rect_to_world, map_bounds, world_to_game},
    },
    render::{
        atlas::{GpuScene, PackedDraws},
        palette::pack_rgba,
        sprite_sheets, MapSoldiers, PendingSceneUpdate, RendererIdle, SceneSources,
    },
    states::loading::MapAssets,
    systems::pixel_view::{MapRedraw, PixelView},
    tileset::{StciSheet, Tileset},
    CLEAR_COLOR,
};

/// Frames that the scene and the renderer have to be idle before the reference is built, and
/// that the view has to be at a position before it is read back
const SETTLE_FRAMES: u32 = 3;
/// Number of mismatches listed in the summary
const LISTED_MISMATCHES: usize = 10;
/// Texels of overlap between the views of neighbouring steps
const STEP_OVERLAP: f32 = 64.;

#[derive(Debug, Default)]
enum Stage {
    /// Waits for the end of the warmup
    #[default]
    Warmup,
    /// Waits for the scene and the renderer to be idle for this many frames
    Settling(u32),
    /// The view is at the step, for this many frames
    Step {
        index: usize,
        frames: u32,
    },
    /// The texture of the step is being read back
    Reading {
        index: usize,
        view: ReadView,
    },
    Done,
}

/// Where the world camera was when the texture was read back
#[derive(Debug, Clone, Copy, PartialEq)]
struct ReadView {
    /// World position of the camera, an integer
    position: Vec2,
    viewport_position: UVec2,
    viewport_size: UVec2,
    texture_width: u32,
}

impl ReadView {
    /// The game pixel shown by a texel
    fn game_pixel(&self, texel: UVec2) -> IVec2 {
        let offset =
            texel.as_vec2() - self.viewport_position.as_vec2() - self.viewport_size.as_vec2() / 2.
                + 0.5;
        // Texel rows go down, world y goes up
        let world = self.position + Vec2::new(offset.x, -offset.y);
        world_to_game(world).floor().as_ivec2()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Mismatch {
    game: IVec2,
    expected: [u8; 3],
    actual: [u8; 3],
}

#[derive(Debug, Default)]
struct Comparison {
    compared: u64,
    mismatched: u64,
    /// The first mismatches
    first: Vec<Mismatch>,
}

impl Comparison {
    fn add(&mut self, other: Comparison) {
        self.compared += other.compared;
        self.mismatched += other.mismatched;
        let room = LISTED_MISMATCHES.saturating_sub(self.first.len());
        self.first.extend(other.first.into_iter().take(room));
    }
}

/// State of the verification of the current scenario
#[derive(Resource, Default)]
pub struct Verify {
    stage: Stage,
    /// Camera positions of the steps
    steps: Vec<Vec2>,
    reference: Option<Arc<GpuScene>>,
    /// The pixels read back for the current step, with the bytes per row
    pixels: Option<(Vec<u8>, usize)>,
    readback: Option<Entity>,
    result: Comparison,
    /// The view before the verification, restored after it
    restore: Option<(Vec2, f32, f32)>,
}

impl Verify {
    /// Whether the view and the map have to stay as they are
    pub fn is_running(&self) -> bool {
        !matches!(self.stage, Stage::Warmup | Stage::Done)
    }
}

/// What the reference is built from
#[derive(bevy::ecs::system::SystemParam)]
pub struct ReferenceSources<'w> {
    sources: Res<'w, SceneSources>,
    tileset: Option<Res<'w, Tileset>>,
    map_assets: Option<Res<'w, MapAssets>>,
    soldiers: Option<Res<'w, MapSoldiers>>,
    sheets: Res<'w, Assets<StciSheet>>,
}

impl ReferenceSources<'_> {
    /// The replay data of the whole map, with the atlas of the scene
    fn build(&self) -> Option<GpuScene> {
        let (tileset, map_assets, soldiers) = (
            self.tileset.as_deref()?,
            self.map_assets.as_deref()?,
            self.soldiers.as_deref()?,
        );
        let sheets = sprite_sheets(tileset, map_assets, &self.sheets, soldiers);
        // The atlas already has everything that the draws need, a copy keeps the scene's as is
        let mut atlas = self.sources.atlas.clone();
        let packed =
            PackedDraws::build(&self.sources.draws.all(), map_bounds(), &mut atlas, &sheets);
        Some(GpuScene::from_parts(
            &atlas.shared_buffers(),
            &packed,
            tileset,
        ))
    }
}

/// Camera positions at zoom 1 whose views cover the map
fn grid_steps(view_size: Vec2) -> Vec<Vec2> {
    let bounds = map_bounds();
    let world = game_rect_to_world(bounds.min, bounds.max);
    let step = (view_size - STEP_OVERLAP).max(Vec2::ONE);
    let count = (world.size() / step).ceil().max(Vec2::ONE).as_uvec2();
    let first = world.min + view_size / 2.;
    let mut steps = vec![];
    for y in 0..count.y {
        for x in 0..count.x {
            let position = (first + UVec2::new(x, y).as_vec2() * step).min(world.max);
            steps.push(position.round());
        }
    }
    steps
}

/// Steps through the verification, between the warmup and the measurement of the scenario
pub fn step(
    mut commands: Commands,
    bench: Res<Bench>,
    mut verify: ResMut<Verify>,
    mut report: ResMut<Report>,
    mut pixel_view: ResMut<PixelView>,
    pending: Res<PendingSceneUpdate>,
    idle: Res<RendererIdle>,
    redraw: Res<MapRedraw>,
    reference: ReferenceSources,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &RenderTarget, &Transform), With<TileCamera>>,
    mut last_position: Local<Option<Vec2>>,
) {
    if !bench.args.verify {
        return;
    }
    let verify = &mut *verify;
    let settled = pending.is_empty() && idle.0 && redraw.is_idle();
    verify.stage = match std::mem::take(&mut verify.stage) {
        Stage::Warmup => {
            if report.frame().is_none() || report.elapsed() < bench.args.warmup.as_secs_f32() {
                Stage::Warmup
            } else {
                verify.restore =
                    Some((pixel_view.position, pixel_view.zoom, pixel_view.target_zoom));
                report.set_paused(true);
                info!("Verifying the rendered map");
                Stage::Settling(0)
            }
        }
        Stage::Settling(frames) => {
            let frames = if settled { frames + 1 } else { 0 };
            if frames < SETTLE_FRAMES {
                Stage::Settling(frames)
            } else if let Some(scene) = reference.build() {
                verify.reference = Some(Arc::new(scene));
                verify.steps = grid_steps(window.physical_size().as_vec2());
                Stage::Step {
                    index: 0,
                    frames: 0,
                }
            } else {
                report.set_verification(json!({ "error": "no map to verify" }));
                finish(verify, &mut report, &mut pixel_view)
            }
        }
        Stage::Step { index, frames } => {
            let target = verify.steps[index];
            if pixel_view.position != target || pixel_view.zoom != 1. {
                pixel_view.position = target;
                pixel_view.zoom = 1.;
                pixel_view.target_zoom = 1.;
            }
            // Wait until the camera has stayed at the clamped position for a few frames and
            // the screen shows its texture
            let (camera, render_target, transform) = *camera;
            let position = transform.translation.truncate();
            let still = *last_position == Some(position);
            *last_position = Some(position);
            let shown = matches!(render_target, RenderTarget::Image(image) if image.handle == pixel_view.image);
            let frames = if settled && still && shown {
                frames + 1
            } else {
                0
            };
            match (&camera.viewport, camera.physical_target_size()) {
                (Some(viewport), Some(size)) if frames >= SETTLE_FRAMES => {
                    let view = ReadView {
                        position,
                        viewport_position: viewport.physical_position,
                        viewport_size: viewport.physical_size,
                        texture_width: size.x,
                    };
                    let readback = commands
                        .spawn(Readback::texture(pixel_view.image.clone()))
                        .observe(on_readback)
                        .id();
                    verify.readback = Some(readback);
                    verify.pixels = None;
                    Stage::Reading { index, view }
                }
                _ => Stage::Step { index, frames },
            }
        }
        Stage::Reading { index, view } => {
            let (Some((pixels, stride)), Some(scene)) =
                (verify.pixels.take(), verify.reference.clone())
            else {
                verify.stage = Stage::Reading { index, view };
                return;
            };
            if let Some(readback) = verify.readback.take() {
                commands.entity(readback).despawn();
            }
            let comparison = compare(&pixels, stride, view, &scene);
            debug!(
                "Verified step {} at {}: {} of {} pixels differ",
                index, view.position, comparison.mismatched, comparison.compared
            );
            verify.result.add(comparison);
            if index + 1 < verify.steps.len() {
                Stage::Step {
                    index: index + 1,
                    frames: 0,
                }
            } else {
                let result = &verify.result;
                info!(
                    "Verified {} pixels in {} steps, {} differ",
                    result.compared,
                    verify.steps.len(),
                    result.mismatched
                );
                report.set_verification(json!({
                    "steps": verify.steps.len(),
                    "compared_pixels": result.compared,
                    "mismatched_pixels": result.mismatched,
                    "first_mismatches": result.first.iter().map(mismatch_json).collect::<Vec<_>>(),
                }));
                finish(verify, &mut report, &mut pixel_view)
            }
        }
        Stage::Done => Stage::Done,
    };
}

fn mismatch_json(mismatch: &Mismatch) -> Value {
    json!({
        "game": [mismatch.game.x, mismatch.game.y],
        "expected": mismatch.expected,
        "actual": mismatch.actual,
    })
}

/// Restores the view and lets the scenario continue, returns the stage after the verification
fn finish(verify: &mut Verify, report: &mut Report, pixel_view: &mut PixelView) -> Stage {
    if let Some((position, zoom, target_zoom)) = verify.restore.take() {
        pixel_view.position = position;
        pixel_view.zoom = zoom;
        pixel_view.target_zoom = target_zoom;
    }
    verify.reference = None;
    report.set_paused(false);
    Stage::Done
}

/// Keeps the first read back pixels, a readback repeats until it is removed
fn on_readback(
    mut event: On<ReadbackComplete>,
    mut commands: Commands,
    mut verify: ResMut<Verify>,
) {
    let entity = event.entity;
    commands.entity(entity).remove::<Readback>();
    if verify.readback != Some(entity) || verify.pixels.is_some() {
        return;
    }
    let Stage::Reading { view, .. } = verify.stage else {
        return;
    };
    // Rows are padded to the alignment of texture copies
    let stride = (view.texture_width as usize * 4).next_multiple_of(256);
    verify.pixels = Some((std::mem::take(&mut event.data), stride));
}

/// Compares the texels of the viewport within the map bounds with the replay, on all cores
fn compare(pixels: &[u8], stride: usize, view: ReadView, scene: &GpuScene) -> Comparison {
    let clear = pack_rgba(CLEAR_COLOR.to_srgba().to_u8_array_no_alpha());
    let bounds = map_bounds();
    let rows: Vec<u32> = (0..view.viewport_size.y)
        .map(|row| view.viewport_position.y + row)
        .filter(|y| (*y as usize + 1) * stride <= pixels.len())
        .collect();
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get());
    let chunk = rows.len().div_ceil(threads).max(1);
    let mut total = Comparison::default();
    std::thread::scope(|scope| {
        let handles: Vec<_> = rows
            .chunks(chunk)
            .map(|rows| {
                scope.spawn(move || {
                    let mut comparison = Comparison::default();
                    for &y in rows {
                        for x in view.viewport_position.x
                            ..view.viewport_position.x + view.viewport_size.x
                        {
                            let game = view.game_pixel(UVec2::new(x, y));
                            if !bounds.contains(game) {
                                continue;
                            }
                            let offset = y as usize * stride + x as usize * 4;
                            let actual = [pixels[offset], pixels[offset + 1], pixels[offset + 2]];
                            let expected = scene.replay_pixel(game, clear).to_le_bytes();
                            let expected = [expected[0], expected[1], expected[2]];
                            comparison.compared += 1;
                            if actual != expected {
                                comparison.mismatched += 1;
                                if comparison.first.len() < LISTED_MISMATCHES {
                                    comparison.first.push(Mismatch {
                                        game,
                                        expected,
                                        actual,
                                    });
                                }
                            }
                        }
                    }
                    comparison
                })
            })
            .collect();
        for handle in handles {
            if let Ok(comparison) = handle.join() {
                total.add(comparison);
            }
        }
    });
    total
}

/// Starts the verification of the next scenario from the beginning
pub fn reset(mut verify: ResMut<Verify>) {
    *verify = Verify::default();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::map::game_to_world;

    #[test]
    fn texels_map_to_game_pixels() {
        let view = ReadView {
            position: Vec2::new(100., -50.),
            viewport_position: UVec2::new(10, 20),
            viewport_size: UVec2::new(8, 6),
            texture_width: 32,
        };
        // The texel right below and right of the center shows the game pixel whose top left
        // corner is at the camera position
        let center = view.game_pixel(UVec2::new(14, 23));
        assert_eq!(game_to_world(center), view.position);
        assert_eq!(view.game_pixel(UVec2::new(15, 23)), center + IVec2::X);
        assert_eq!(view.game_pixel(UVec2::new(14, 24)), center + IVec2::Y);
        assert_eq!(
            view.game_pixel(UVec2::new(10, 20)),
            center - IVec2::new(4, 3)
        );
    }

    #[test]
    fn grid_steps_cover_the_map() {
        let view = Vec2::new(1920., 1080.);
        let steps = grid_steps(view);
        let bounds = map_bounds();
        let world = game_rect_to_world(bounds.min, bounds.max);
        for corner in [world.min, world.max, Vec2::new(world.min.x, world.max.y)] {
            assert!(
                steps
                    .iter()
                    .any(|step| Rect::from_center_size(*step, view).contains(corner)),
                "{corner}"
            );
        }
    }
}
