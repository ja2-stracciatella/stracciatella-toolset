//! Renders the radar map of the loaded map for the host, see `render/radar.rs`
//!
//! The map is drawn once more with the overhead draw order and the lighting of the game's map
//! utility, into an offscreen texture of the area of the overhead image, which is read back and
//! shrunk to the radar map. The scene and the view of the editor are left alone.

use bevy::{
    image::ImageSampler,
    prelude::*,
    render::{
        gpu_readback::{Readback, ReadbackComplete},
        render_resource::TextureUsages,
        storage::ShaderBuffer,
    },
};

use crate::{
    host::{finish_radars, requested_radar_maps},
    map::TacticalMap,
    map_requested,
    render::{
        atlas::{current_animation_frames, PackedDraws, SceneAtlas, DRAW_WORDS},
        radar::{light_for_radar, overhead_draws, radar_map, radar_rect},
        replay_pass::{
            storage_buffer, PendingReplayBindings, ReadyReplayBindings, ReplayBindings, ReplayPass,
            ReplayPasses, REPLAY_TEXTURE_FORMAT,
        },
        sprite_sheets,
        world::MapWorld,
        MapSoldiers,
    },
    states::{
        loading::{LoadingError, MapAssets, MapToLoad},
        AppState, LoadingPhase,
    },
    tileset::{StciSheet, Tileset},
};

/// How many frames the radar pass is drawn once its bindings are ready. More than one, in case a
/// pipeline only becomes ready a frame later.
const RENDER_FRAMES: u32 = 2;

/// Where a radar map render is
#[derive(Debug)]
enum Stage {
    /// Waiting for the render world to prepare the bindings of this generation
    WaitingForBindings(u32),
    /// The pass is drawn for this many more frames
    Rendering(u32),
    /// The texture is being read back
    ReadingBack,
}

/// A radar map that is being rendered
#[derive(Debug)]
struct RadarJob {
    map: String,
    restricted: bool,
    image: Handle<Image>,
    readback: Option<Entity>,
    stage: Stage,
}

#[derive(Resource, Debug, Default)]
struct RadarRender {
    job: Option<RadarJob>,
    /// The pixels read back from the texture of the job, with the bytes per row
    pixels: Option<(Vec<u8>, usize)>,
}

pub struct RadarPlugin;

impl Plugin for RadarPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RadarRender>()
            // In every state, so requests for maps that fail to load are answered
            .add_systems(Update, render_radar_maps.after(super::MapSystems::Scene));
    }
}

/// Whether the map that a radar map was requested for can be rendered
enum MapReadiness {
    Ready,
    Waiting,
    Failed(String),
}

fn map_readiness(
    map: &str,
    state: &AppState,
    phase: Option<&LoadingPhase>,
    map_to_load: Option<&MapToLoad>,
    error: Option<&LoadingError>,
) -> MapReadiness {
    // The map that was requested last is switched to in the next frame
    if map_requested() {
        return MapReadiness::Waiting;
    }
    if map_to_load.is_none_or(|loading| loading.0 != map) {
        return MapReadiness::Failed(format!("The map {} is not loaded", map));
    }
    match (state, phase) {
        (AppState::Map, _) => MapReadiness::Ready,
        (AppState::Loading, Some(LoadingPhase::Failed)) => MapReadiness::Failed(error.map_or_else(
            || format!("The map {} could not be loaded", map),
            |e| e.0.clone(),
        )),
        (AppState::Loading, _) => MapReadiness::Waiting,
        (AppState::Idle, _) => MapReadiness::Failed(format!("The map {} is not loaded", map)),
    }
}

/// What starting a radar map render needs
#[derive(bevy::ecs::system::SystemParam)]
struct RadarSources<'w> {
    world: Option<Res<'w, MapWorld>>,
    map: Option<Res<'w, TacticalMap>>,
    tileset: Option<Res<'w, Tileset>>,
    map_assets: Option<Res<'w, MapAssets>>,
    soldiers: Option<Res<'w, MapSoldiers>>,
    sheets: Res<'w, Assets<StciSheet>>,
    images: ResMut<'w, Assets<Image>>,
    buffers: ResMut<'w, Assets<ShaderBuffer>>,
    pending: ResMut<'w, PendingReplayBindings>,
    passes: ResMut<'w, ReplayPasses>,
}

