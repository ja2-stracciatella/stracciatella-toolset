use std::{
    collections::{BTreeSet, HashMap},
    sync::Arc,
};

use bevy::{
    asset::LoadState,
    prelude::*,
    tasks::{block_on, futures_lite::future, IoTaskPool, Task},
};

use crate::{
    file_formats::{
        ja2_map::Ja2Map,
        ja2_structure::Ja2StructureFile,
        ja2_tile_database::{Ja2TileDatabase, TileSurfaceInfo},
        ja2_tile_types::{TerrainType, NUMBER_OF_TILE_TYPES, TILE_TYPE_NAMES},
        ja2_tileset::Ja2Tilesets,
    },
    host::{self, report_status, MapStatus, JA2_ASSET_SOURCE},
    item::{ItemFile, ItemTable, ITEM_FILES},
    light::{LightTemplate, LightTemplates, LIGHT_RADII},
    map::{light_template_name, new_ja2_map, MapContext, TacticalMap},
    random::Random,
    render::{
        items::add_items,
        movement_costs::MovementCosts,
        soldiers::{add_soldiers, SoldierFiles},
        world::MapWorld,
        MapSoldiers, PendingSceneUpdate, ReplayAnimation, ReplayScene, SceneSources,
    },
    soldier::{
        palette_files::{ColPalette, PaletteReplacements, SOLDIER_PALETTE_FILE},
        SoldierBodyTypes, SoldierRegistries, SoldierSystems, StandingSurfaces,
    },
    states::{AppState, LoadingPhase, UiFont},
    systems::{pixel_view::PixelView, MapSystems},
    take_requested_map,
    tileset::{StciSheet, Tileset, TilesetId},
    MapRequest, NewMapOptions,
};

/// Path of the map that should be loaded, relative to the data root, and how to create it if it
/// is a new map that is not read from the path
#[derive(Resource, Clone, Debug)]
pub struct MapToLoad(pub String, pub Option<NewMapOptions>);

/// Tile surface of a single JA2 tile type: the sprite sheet and the optional structure data
pub struct Ja2TileTypeDefinition {
    /// Tileset the files are loaded from, the default tileset if the map tileset has no file
    pub tileset_id: TilesetId,
    pub filename: String,
    pub sheet: Handle<StciSheet>,
    /// `None` if there is no JSD file, set once `MapAssets::structure_check` finished
    pub structure: Option<Handle<Ja2StructureFile>>,
    /// Terrain of the tile type in the map tileset
    pub terrain: TerrainType,
}

