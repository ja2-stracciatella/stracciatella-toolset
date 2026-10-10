//! What the host is told about the exit grids, the entry points and the edge points, how it
//! edits them, and how the scene and the edge points catch up with the edits
//!
//! Whenever the exit grids, the entry points, the selection, the queried exit grid or the edge
//! points change, the host gets a `traversal` event. It edits the selected exit grid or entry
//! point with `set_traversal`.
//!
//! The edge points are generated again (`generate_edge_points`) once the movement costs caught
//! up with the edits that can move them, while the traversal mode is shown, and before the map
//! is saved. They are not part of undo, undo just generates them again.

use std::collections::BTreeSet;

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::{
    host::report_editor_event,
    map::{EdgePointSet, EntryPoint, ExitGrid, GridNo, SectorId, TacticalMap},
    render::{
        edge_points::generate_edge_points, movement_costs::MovementCosts, world::MapWorld,
        PendingSceneUpdate,
    },
    states::loading::MapToLoad,
};

use super::{
    commands::CurrentMode,
    document::{EditContext, EditorDocument},
    traversal::{exit_grid_at, traversal_mode, TraversalItem, TraversalSelection},
};

/// An exit grid as the host shows it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitGridValue {
    pub grid_no: u16,
    pub dest_sector: SectorId,
    pub dest_grid_no: u16,
}

impl ExitGridValue {
    pub fn of(grid: &ExitGrid) -> Self {
        Self {
            grid_no: grid.grid_no.index() as u16,
            dest_sector: grid.dest_sector,
            dest_grid_no: grid.dest_grid_no.index() as u16,
        }
    }
}

/// The grid numbers of the entry points, `None` for the ones that are not set
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct EntryPointsValue {
    pub north: Option<u16>,
    pub east: Option<u16>,
    pub south: Option<u16>,
    pub west: Option<u16>,
    pub center: Option<u16>,
    pub isolated: Option<u16>,
}

/// The number of edge points of each side
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct SideCounts {
    pub north: usize,
    pub east: usize,
    pub south: usize,
    pub west: usize,
}

impl SideCounts {
    pub fn of(set: &EdgePointSet) -> Self {
        let [north, east, south, west] = set.sides().map(|list| list.grid_nos.len());
        Self {
            north,
            east,
            south,
            west,
        }
    }
}

/// The edge points as the host shows them
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgePointSummary {
    pub primary: SideCounts,
    pub secondary: SideCounts,
    /// The sides with an entry point but without primary edge points, soldiers can't enter the
    /// map there
    pub missing: Vec<EntryPoint>,
    /// Whether they are being generated again, the counts are the ones before the last edits
    pub pending: bool,
}

/// The selected exit grid or entry point as the host shows it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SelectedTraversalValue {
    ExitGrid {
        #[serde(flatten)]
        grid: ExitGridValue,
    },
    Entry {
        point: EntryPoint,
        grid_no: u16,
    },
}

impl SelectedTraversalValue {
    /// The selected item, `None` if it is gone
    pub fn of(map: &TacticalMap, item: TraversalItem) -> Option<Self> {
        match item {
            TraversalItem::Exit(grid_no) => {
                exit_grid_at(map, grid_no).map(|grid| SelectedTraversalValue::ExitGrid {
                    grid: ExitGridValue::of(grid),
                })
            }
            TraversalItem::Entry(point) => {
                map.info
                    .entry(point)
                    .map(|grid_no| SelectedTraversalValue::Entry {
                        point,
                        grid_no: grid_no.index() as u16,
                    })
            }
        }
    }
}

/// The exit grids, the entry points and the edge points of the map
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "traversal", rename_all = "camelCase")]
pub struct TraversalEvent {
    pub map: String,
    /// The number of exit grids of the map
    pub exit_grids: usize,
    /// The exit grid or entry point the select tool selected
    pub selected: Option<SelectedTraversalValue>,
    /// The exit grid that was right clicked last, its values go into the mode
    pub queried: Option<ExitGridValue>,
    pub entry_points: EntryPointsValue,
    pub edge_points: EdgePointSummary,
}

