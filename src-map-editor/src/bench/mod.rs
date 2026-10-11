//! Native benchmark of the GPU memory and frame times of the viewer, see the README
//!
//! The viewer runs with the same plugins as in the toolset, but on a synthetic map instead of
//! game files, so it needs neither the host nor the game data. Scenarios edit the map every frame
//! through the editor, and the GPU memory is read from the allocator of wgpu (Vulkan and DX12).

use std::collections::HashMap;

use bevy::{
    prelude::*,
    render::diagnostic::RenderDiagnosticsPlugin,
    window::{PresentMode, PrimaryWindow, WindowResolution},
};

use crate::{
    editor::{
        commands::CurrentMode, document::EditorDocument, soldiers::SoldierMode,
        soldiers::SoldierTeam,
    },
    light::LightTemplates,
    map::{light_template_name, placement::AppearanceContext, Level, TacticalMap},
    render::{
        items::add_items,
        movement_costs::MovementCosts,
        soldiers::{add_soldiers, SoldierFiles},
        world::MapWorld,
        MapSoldiers, PendingSceneUpdate, ReplayAnimation, ReplayScene, SceneSources,
    },
    soldier::{palette_files::PaletteReplacements, SoldierRegistries, SoldierSystems},
    states::{
        loading::{MapAssets, MapToLoad},
        AppState,
    },
    systems::{pixel_view::PixelView, MapSystems},
    tileset::{StciSheet, Tileset, TilesetId},
};

mod args;
mod report;
mod scenario;
mod synthetic;
mod verify;

use args::{Args, Command, Scenario};
use report::Report;
use synthetic::MapTiles;

/// Exit code for invalid arguments
const USAGE_ERROR: i32 = 2;

/// Runs the benchmark with the arguments of the process, returns the exit code
pub fn run() -> i32 {
    let args = match args::parse(std::env::args().skip(1)) {
        Ok(Command::Run(args)) => args,
        Ok(Command::List) => {
            for (name, description) in Scenario::DESCRIPTIONS {
                println!("{:<12} {}", name, description);
            }
            return 0;
        }
        Ok(Command::Help) => {
            print!("{}", args::USAGE);
            return 0;
        }
        Err(error) => {
            eprintln!("error: {}\n\n{}", error, args::USAGE);
            return USAGE_ERROR;
        }
    };
    let report = match Report::new(args.out.as_deref()) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: can't create the output file: {}", error);
            return USAGE_ERROR;
        }
    };

    let mut resolution = WindowResolution::new(args.width, args.height);
    if let Some(scale) = args.scale {
        resolution.set_scale_factor_override(Some(scale));
    }
    let window = Window {
        title: "Ja2 Map Viewer benchmark".into(),
        resolution,
        present_mode: if args.no_vsync {
            PresentMode::AutoNoVsync
        } else {
            PresentMode::AutoVsync
        },
        // Tiling window managers keep the size of windows that can't be resized
        resizable: false,
        ..default()
    };
    let mut app = App::new();
    crate::build_app(&mut app, window, false);
    // GPU time of the render passes, where the adapter has timestamp queries
    app.add_plugins(RenderDiagnosticsPlugin)
        .insert_resource(Bench::new(args))
        .insert_resource(report)
        .init_resource::<verify::Verify>()
        .add_systems(Startup, start_loading)
        .add_systems(
            OnEnter(AppState::Loading),
            load_synthetic_map.after(SoldierSystems::Register),
        )
        .add_systems(
            OnEnter(AppState::Map),
            (report::start_scenario, verify::reset),
        )
        .add_systems(
            Update,
            (verify::step, scenario::drive)
                .chain()
                // The light preview scenario moves the cursor
                .after(MapSystems::Cursor)
                .before(MapSystems::Edit)
                .run_if(in_state(AppState::Map)),
        )
        .add_systems(
            Last,
            (report::measure, next_scenario)
                .chain()
                .run_if(in_state(AppState::Map)),
        );
    match app.run() {
        AppExit::Success => 0,
        AppExit::Error(code) => i32::from(code.get()),
    }
}

/// The arguments and the progress through the scenarios
#[derive(Resource)]
struct Bench {
    args: Args,
    /// Index of the current scenario in `args.scenarios`
    current: usize,
    /// The current scenario is over
    finished: bool,
    /// Something went wrong, the process exits with an error after the last scenario
    failed: bool,
}

impl Bench {
    fn new(args: Args) -> Self {
        Self {
            args,
            current: 0,
            finished: false,
            failed: false,
        }
    }

    fn scenario(&self) -> Scenario {
        self.args.scenarios[self.current]
    }
}

/// The tiles of the synthetic map and what the scenarios place
#[derive(Resource)]
struct Synthetic {
    tiles: MapTiles,
    light: std::sync::Arc<crate::light::LightTemplate>,
}

fn start_loading(mut next_state: ResMut<NextState<AppState>>) {
    next_state.set(AppState::Loading);
}

