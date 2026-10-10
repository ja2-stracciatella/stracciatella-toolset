//! Editing of the loaded map, ported from the original editor (`src/game/Editor/` of the game)
//!
//! The terrain mode (ground textures, roads and cliffs), the objects mode (debris, trees, rocks,
//! other junk and furniture), the buildings mode, the soldiers mode, the items mode, the lights
//! mode and the traversal mode exist so far. The host chooses the mode (`set_editor_mode`), the
//! mouse paints with it (`input.rs`) and the objects mode and the roads and cliffs tools preview
//! what a click does (`preview.rs`). The soldiers mode places and selects
//! soldiers instead (`soldiers.rs`), the host edits the selected one (`soldier_events.rs`), and
//! the items mode does the same for world items (`items.rs`, `item_events.rs`) and the lights mode
//! for lights and the lighting of the map (`lights.rs`, `light_events.rs`). The traversal mode
//! places and selects exit grids and entry points, and the edge points are generated again after the
//! edits that move them (`traversal.rs`, `traversal_events.rs`). The edits
//! change the [`TacticalMap`] through an [`EditContext`], which keeps the world in sync and
//! records undo, and the scene catches up with the touched tiles (`PendingSceneUpdate`).

use bevy::{ecs::system::SystemParam, prelude::*};

use crate::{
    map::{EditorSmoothing, TacticalMap},
    render::{world::MapWorld, PendingSceneUpdate},
    states::AppState,
    systems::MapSystems,
    tileset::Tileset,
};

use document::{EditContext, EditorDocument};

pub mod brush;
pub mod building;
pub mod building_mode;
pub mod caves;
pub mod cliffs;
pub mod commands;
pub mod document;
pub mod doors;
pub mod ground;
pub mod input;
pub mod item_events;
pub mod item_overlay;
pub mod items;
pub mod layout;
pub mod light_events;
pub mod light_overlay;
pub mod lights;
pub mod mode;
pub mod palette;
pub mod placement;
pub mod preview;
pub mod roads;
pub mod rooms_overlay;
pub mod save;
pub mod schedules;
pub mod scroll_area;
pub mod selection;
pub mod smart;
pub mod smooth;
pub mod soldier_events;
pub mod soldier_overlay;
pub mod soldiers;
pub mod traversal;
pub mod traversal_events;
pub mod traversal_overlay;
pub mod undo;
pub mod walls;

pub struct EditorPlugin;