impl TraversalEvent {
    pub fn of(
        map_name: &str,
        map: &TacticalMap,
        document: &EditorDocument,
        selection: &TraversalSelection,
    ) -> Self {
        let entry = |point| map.info.entry(point).map(|g| g.index() as u16);
        let primary = &map.edge_points.primary;
        let sides = [
            EntryPoint::North,
            EntryPoint::East,
            EntryPoint::South,
            EntryPoint::West,
        ];
        let missing = sides
            .into_iter()
            .zip(primary.sides())
            .filter(|(point, list)| map.info.entry(*point).is_some() && list.grid_nos.is_empty())
            .map(|(point, _)| point)
            .collect();
        Self {
            map: map_name.to_owned(),
            exit_grids: map.exit_grids.len(),
            selected: selection
                .selected
                .and_then(|item| SelectedTraversalValue::of(map, item)),
            queried: selection.queried.as_ref().map(ExitGridValue::of),
            entry_points: EntryPointsValue {
                north: entry(EntryPoint::North),
                east: entry(EntryPoint::East),
                south: entry(EntryPoint::South),
                west: entry(EntryPoint::West),
                center: entry(EntryPoint::Center),
                isolated: entry(EntryPoint::Isolated),
            },
            edge_points: EdgePointSummary {
                primary: SideCounts::of(primary),
                secondary: map
                    .edge_points
                    .secondary
                    .as_ref()
                    .map(SideCounts::of)
                    .unwrap_or_default(),
                missing,
                pending: document.edge_points_dirty,
            },
        }
    }
}

/// How the host edits the selection with `set_traversal`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TraversalEdit {
    /// Moves an entry point to a tile
    Entry { point: EntryPoint, grid_no: u16 },
    /// Changes the destination of the exit grid of a tile
    ExitGrid {
        grid_no: u16,
        dest_sector: SectorId,
        dest_grid_no: u16,
    },
}

/// Edits an entry point or an exit grid, grid numbers off the map are ignored
pub fn update_traversal(context: &mut EditContext, edit: &TraversalEdit) {
    match *edit {
        TraversalEdit::Entry { point, grid_no } => {
            if let Some(grid_no) = GridNo::new(usize::from(grid_no)) {
                context.set_entry_point(point, Some(grid_no));
            }
        }
        TraversalEdit::ExitGrid {
            grid_no,
            dest_sector,
            dest_grid_no,
        } => {
            if let Some(grid_no) = GridNo::new(usize::from(grid_no)) {
                context.change_exit_grid(grid_no, dest_sector, dest_grid_no);
            }
        }
    }
}

/// The traversal event that was reported last, `None` for a new map
#[derive(Resource, Clone, Debug, Default)]
pub struct ReportedTraversal(Option<TraversalEvent>);

pub fn reset_reported_traversal(mut reported: ResMut<ReportedTraversal>) {
    reported.0 = None;
}

/// Tells the host about the exit grids, the entry points and the edge points when they change
pub fn report_traversal(
    map_to_load: Res<MapToLoad>,
    map: Res<TacticalMap>,
    document: Res<EditorDocument>,
    selection: Res<TraversalSelection>,
    mut reported: ResMut<ReportedTraversal>,
) {
    if !map.is_changed()
        && !selection.is_changed()
        && !document.is_changed()
        && reported.0.is_some()
    {
        return;
    }
    let event = TraversalEvent::of(&map_to_load.0, &map, &document, &selection);
    if reported.0.as_ref() != Some(&event) {
        report_editor_event(&event);
        reported.0 = Some(event);
    }
}

/// The exit grids that the movement costs were last compiled with
#[derive(Resource, Clone, Debug, Default)]
pub struct SyncedExitGrids(Vec<ExitGrid>);

pub fn reset_synced_exit_grids(map: Res<TacticalMap>, mut synced: ResMut<SyncedExitGrids>) {
    synced.0 = map.exit_grids.clone();
}

