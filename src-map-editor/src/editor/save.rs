//! Saving the edited map for the host
//!
//! The map file that was loaded is still an asset. The edits are put into a copy of it, which is
//! written in the version it was loaded in, so everything the editor can't change stays as it was.
//! Edge points that are out of date are generated again first, like the original editor does
//! before it saves.

use bevy::prelude::*;

use crate::{
    file_formats::ja2_map::Ja2Map,
    host::{finish_saves, save_requested, SavedMap},
    map::TacticalMap,
    render::{movement_costs::MovementCosts, world::MapWorld, PendingSceneUpdate},
    states::{
        loading::{MapAssets, MapToLoad},
        AppState,
    },
};

use super::{document::EditorDocument, traversal_events::regenerate};

/// What generating the edge points needs
#[derive(bevy::ecs::system::SystemParam)]
pub struct EdgePointSources<'w> {
    world: Option<ResMut<'w, MapWorld>>,
    costs: Option<Res<'w, MovementCosts>>,
    pending: Option<Res<'w, PendingSceneUpdate>>,
}

/// Saves the map if the host asked for it, after the edits requested before were applied
pub fn save_requested_map(
    state: Res<State<AppState>>,
    map_to_load: Option<Res<MapToLoad>>,
    map: Option<ResMut<TacticalMap>>,
    mut document: ResMut<EditorDocument>,
    map_assets: Option<Res<MapAssets>>,
    mut maps: ResMut<Assets<Ja2Map>>,
    sources: EdgePointSources,
) {
    if !save_requested() {
        return;
    }
    let (Some(map_to_load), Some(mut map), Some(map_assets), AppState::Map) =
        (map_to_load, map, map_assets, state.get())
    else {
        finish_saves(Err("No map is loaded".to_owned()));
        return;
    };
    if document.edge_points_dirty {
        if let Some(mut world) = sources.world {
            // The movement costs only include the last edits once the scene caught up with them
            let caught_up = sources.pending.is_none_or(|pending| pending.is_empty());
            match sources.costs.filter(|_| caught_up) {
                Some(costs) => regenerate(&mut map, &world, &costs, &mut document, &map_to_load.0),
                None => {
                    let costs = MovementCosts::compile(&mut world, &map);
                    regenerate(&mut map, &world, &costs, &mut document, &map_to_load.0);
                }
            }
        }
    }
    let result = save_map(&map, &map_assets.map, &mut maps).map(|data| SavedMap {
        map: map_to_load.0.clone(),
        revision: document.revision,
        data,
    });
    if let Err(error) = &result {
        error!("Error saving {}: {}", map_to_load.0, error);
    }
    finish_saves(result);
}

/// Puts the map into its map file and writes it, the map file asset is replaced by the saved one
fn save_map(
    map: &TacticalMap,
    handle: &Handle<Ja2Map>,
    maps: &mut Assets<Ja2Map>,
) -> Result<Vec<u8>, String> {
    let mut raw = maps
        .get(handle)
        .cloned()
        .ok_or_else(|| "The map file is not loaded".to_owned())?;
    map.apply_to_ja2(&mut raw)?;
    let data = raw.to_bytes().map_err(|e| e.to_string())?;
    // Later saves and reopening the map start from what was saved
    maps.insert(handle, raw).map_err(|e| e.to_string())?;
    Ok(data)
}