impl Plugin for EditorPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<commands::CurrentMode>()
            .init_resource::<EditorDocument>()
            .init_resource::<input::EditorStroke>()
            .init_resource::<commands::ReportedEditorState>()
            .init_resource::<preview::PreviewSprites>()
            .init_resource::<rooms_overlay::RoomLabels>()
            .init_resource::<selection::BuildingSelection>()
            .init_resource::<soldiers::SoldierSelection>()
            .init_resource::<soldier_events::ReportedSoldier>()
            .init_resource::<soldier_events::ShownTeams>()
            .init_resource::<soldier_overlay::SoldierLabels>()
            .init_resource::<items::ItemSelection>()
            .init_resource::<item_events::ReportedItem>()
            .init_resource::<lights::LightSelection>()
            .init_resource::<light_events::ReportedLight>()
            .init_resource::<light_events::SyncedLights>()
            .init_resource::<traversal::TraversalSelection>()
            .init_resource::<traversal_events::ReportedTraversal>()
            .init_resource::<traversal_events::SyncedExitGrids>()
            .init_resource::<traversal_overlay::TraversalLabels>()
            .add_systems(PreUpdate, commands::apply_requested_mode)
            .add_systems(
                OnEnter(AppState::Map),
                (
                    commands::start_editing,
                    palette::report_palette,
                    preview::reset_preview,
                    rooms_overlay::reset_room_labels,
                    selection::reset_selection,
                    soldiers::reset_soldier_selection,
                    soldier_events::report_soldier_palette,
                    soldier_events::reset_shown_teams,
                    soldier_overlay::reset_soldier_labels,
                    items::reset_item_selection,
                    item_events::reset_reported_item,
                    lights::reset_light_selection,
                    light_events::reset_reported_light,
                    light_events::reset_synced_lights,
                    (
                        traversal::reset_traversal_selection,
                        traversal_events::reset_reported_traversal,
                        traversal_events::reset_synced_exit_grids,
                        traversal_overlay::reset_traversal_labels,
                    ),
                ),
            )
            .add_systems(
                Update,
                (
                    // Edits and the scene catching up with them
                    (
                        commands::run_editor_commands,
                        input::paint,
                        selection::select_buildings,
                        soldiers::edit_soldiers,
                        items::edit_items,
                        lights::edit_lights,
                        lights::update_light_preview,
                        traversal::edit_traversal,
                        soldier_events::sync_soldier_scene,
                        item_events::sync_item_scene,
                        light_events::sync_light_scene,
                        traversal_events::sync_traversal_scene,
                        scroll_area::sync_restricted_scroll,
                        // After the scene update of this frame compiled the movement costs
                        traversal_events::regenerate_edge_points,
                    )
                        .chain(),
                    // Overlays and reports
                    (
                        scroll_area::draw_scrollable_area,
                        input::draw_brush_cursor,
                        soldier_overlay::draw_soldier_markers,
                        item_overlay::draw_item_markers,
                        light_overlay::draw_light_markers,
                        traversal_overlay::draw_traversal_markers,
                        traversal_overlay::update_traversal_labels,
                        selection::draw_selection,
                        preview::update_preview,
                        rooms_overlay::update_room_labels,
                        soldier_overlay::update_soldier_labels,
                        doors::draw_door_markers,
                        doors::report_door_request,
                        soldier_events::report_soldier,
                        item_events::report_item,
                        light_events::report_light,
                        traversal_events::report_traversal,
                        commands::report_editor_state,
                    )
                        .chain(),
                )
                    .chain()
                    .in_set(MapSystems::Edit)
                    .run_if(in_state(AppState::Map)),
            )
            .add_systems(
                Update,
                save::save_requested_map.after(commands::run_editor_commands),
            );
    }
}

/// Everything that edits of the map change
#[derive(SystemParam)]
pub struct MapEdit<'w> {
    map: ResMut<'w, TacticalMap>,
    world: ResMut<'w, MapWorld>,
    tileset: Res<'w, Tileset>,
    document: ResMut<'w, EditorDocument>,
    pending: ResMut<'w, PendingSceneUpdate>,
}

impl MapEdit<'_> {
    /// Edits the map, the scene is updated with the touched tiles
    pub fn edit(&mut self, edit: impl FnOnce(&mut EditContext)) {
        let records = self.document.undo.records();
        let mut context = EditContext::new(
            &mut self.map,
            &mut self.world,
            &self.tileset,
            &mut self.document,
        );
        edit(&mut context);
        let touched = context.finish();
        if !touched.is_empty() || self.document.undo.records() != records {
            self.document.revision += 1;
        }
        self.pending.tiles.extend(touched);
    }

    pub fn document(&mut self) -> &mut EditorDocument {
        &mut self.document
    }

    pub fn tileset(&self) -> &Tileset {
        &self.tileset
    }

    /// How the editor smooths the map
    pub fn smoothing(&self) -> EditorSmoothing {
        self.map.info.editor_smoothing
    }
}

#[cfg(test)]
mod tests {
    /// Bevy checks that the parameters of a system don't access the same data mutably twice when
    /// it builds the schedules, which only happens in the app
    #[test]
    fn editor_systems_have_no_conflicting_access() {
        use bevy::{prelude::*, state::app::StatesPlugin};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, StatesPlugin, AssetPlugin::default()))
            .init_asset::<crate::file_formats::ja2_map::Ja2Map>()
            .init_state::<crate::states::AppState>()
            .add_plugins(super::EditorPlugin);
        // Builds the schedules, which checks the access of the systems
        app.update();
        app.update();
    }
}