/// The tiles whose exit grids were added, removed or changed
pub fn changed_exit_grid_tiles(old: &[ExitGrid], new: &[ExitGrid]) -> BTreeSet<GridNo> {
    old.iter()
        .filter(|grid| !new.contains(grid))
        .chain(new.iter().filter(|grid| !old.contains(grid)))
        .map(|grid| grid.grid_no)
        .collect()
}

/// Compiles the movement costs of the tiles whose exit grids changed again, with the scene
/// update (`TRAVELCOST_EXITGRID`)
pub fn sync_traversal_scene(
    mut document: ResMut<EditorDocument>,
    map: Res<TacticalMap>,
    mut synced: ResMut<SyncedExitGrids>,
    mut pending: ResMut<PendingSceneUpdate>,
) {
    if !document.traversal_dirty {
        return;
    }
    document.traversal_dirty = false;
    pending
        .tiles
        .extend(changed_exit_grid_tiles(&synced.0, &map.exit_grids));
    synced.0 = map.exit_grids.clone();
}

/// Generates the edge points of the map again and marks them to be saved
pub fn regenerate(
    map: &mut TacticalMap,
    world: &MapWorld,
    costs: &MovementCosts,
    document: &mut EditorDocument,
    map_path: &str,
) {
    map.edge_points = generate_edge_points(world, costs, map, SectorId::from_map_path(map_path));
    map.edge_points_stale = true;
    document.edge_points_dirty = false;
}