/// Answers the requests of the host for radar maps, one map at a time
fn render_radar_maps(
    mut commands: Commands,
    mut radar: ResMut<RadarRender>,
    state: Res<State<AppState>>,
    phase: Option<Res<State<LoadingPhase>>>,
    map_to_load: Option<Res<MapToLoad>>,
    error: Option<Res<LoadingError>>,
    ready: Res<ReadyReplayBindings>,
    mut sources: RadarSources,
) {
    let radar = &mut *radar;
    let readiness = |map: &str| {
        map_readiness(
            map,
            state.get(),
            phase.as_deref().map(State::get),
            map_to_load.as_deref(),
            error.as_deref(),
        )
    };

    if let Some(job) = &mut radar.job {
        // Another map was loaded in the meantime
        if !matches!(readiness(&job.map), MapReadiness::Ready) {
            let job = radar.job.take().unwrap();
            radar.pixels = None;
            finish_radars(
                &job.map,
                Err(format!("The map {} was replaced while rendering", job.map)),
            );
            despawn_job(&mut commands, &mut sources.passes, job);
        }
    }

    if radar.job.is_none() {
        for map in requested_radar_maps() {
            match readiness(&map) {
                MapReadiness::Waiting => {}
                MapReadiness::Failed(message) => finish_radars(&map, Err(message)),
                MapReadiness::Ready if radar.job.is_none() => {
                    match start_job(&mut sources, map.clone()) {
                        Ok(job) => radar.job = Some(job),
                        Err(message) => finish_radars(&map, Err(message)),
                    }
                }
                // Started after the current one
                MapReadiness::Ready => {}
            }
        }
    }

    let Some(job) = &mut radar.job else {
        return;
    };
    match job.stage {
        Stage::WaitingForBindings(generation) => {
            if ready.radar() == generation {
                set_pass_active(&mut sources.passes, true);
                job.stage = Stage::Rendering(RENDER_FRAMES);
            }
        }
        Stage::Rendering(frames) => {
            let frames = frames.saturating_sub(1);
            job.stage = Stage::Rendering(frames);
            if frames == 0 {
                set_pass_active(&mut sources.passes, false);
                let readback = commands
                    .spawn(Readback::texture(job.image.clone()))
                    .observe(on_readback)
                    .id();
                job.readback = Some(readback);
                job.stage = Stage::ReadingBack;
            }
        }
        Stage::ReadingBack => {
            let Some((pixels, stride)) = radar.pixels.take() else {
                return;
            };
            let job = radar.job.take().unwrap();
            let size = radar_rect().size().as_uvec2();
            let result = radar_map(
                &pixels,
                size.x as usize,
                size.y as usize,
                stride,
                job.restricted,
            )
            .map_err(|e| format!("Error writing the radar map of {}: {}", job.map, e));
            if let Err(message) = &result {
                error!("{}", message);
            } else {
                info!("Rendered the radar map of {}", job.map);
            }
            finish_radars(&job.map, result);
            despawn_job(&mut commands, &mut sources.passes, job);
        }
    }
}

fn set_pass_active(passes: &mut ReplayPasses, active: bool) {
    if let Some(pass) = &mut passes.radar {
        pass.active = active;
    }
}

/// Keeps the first read back pixels, a readback repeats until it is removed
fn on_readback(
    mut event: On<ReadbackComplete>,
    mut commands: Commands,
    mut radar: ResMut<RadarRender>,
) {
    let entity = event.entity;
    commands.entity(entity).remove::<Readback>();
    let Some(job) = &radar.job else {
        return;
    };
    if job.readback != Some(entity) || radar.pixels.is_some() {
        return;
    }
    let width = radar_rect().size().x as usize;
    // Rows are padded to the alignment of texture copies
    let stride = (width * 4).next_multiple_of(256);
    radar.pixels = Some((std::mem::take(&mut event.data), stride));
}

fn despawn_job(commands: &mut Commands, passes: &mut ReplayPasses, job: RadarJob) {
    passes.radar = None;
    if let Some(readback) = job.readback {
        commands.entity(readback).despawn();
    }
}

/// Builds the overhead draws of the loaded map and sets up the pass that draws them
fn start_job(sources: &mut RadarSources, map_path: String) -> Result<RadarJob, String> {
    let (Some(world), Some(map), Some(tileset), Some(map_assets), Some(soldiers)) = (
        sources.world.as_deref(),
        sources.map.as_deref(),
        sources.tileset.as_deref(),
        sources.map_assets.as_deref(),
        sources.soldiers.as_deref(),
    ) else {
        return Err(format!("The map {} is not loaded", map_path));
    };
    let mut world = world.clone();
    let lighting = light_for_radar(&mut world, map);
    let sheets = sprite_sheets(tileset, map_assets, &sources.sheets, soldiers);
    let draws = overhead_draws(&world, |key| sheets.geometry(key));
    let rect = radar_rect();
    let mut atlas = SceneAtlas::new(lighting.light_color);
    let packed = PackedDraws::build(&draws, rect, &mut atlas, &sheets);
    let shared = atlas.shared_buffers();
    let frames = current_animation_frames(&shared.animations, tileset);
    debug!(
        "Rendering the radar map of {} with {} draws",
        map_path,
        draws.len()
    );

    let buffers = &mut sources.buffers;
    let bindings = ReplayBindings {
        bins_origin: packed.bins_origin,
        bins_count: packed.bins_count,
        bin_indices_offset: packed.bin_indices_offset,
        animations_offset: shared.animations_offset,
        draws: storage_buffer(buffers, &packed.draws, DRAW_WORDS),
        bins: storage_buffer(buffers, &packed.bins, 1),
        atlas: storage_buffer(buffers, &shared.atlas, 1),
        palettes: storage_buffer(buffers, &shared.palettes, 1),
        sprites: storage_buffer(buffers, &shared.sprites, 1),
        animation_frames: storage_buffer(buffers, &frames, 1),
    };
    let generation = sources.pending.radar.wait_for(&bindings);

    let size = rect.size().as_uvec2();
    let mut image = Image::new_target_texture(size.x, size.y, REPLAY_TEXTURE_FORMAT, None);
    image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    image.sampler = ImageSampler::nearest();
    image.data = None;
    image.copy_on_resize = false;
    let image = sources.images.add(image);
    sources.passes.radar = Some(ReplayPass {
        target: image.clone(),
        viewport_position: UVec2::ZERO,
        viewport_size: size,
        origin: rect.min,
        clear_color: Color::BLACK,
        bindings: Some(bindings),
        active: false,
    });
    Ok(RadarJob {
        map: map_path,
        restricted: world.restricted_scroll,
        image,
        readback: None,
        stage: Stage::WaitingForBindings(generation),
    })
}