/// Inserts everything that loading a map from files would, like `poll_tile_sheets`
///
/// The window gets the requested physical size again, the compositor may have scaled it and the
/// zoom scenario resizes it.
fn load_synthetic_map(
    mut commands: Commands,
    bench: Res<Bench>,
    mut window: Single<&mut Window, With<PrimaryWindow>>,
    registries: SoldierRegistries,
    mut sheets: ResMut<Assets<StciSheet>>,
    mut replacements: ResMut<Assets<PaletteReplacements>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    let scenario = bench.scenario();
    info!("Building the synthetic map for scenario {}", scenario);
    window
        .resolution
        .set_physical_resolution(bench.args.width, bench.args.height);
    let definitions = registries.definitions();
    let (tileset, surface_sheets) = synthetic::tileset();
    let soldier_sheets = synthetic::soldier_sheets(definitions.surfaces);
    let palettes = synthetic::soldier_palettes();
    let items = synthetic::item_table();
    let tiles = MapTiles::new(&tileset);
    let light = synthetic::omni_light_template(synthetic::LIGHT_RADIUS);

    let mut map = TacticalMap::empty(TilesetId::new(0));
    for position in synthetic::positions() {
        synthetic::fill_tile(&mut map.tiles[position.into()], &tiles, position, 0);
    }
    map.lights = synthetic::lights(&light, 0);
    map.items = synthetic::items(&tiles, 0);
    let mut world = MapWorld::build(&map);

    // Soldiers are placed like the editor does, so that they get appearances and palettes
    let sc = crate::editor::soldiers::SoldierContext {
        definitions,
        appearance: AppearanceContext {
            items: &items,
            bodies: definitions.bodies,
            palettes: Some(&palettes),
        },
    };
    let mut document = EditorDocument::new(1);
    let mut context =
        crate::editor::document::EditContext::new(&mut map, &mut world, &tileset, &mut document);
    let teams = [
        SoldierTeam::Enemy,
        SoldierTeam::Rebel,
        SoldierTeam::Civilian,
    ];
    for (index, team) in teams.into_iter().enumerate() {
        let mode = SoldierMode { team, ..default() };
        let per_team = crate::editor::soldiers::MAX_TEAM_SOLDIERS;
        for grid_no in synthetic::spread(per_team, 2 + index as u32) {
            // Two teams can pick the same tile, that soldier is left out
            let _ = context.place_soldier(&sc, &mode, grid_no, Level::Ground);
        }
    }
    context.finish();

    add_items(&mut world, &map);
    let costs = MovementCosts::compile(&mut world, &map);
    let mut soldiers = MapSoldiers::default();
    add_soldiers(
        &mut world,
        &map.soldiers,
        |_| true,
        &SoldierFiles {
            surfaces: soldier_sheets.iter().map(Some).collect(),
            replacements: Some(&palettes),
            monster_palettes: vec![],
        },
        &mut soldiers,
    );
    info!(
        "Synthetic map: {} soldiers, {} items, {} lights",
        map.soldiers.len(),
        map.items.len(),
        map.lights.len()
    );

    commands.insert_resource(MapAssets {
        tilesets: Handle::default(),
        map: Handle::default(),
        tile_definitions: vec![],
        surface_sheets: surface_sheets
            .into_iter()
            .map(|sheet| sheets.add(sheet))
            .collect(),
        structure_check: None,
        light_templates: vec![],
        item_files: vec![],
        soldier_surfaces: soldier_sheets
            .into_iter()
            .map(|sheet| sheets.add(sheet))
            .collect(),
        soldier_palettes: replacements.add(palettes),
        monster_palettes: vec![],
    });
    commands.insert_resource(Synthetic {
        tiles,
        light: light.clone(),
    });
    commands.insert_resource(MapToLoad(format!("bench/{}.dat", scenario), None));
    let mut templates = HashMap::from([(synthetic::LIGHT_TEMPLATE.to_lowercase(), light)]);
    if let Scenario::LightPreview(radius) = scenario {
        templates.insert(
            light_template_name(radius).to_lowercase(),
            synthetic::omni_light_template(radius),
        );
    }
    commands.insert_resource(LightTemplates(templates));
    commands.insert_resource(world);
    commands.insert_resource(costs);
    commands.insert_resource(map);
    commands.insert_resource(soldiers);
    commands.insert_resource(items);
    commands.insert_resource(tileset);
    commands.insert_resource(PendingSceneUpdate::full());
    next_state.set(AppState::Map);
}

/// Starts the next scenario on a fresh map once the current one is over, or exits after the last
fn next_scenario(
    mut commands: Commands,
    mut bench: ResMut<Bench>,
    mut pixel_view: ResMut<PixelView>,
    mut next_state: ResMut<NextState<AppState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if !bench.finished {
        return;
    }
    bench.finished = false;
    bench.current += 1;
    if bench.current >= bench.args.scenarios.len() {
        exit.write(if bench.failed {
            AppExit::error()
        } else {
            AppExit::Success
        });
        return;
    }
    // Like `switch_map`
    commands.remove_resource::<MapWorld>();
    commands.remove_resource::<MovementCosts>();
    commands.remove_resource::<TacticalMap>();
    commands.remove_resource::<MapSoldiers>();
    commands.remove_resource::<crate::item::ItemTable>();
    commands.remove_resource::<LightTemplates>();
    commands.remove_resource::<Tileset>();
    commands.insert_resource(CurrentMode::default());
    commands.insert_resource(ReplayScene::default());
    commands.insert_resource(ReplayAnimation::default());
    commands.insert_resource(SceneSources::default());
    commands.insert_resource(PendingSceneUpdate::default());
    pixel_view.position = Vec2::ZERO;
    next_state.set(AppState::Loading);
}