/// Generates the edge points again while the traversal mode is shown, once the movement costs
/// caught up with the edits
pub fn regenerate_edge_points(
    mode: Res<CurrentMode>,
    pending: Res<PendingSceneUpdate>,
    world: Res<MapWorld>,
    costs: Res<MovementCosts>,
    map_to_load: Res<MapToLoad>,
    mut map: ResMut<TacticalMap>,
    mut document: ResMut<EditorDocument>,
) {
    if !document.edge_points_dirty || traversal_mode(&mode).is_none() || !pending.is_empty() {
        return;
    }
    regenerate(&mut map, &world, &costs, &mut document, &map_to_load.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        document::tests::{grid, Fixture},
        traversal::tests::{edit, mode},
    };

    #[test]
    fn edits_are_read_from_json() {
        let parse = |json: &str| serde_json::from_str::<TraversalEdit>(json).unwrap();
        assert_eq!(
            parse(r#"{"kind":"entry","point":"center","gridNo":12880}"#),
            TraversalEdit::Entry {
                point: EntryPoint::Center,
                grid_no: 12880
            }
        );
        assert_eq!(
            parse(
                r#"{"kind":"exitGrid","gridNo":5,"destSector":{"x":9,"y":1,"z":1},"destGridNo":7}"#
            ),
            TraversalEdit::ExitGrid {
                grid_no: 5,
                dest_sector: SectorId { x: 9, y: 1, z: 1 },
                dest_grid_no: 7
            }
        );
    }

    #[test]
    fn the_host_edits_entry_points_and_exit_grids() {
        let mut fixture = Fixture::new();
        let entry = |grid_no| TraversalEdit::Entry {
            point: EntryPoint::Center,
            grid_no,
        };
        edit(&mut fixture, |context| {
            update_traversal(context, &entry(12880))
        });
        assert_eq!(fixture.map.info.center, GridNo::new(12880));
        // Grid numbers off the map are ignored
        edit(&mut fixture, |context| {
            update_traversal(context, &entry(30000))
        });
        assert_eq!(fixture.map.info.center, GridNo::new(12880));

        let paint = mode(r#"{"mode":"traversal","tool":"place"}"#);
        let area = crate::editor::brush::Brush::default().region(None, grid(40, 40).position());
        edit(&mut fixture, |context| paint.apply(context, &area));
        let change = TraversalEdit::ExitGrid {
            grid_no: grid(40, 40).index() as u16,
            dest_sector: SectorId { x: 3, y: 4, z: 2 },
            dest_grid_no: 99,
        };
        edit(&mut fixture, |context| update_traversal(context, &change));
        let changed = fixture.map.exit_grids[0];
        assert_eq!(changed.dest_sector, SectorId { x: 3, y: 4, z: 2 });
        assert_eq!(changed.dest_grid_no.index(), 99);
    }

    #[test]
    fn events_have_the_exit_grids_and_the_edge_points() {
        let mut fixture = Fixture::new();
        let paint = mode(
            r#"{"mode":"traversal","tool":"place","destSector":{"x":11,"y":10,"z":1},"destGridNo":7}"#,
        );
        let area = crate::editor::brush::Brush::default().region(None, grid(40, 40).position());
        edit(&mut fixture, |context| paint.apply(context, &area));
        fixture.map.info.north_entry = Some(grid(80, 80));
        fixture.map.info.isolated = Some(grid(81, 80));
        fixture.map.edge_points.primary.west.grid_nos = vec![grid(10, 90)];
        let mut selection = TraversalSelection::default();
        selection.queried = fixture.map.exit_grids.first().copied();
        selection.selected = Some(TraversalItem::Entry(EntryPoint::Isolated));
        let event = TraversalEvent::of("maps/a9.dat", &fixture.map, &fixture.document, &selection);
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "traversal");
        assert_eq!(json["exitGrids"], 1);
        assert_eq!(
            json["queried"],
            serde_json::json!({
                "gridNo": 40 * 160 + 40,
                "destSector": { "x": 11, "y": 10, "z": 1 },
                "destGridNo": 7
            })
        );
        assert_eq!(
            json["selected"],
            serde_json::json!({ "kind": "entry", "point": "isolated", "gridNo": 80 * 160 + 81 })
        );
        let exit = SelectedTraversalValue::of(&fixture.map, TraversalItem::Exit(grid(40, 40)));
        assert_eq!(
            serde_json::to_value(exit).unwrap(),
            serde_json::json!({
                "kind": "exitGrid",
                "gridNo": 40 * 160 + 40,
                "destSector": { "x": 11, "y": 10, "z": 1 },
                "destGridNo": 7
            })
        );
        assert_eq!(
            SelectedTraversalValue::of(&fixture.map, TraversalItem::Exit(grid(1, 1))),
            None
        );
        assert_eq!(json["entryPoints"]["north"], 80 * 160 + 80);
        assert_eq!(json["entryPoints"]["east"], serde_json::Value::Null);
        assert_eq!(json["edgePoints"]["primary"]["west"], 1);
        assert_eq!(json["edgePoints"]["missing"], serde_json::json!(["north"]));
        assert_eq!(json["edgePoints"]["pending"], true);
    }

    #[test]
    fn changed_exit_grids_compile_their_tiles_again() {
        let mut fixture = Fixture::new();
        let old = fixture.map.exit_grids.clone();
        let paint =
            mode(r#"{"mode":"traversal","tool":"place","brush":{"kind":"square","size":3}}"#);
        let area =
            crate::editor::brush::Brush::Square { size: 3 }.region(None, grid(40, 40).position());
        edit(&mut fixture, |context| paint.apply(context, &area));
        let tiles = changed_exit_grid_tiles(&old, &fixture.map.exit_grids);
        assert_eq!(tiles.len(), 9);
        assert!(
            changed_exit_grid_tiles(&fixture.map.exit_grids, &fixture.map.exit_grids).is_empty()
        );
    }

    #[test]
    fn regenerating_marks_the_edge_points_to_be_saved() {
        let mut fixture = Fixture::new();
        fixture.map.info.north_entry = Some(grid(80, 80));
        fixture.document.edge_points_dirty = true;
        let costs = MovementCosts::compile(&mut fixture.world, &fixture.map);
        regenerate(
            &mut fixture.map,
            &fixture.world,
            &costs,
            &mut fixture.document,
            "maps/a9.dat",
        );
        assert!(fixture.map.edge_points_stale && !fixture.document.edge_points_dirty);
        assert!(!fixture.map.edge_points.primary.north.grid_nos.is_empty());
        assert!(fixture.map.edge_points.primary.east.grid_nos.is_empty());
    }
}