/// Handles to all assets of the loaded map, keeps them alive
#[derive(Resource)]
pub struct MapAssets {
    pub tilesets: Handle<Ja2Tilesets>,
    pub map: Handle<Ja2Map>,
    /// One entry per tile type, `None` if the type has no file
    pub tile_definitions: Vec<Option<Ja2TileTypeDefinition>>,
    /// Sheets of the surfaces of the tileset, indexed by `SurfaceId`
    pub surface_sheets: Vec<Handle<StciSheet>>,
    /// JSD paths (relative to the data root) and the host check which of them exist
    ///
    /// Most tile surfaces have no JSD. Loading only the existing ones avoids an error being logged
    /// for every missing one.
    pub structure_check: Option<(Vec<String>, Task<Result<Vec<bool>, String>>)>,
    /// Light templates of the map lights by file name
    pub light_templates: Vec<(String, Handle<LightTemplate>)>,
    /// The item files with the item graphics, see `ITEM_FILES`
    pub item_files: Vec<Handle<ItemFile>>,
    /// Animation surfaces of standing soldiers, indexed by `StandingSurfaceId`
    pub soldier_surfaces: Vec<Handle<StciSheet>>,
    pub soldier_palettes: Handle<PaletteReplacements>,
    /// Palette files of the body types by file name, see `SoldierBodyTypes::palette_files`
    pub monster_palettes: Vec<(&'static str, Handle<ColPalette>)>,
}

#[derive(Resource, Clone, Debug)]
pub(crate) struct LoadingError(pub String);

#[derive(Component)]
struct LoadingText;

pub struct LoadingPlugin;

impl Plugin for LoadingPlugin {
    fn build(&self, app: &mut App) {
        // State transitions are applied after `PreUpdate`, so the pollers never see a stale map
        app.add_systems(PreUpdate, switch_map)
            .add_systems(
                OnEnter(AppState::Loading),
                start_loading.after(SoldierSystems::Register),
            )
            .add_systems(OnEnter(LoadingPhase::Failed), show_loading_error)
            .add_systems(
                Update,
                (
                    poll_map_and_tilesets.run_if(in_state(LoadingPhase::MapAndTilesets)),
                    poll_tile_sheets.run_if(in_state(LoadingPhase::TileSheets)),
                )
                    // The map needs to be complete before the render systems see it
                    .before(MapSystems::Scene),
            );
    }
}

/// Replaces the current map when the host requested a new one
fn switch_map(
    mut commands: Commands,
    mut pixel_view: ResMut<PixelView>,
    mut next_app_state: ResMut<NextState<AppState>>,
    mut next_phase: ResMut<NextState<LoadingPhase>>,
) {
    let Some(request) = take_requested_map() else {
        return;
    };
    info!("Switching to map {}", request.path());

    commands.remove_resource::<MapWorld>();
    commands.remove_resource::<MovementCosts>();
    commands.remove_resource::<TacticalMap>();
    commands.remove_resource::<MapSoldiers>();
    commands.remove_resource::<ItemTable>();
    commands.remove_resource::<LightTemplates>();
    commands.remove_resource::<Tileset>();
    // An empty scene clears the map
    commands.insert_resource(ReplayScene::default());
    commands.insert_resource(ReplayAnimation::default());
    commands.insert_resource(SceneSources::default());
    commands.insert_resource(PendingSceneUpdate::default());
    // Keep the zoom
    pixel_view.position = Vec2::ZERO;

    commands.insert_resource(match request {
        MapRequest::File(path) => MapToLoad(path, None),
        MapRequest::New { path, options } => MapToLoad(path, Some(options)),
    });
    // `set` also restarts loading when we are already loading another map
    next_app_state.set(AppState::Loading);
    next_phase.set(LoadingPhase::MapAndTilesets);
}

fn start_loading(
    mut commands: Commands,
    time: Res<Time<Real>>,
    asset_server: Res<AssetServer>,
    mut maps: ResMut<Assets<Ja2Map>>,
    map_to_load: Res<MapToLoad>,
    font: Res<UiFont>,
    standing_surfaces: Res<StandingSurfaces>,
    body_types: Res<SoldierBodyTypes>,
) {
    info!("Start loading map {}", map_to_load.0);
    report_status(MapStatus::Loading, &map_to_load.0, None);
    let map = match map_to_load.1 {
        // An added asset is loaded right away, the tileset is checked with the tile definitions
        Some(options) => maps.add(new_ja2_map(
            TilesetId::new(options.tileset),
            options.smoothing,
            &mut Random::new(time.elapsed().as_nanos() as u64),
        )),
        None => asset_server.load(format!("{JA2_ASSET_SOURCE}://{}", map_to_load.0)),
    };
    commands.insert_resource(MapAssets {
        tilesets: asset_server.load(format!("{JA2_ASSET_SOURCE}://binarydata/ja2set.dat")),
        map,
        tile_definitions: vec![],
        surface_sheets: vec![],
        structure_check: None,
        light_templates: vec![],
        item_files: ITEM_FILES
            .iter()
            .map(|file| asset_server.load(format!("{JA2_ASSET_SOURCE}://{}", file)))
            .collect(),
        soldier_surfaces: standing_surfaces
            .iter()
            .map(|surface| asset_server.load(format!("{JA2_ASSET_SOURCE}://{}", surface.file)))
            .collect(),
        soldier_palettes: asset_server.load(format!("{JA2_ASSET_SOURCE}://{SOLDIER_PALETTE_FILE}")),
        monster_palettes: body_types
            .palette_files()
            .into_iter()
            .map(|file| {
                (
                    file,
                    asset_server.load(format!("{JA2_ASSET_SOURCE}://{}", file)),
                )
            })
            .collect(),
    });

    commands.spawn((
        Node {
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        DespawnOnExit(AppState::Loading),
        children![(
            Text::new("Loading..."),
            font.text_font(30.),
            TextColor(Color::WHITE),
            LoadingText,
        )],
    ));
}

fn load_error_message(what: &str, state: &LoadState) -> Option<String> {
    match state {
        LoadState::Failed(error) => Some(format!("Error loading {}: {}", what, error)),
        _ => None,
    }
}

fn poll_map_and_tilesets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut map_assets: ResMut<MapAssets>,
    map_to_load: Res<MapToLoad>,
    maps: Res<Assets<Ja2Map>>,
    tilesets: Res<Assets<Ja2Tilesets>>,
    mut next_phase: ResMut<NextState<LoadingPhase>>,
) {
    let map_state = asset_server.load_state(&map_assets.map);
    let tilesets_state = asset_server.load_state(&map_assets.tilesets);
    let error = load_error_message(&map_to_load.0, &map_state)
        .or_else(|| load_error_message("tilesets", &tilesets_state));
    if let Some(error) = error {
        commands.insert_resource(LoadingError(error));
        next_phase.set(LoadingPhase::Failed);
        return;
    }
    let (Some(map), Some(tilesets)) = (
        maps.get(&map_assets.map),
        tilesets.get(&map_assets.tilesets),
    ) else {
        return;
    };
    info!("Loaded map and tilesets");

    match load_tile_definitions(map, tilesets, &asset_server) {
        Ok(tile_definitions) => {
            let structure_paths: Vec<String> = tile_definitions
                .iter()
                .flatten()
                .map(structure_path)
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let task = IoTaskPool::get().spawn(host::files_exist(structure_paths.clone()));
            map_assets.structure_check = Some((structure_paths, task));
            map_assets.tile_definitions = tile_definitions;
            map_assets.light_templates = load_light_templates(map, &asset_server);
            next_phase.set(LoadingPhase::TileSheets);
        }
        Err(error) => {
            commands.insert_resource(LoadingError(format!(
                "Error loading tile definitions: {}",
                error
            )));
            next_phase.set(LoadingPhase::Failed);
        }
    }
}

/// Replaces the extension of a file name, or appends one (`FileMan::replaceExtension`)
fn replace_extension(filename: &str, extension: &str) -> String {
    let stem = match filename.rfind('.') {
        Some(dot) if !filename[dot..].contains(['/', '\\']) => &filename[..dot],
        _ => filename,
    };
    format!("{}.{}", stem, extension)
}

/// Path of the JSD file of a tile surface, relative to the data root
fn structure_path(tile_def: &Ja2TileTypeDefinition) -> String {
    format!(
        "tilesets/{}/{}",
        tile_def.tileset_id,
        replace_extension(&tile_def.filename, "jsd")
    )
}

/// Starts loading the tile surfaces of all tile types, like `LoadTileSurfaces` in the game
fn load_tile_definitions(
    map: &Ja2Map,
    tilesets: &Ja2Tilesets,
    asset_server: &AssetServer,
) -> Result<Vec<Option<Ja2TileTypeDefinition>>, String> {
    let tileset = u8::try_from(map.tileset_id)
        .ok()
        .map(TilesetId::new)
        .and_then(|id| Some((id, tilesets.get(id.index())?)));
    let Some((tileset_id, tileset)) = tileset else {
        return Err(format!(
            "The map uses tileset {}, which does not exist. There are {} tilesets.",
            map.tileset_id,
            tilesets.tilesets.len()
        ));
    };

    let mut tile_definitions = Vec::with_capacity(NUMBER_OF_TILE_TYPES);
    for (tile_type, type_name) in TILE_TYPE_NAMES.iter().enumerate() {
        let Some(resolved) = tilesets.resolve_tile_surface(tileset_id, tile_type) else {
            warn!(
                "Tileset {} and the default tileset have no file for tile type {} ({})",
                tileset_id, tile_type, type_name
            );
            tile_definitions.push(None);
            continue;
        };
        // Tile types that share a file also share the asset handles
        tile_definitions.push(Some(Ja2TileTypeDefinition {
            tileset_id: resolved.tileset_id,
            sheet: asset_server.load(format!(
                "{JA2_ASSET_SOURCE}://tilesets/{}/{}",
                resolved.tileset_id, resolved.filename
            )),
            // Loaded once it is known whether the JSD exists
            structure: None,
            filename: resolved.filename,
            terrain: tileset.terrain_id(tile_type),
        }));
    }
    Ok(tile_definitions)
}

/// Starts loading the templates of the map lights and the ones the editor places, which are in
/// the data root like in the game
fn load_light_templates(
    map: &Ja2Map,
    asset_server: &AssetServer,
) -> Vec<(String, Handle<LightTemplate>)> {
    let names: BTreeSet<String> = map
        .lights
        .iter()
        .flat_map(|lights| &lights.sprites)
        .map(|sprite| sprite.template_name.to_lowercase())
        .chain(LIGHT_RADII.map(|radius| light_template_name(radius).to_lowercase()))
        .collect();
    names
        .into_iter()
        .map(|name| {
            let handle = asset_server.load(format!("{JA2_ASSET_SOURCE}://{}", name));
            (name, handle)
        })
        .collect()
}

fn poll_tile_sheets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut map_assets: ResMut<MapAssets>,
    map_to_load: Res<MapToLoad>,
    maps: Res<Assets<Ja2Map>>,
    sheets: Res<Assets<StciSheet>>,
    structures: Res<Assets<Ja2StructureFile>>,
    light_templates: Res<Assets<LightTemplate>>,
    item_files: Res<Assets<ItemFile>>,
    replacements: Res<Assets<PaletteReplacements>>,
    col_palettes: Res<Assets<ColPalette>>,
    registries: SoldierRegistries,
    mut loading_text: Single<&mut Text, With<LoadingText>>,
    mut next_phase: ResMut<NextState<LoadingPhase>>,
    mut next_app_state: ResMut<NextState<AppState>>,
) {
    let structures_checked = match map_assets.structure_check.as_mut() {
        None => true,
        Some((_, task)) => match block_on(future::poll_once(task)) {
            None => false,
            Some(Err(error)) => {
                commands.insert_resource(LoadingError(format!(
                    "Error checking for structure data: {}",
                    error
                )));
                next_phase.set(LoadingPhase::Failed);
                return;
            }
            Some(Ok(exists)) => {
                let (paths, _) = map_assets
                    .structure_check
                    .take()
                    .expect("Expected structure check to exist.");
                let existing: BTreeSet<String> = paths
                    .into_iter()
                    .zip(exists)
                    .filter_map(|(path, exists)| exists.then_some(path))
                    .collect();
                for tile_def in map_assets.tile_definitions.iter_mut().flatten() {
                    let path = structure_path(tile_def);
                    if existing.contains(&path) {
                        tile_def.structure =
                            Some(asset_server.load(format!("{JA2_ASSET_SOURCE}://{}", path)));
                    }
                }
                true
            }
        },
    };

    let mut loaded = 0;
    let mut total = 0;
    for tile_def in map_assets.tile_definitions.iter().flatten() {
        total += 1;
        let path = format!("tilesets/{}/{}", tile_def.tileset_id, tile_def.filename);
        let sheet_state = asset_server.recursive_dependency_load_state(&tile_def.sheet);
        if sheet_state.is_failed() {
            let error = load_error_message(&path, &asset_server.load_state(&tile_def.sheet))
                .unwrap_or_else(|| format!("Error loading dependencies of {}", path));
            commands.insert_resource(LoadingError(error));
            next_phase.set(LoadingPhase::Failed);
            return;
        }
        // Most tile surfaces have no structure file
        let structure_state = match &tile_def.structure {
            Some(structure) => asset_server.load_state(structure),
            None if structures_checked => LoadState::Loaded,
            None => LoadState::NotLoaded,
        };
        let structure_loaded = match structure_state {
            LoadState::Loaded => true,
            state @ LoadState::Failed(_) => {
                let error = load_error_message(&format!("structure data of {}", path), &state)
                    .unwrap_or_default();
                commands.insert_resource(LoadingError(error));
                next_phase.set(LoadingPhase::Failed);
                return;
            }
            LoadState::NotLoaded | LoadState::Loading => false,
        };
        if sheet_state.is_loaded() && structure_loaded {
            loaded += 1;
        }
    }
    // Without these files items or soldiers can't be shown, which is not an error
    let definitions = registries.definitions();
    let optional_files = ITEM_FILES
        .iter()
        .copied()
        .zip(map_assets.item_files.iter().map(|h| h.id().untyped()))
        .chain(
            definitions
                .surfaces
                .iter()
                .map(|surface| surface.file)
                .zip(map_assets.soldier_surfaces.iter().map(|h| h.id().untyped())),
        )
        .chain(
            map_assets
                .monster_palettes
                .iter()
                .map(|(file, h)| (*file, h.id().untyped())),
        )
        .chain([(
            SOLDIER_PALETTE_FILE,
            map_assets.soldier_palettes.id().untyped(),
        )]);
    for (name, file) in optional_files {
        total += 1;
        match asset_server.load_state(file) {
            LoadState::Loaded => loaded += 1,
            LoadState::Failed(error) => {
                warn!("Error loading {}: {}", name, error);
                loaded += 1;
            }
            LoadState::NotLoaded | LoadState::Loading => {}
        }
    }
    for (name, template) in &map_assets.light_templates {
        total += 1;
        match asset_server.load_state(template) {
            LoadState::Loaded => loaded += 1,
            // The game ignores lights whose template can't be loaded
            LoadState::Failed(error) => {
                warn!("Error loading light template {}: {}", name, error);
                loaded += 1;
            }
            LoadState::NotLoaded | LoadState::Loading => {}
        }
    }
    if loaded < total {
        let text = format!("Loading tiles... {}/{}", loaded, total);
        if loading_text.0 != text {
            loading_text.0 = text;
        }
        return;
    }
    info!("Loaded {} tile surfaces", total);

    let surfaces = map_assets
        .tile_definitions
        .iter()
        .map(|tile_def| {
            let tile_def = tile_def.as_ref()?;
            let sheet = sheets
                .get(&tile_def.sheet)
                .expect("Expected sprite sheet to be loaded.");
            Some(TileSurfaceInfo {
                tileset_id: tile_def.tileset_id,
                filename: tile_def.filename.clone(),
                sub_images: sheet.database_info.clone(),
                structure: tile_def
                    .structure
                    .as_ref()
                    .and_then(|structure| structures.get(structure))
                    .cloned(),
                terrain: tile_def.terrain,
            })
        })
        .collect();
    let tileset = match Ja2TileDatabase::build(surfaces) {
        Ok(database) => Tileset::from_database(database),
        Err(error) => {
            commands.insert_resource(LoadingError(format!(
                "Error creating the tile database: {}",
                error
            )));
            next_phase.set(LoadingPhase::Failed);
            return;
        }
    };
    // The tileset has a surface for every tile type that could be loaded
    let mut surface_sheets = vec![Handle::default(); tileset.surfaces().len()];
    for (tile_type, definition) in map_assets.tile_definitions.iter().enumerate() {
        if let (Some(surface), Some(definition)) = (tileset.surface_of_type(tile_type), definition)
        {
            surface_sheets[surface.id.index()] = definition.sheet.clone();
        }
    }
    map_assets.surface_sheets = surface_sheets;

    let item_table = ItemTable::new(
        map_assets
            .item_files
            .iter()
            .filter_map(|file| item_files.get(file)),
    );
    let templates: HashMap<String, Arc<LightTemplate>> = map_assets
        .light_templates
        .iter()
        .filter_map(|(name, handle)| {
            Some((name.clone(), Arc::new(light_templates.get(handle)?.clone())))
        })
        .collect();
    let palettes = replacements.get(&map_assets.soldier_palettes);
    let ja2_map = maps
        .get(&map_assets.map)
        .expect("Expected map to be loaded.");
    let context = MapContext {
        tileset: &tileset,
        items: &item_table,
        soldiers: definitions,
        palettes,
        light_templates: &templates,
    };
    let map = match TacticalMap::from_ja2(ja2_map, &context) {
        Ok(map) => map,
        Err(error) => {
            commands.insert_resource(LoadingError(format!("Error converting the map: {}", error)));
            next_phase.set(LoadingPhase::Failed);
            return;
        }
    };

    let mut world = MapWorld::build(&map);
    add_items(&mut world, &map);
    let costs = MovementCosts::compile(&mut world, &map);
    let soldier_files = soldier_files(&map_assets, &sheets, &replacements, &col_palettes);
    let mut soldiers = MapSoldiers::default();
    add_soldiers(
        &mut world,
        &map.soldiers,
        |_| true,
        &soldier_files,
        &mut soldiers,
    );
    commands.insert_resource(world);
    commands.insert_resource(costs);
    commands.insert_resource(map);
    commands.insert_resource(soldiers);
    commands.insert_resource(item_table);
    commands.insert_resource(tileset);
    commands.insert_resource(LightTemplates(templates));
    commands.insert_resource(PendingSceneUpdate::full());
    info!("Done loading map: {}", map_to_load.0);
    report_status(MapStatus::Loaded, &map_to_load.0, None);
    next_app_state.set(AppState::Map);
}

/// The loaded files that soldiers need
pub fn soldier_files<'a>(
    map_assets: &MapAssets,
    sheets: &'a Assets<StciSheet>,
    replacements: &'a Assets<PaletteReplacements>,
    col_palettes: &'a Assets<ColPalette>,
) -> SoldierFiles<'a> {
    SoldierFiles {
        surfaces: map_assets
            .soldier_surfaces
            .iter()
            .map(|sheet| sheets.get(sheet))
            .collect(),
        replacements: replacements.get(&map_assets.soldier_palettes),
        monster_palettes: map_assets
            .monster_palettes
            .iter()
            .map(|(file, palette)| (*file, col_palettes.get(palette)))
            .collect(),
    }
}

fn show_loading_error(
    error: Res<LoadingError>,
    map_to_load: Res<MapToLoad>,
    mut loading_text: Single<&mut Text, With<LoadingText>>,
) {
    error!("{}", error.0);
    report_status(MapStatus::Error, &map_to_load.0, Some(&error.0));
    loading_text.0 = error.0.clone();
}
