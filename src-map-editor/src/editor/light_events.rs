//! What the host is told about the lights and how it edits them, and how the scene catches up
//! with them
//!
//! Whenever the selected light, the number of lights, the light colour or the ambient light
//! change, the host gets a `light` event. The host sends the radius and the time of the selected
//! light with `set_light` and everything else with `light_command`. Failed edits are reported
//! with a `notice`.

use std::collections::BTreeSet;

use bevy::{ecs::system::SystemParam, prelude::*};
use serde::{Deserialize, Serialize};

use crate::{
    host::report_editor_event,
    light::LightTemplates,
    map::{AmbientLight, GridNo, LightId, LightTime, MapLight, TacticalMap},
    render::{
        lighting::{light_tiles, scene_lights, PreviewLight},
        PendingSceneUpdate,
    },
    states::loading::MapToLoad,
    systems::bake::MapBake,
};

use super::{
    document::{EditContext, EditorDocument},
    lights::{editor_light_level, map_light, LightSelection},
    soldier_events::NoticeEvent,
    soldiers::edit_once,
    MapEdit,
};

/// A light as the host shows it
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LightValue {
    pub id: u32,
    pub grid_no: u16,
    pub x: u8,
    pub y: u8,
    /// The radius of the editor's templates, `None` for other templates
    pub radius: Option<u8>,
    /// File name of the template
    pub template: String,
    pub time: LightTime,
}

impl LightValue {
    pub fn of(light: &MapLight) -> Self {
        Self {
            id: light.id.0,
            grid_no: GridNo::from(light.position).index() as u16,
            x: light.position.x,
            y: light.position.y,
            radius: light.radius(),
            template: light.template_name.clone(),
            time: light.time,
        }
    }
}

/// The ambient light of basements and caves as the editor shows it
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct AmbientValue {
    pub basement: bool,
    pub caves: bool,
    /// The light level of the original editor, 1 to 15, higher is brighter
    pub level: u8,
}

impl AmbientValue {
    pub fn of(ambient: &AmbientLight) -> Self {
        Self {
            basement: ambient.basement,
            caves: ambient.caves,
            level: editor_light_level(ambient.level),
        }
    }
}

/// The selected light, `null` if none is, and the lighting of the map
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "type", rename = "light", rename_all = "camelCase")]
pub struct LightEvent {
    pub map: String,
    pub light: Option<LightValue>,
    /// The number of lights of the map
    pub count: usize,
    /// The light colour as red, green and blue
    pub color: [u8; 3],
    /// `None` for maps that are neither basements nor caves
    pub ambient: Option<AmbientValue>,
    /// The radii whose templates are loaded, only these can be placed
    pub radii: Vec<u8>,
}

/// The values of the selected light sent with `set_light`
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LightEdit {
    pub id: u32,
    /// `None` keeps the template
    pub radius: Option<u8>,
    pub time: LightTime,
}

/// What the host asks of the lights besides editing the selected one, the selected light is
/// removed with the Delete key
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum LightCommand {
    Deselect,
    /// Sets the light colour and the light level of basements and caves (1 to 15), `None`
    /// keeps the level
    Settings {
        color: [u8; 3],
        ambient_level: Option<u8>,
    },
}

/// What light commands need besides the map
#[derive(SystemParam)]
pub struct LightCommandParams<'w> {
    pub selection: ResMut<'w, LightSelection>,
    pub templates: Option<Res<'w, LightTemplates>>,
}

/// Edits the selected light with the values of the host, ignored if the light is gone
pub fn update_light(
    context: &mut EditContext,
    templates: &LightTemplates,
    edit: &LightEdit,
) -> Result<(), String> {
    context
        .change_light(templates, LightId(edit.id), edit.radius, edit.time)
        .map(|_| ())
}

/// Runs a light command of the host
pub fn run_light_command(
    command: LightCommand,
    edit: &mut MapEdit,
    params: &mut LightCommandParams,
) {
    let selection = &mut *params.selection;
    match command {
        LightCommand::Deselect => selection.clear(),
        LightCommand::Settings {
            color,
            ambient_level,
        } => edit_once(edit, |context| {
            context.change_light_settings(color, ambient_level);
        }),
    }
}

/// The light event that was reported last, `None` for a new map
#[derive(Resource, Clone, Debug, Default)]
pub struct ReportedLight(Option<LightEvent>);

pub fn reset_reported_light(mut reported: ResMut<ReportedLight>) {
    reported.0 = None;
}

/// Tells the host about the selected light and the lighting when they change, and about failed
/// edits
pub fn report_light(
    map_to_load: Res<MapToLoad>,
    map: Res<TacticalMap>,
    templates: Option<Res<LightTemplates>>,
    mut selection: ResMut<LightSelection>,
    mut reported: ResMut<ReportedLight>,
) {
    if let Some(message) = selection.notice.take() {
        report_editor_event(&NoticeEvent {
            map: map_to_load.0.clone(),
            message,
        });
    }
    if !map.is_changed() && !selection.is_changed() && reported.0.is_some() {
        return;
    }
    let event = LightEvent {
        map: map_to_load.0.clone(),
        light: selection
            .selected
            .and_then(|id| map_light(&map, id))
            .map(LightValue::of),
        count: map.lights.len(),
        color: map.light_color,
        ambient: map.ambient_light.as_ref().map(AmbientValue::of),
        radii: templates.map(|t| t.radii()).unwrap_or_default(),
    };
    if reported.0.as_ref() != Some(&event) {
        report_editor_event(&event);
        reported.0 = Some(event);
    }
}

/// The lights, the light colour and the ambient light that the scene was last synced with
#[derive(Resource, Clone, Debug, Default)]
pub struct SyncedLights {
    /// The scene lights, with the preview light (`scene_lights`)
    lights: Vec<MapLight>,
    color: [u8; 3],
    ambient: Option<AmbientLight>,
    /// The preview light changed since the last sync
    preview_dirty: bool,
}

impl SyncedLights {
    fn of(map: &TacticalMap, preview: &PreviewLight) -> Self {
        Self {
            lights: scene_lights(map, preview.0.as_ref()).cloned().collect(),
            color: map.light_color,
            ambient: map.ambient_light,
            preview_dirty: false,
        }
    }

    /// Syncs with the lights of the map and the preview light, queueing what the scene has to
    /// update
    fn sync(
        &mut self,
        map: &TacticalMap,
        preview: &PreviewLight,
        pending: &mut PendingSceneUpdate,
    ) {
        let new = Self::of(map, preview);
        if self.color != new.color || self.ambient != new.ambient {
            pending.full = true;
        } else {
            pending
                .tiles
                .extend(changed_light_tiles(&self.lights, &new.lights));
        }
        *self = new;
    }
}

pub fn reset_synced_lights(
    map: Res<TacticalMap>,
    mut preview: ResMut<PreviewLight>,
    mut synced: ResMut<SyncedLights>,
) {
    *preview = PreviewLight::default();
    *synced = SyncedLights::of(&map, &preview);
}

/// Whether two lights light the same tiles in the same way, whatever their ids
fn same_lighting(a: &MapLight, b: &MapLight) -> bool {
    a.position == b.position && a.time == b.time && a.template_name == b.template_name
}

/// The tiles to relight after the lights changed from `old` to `new`: the tiles of the lights
/// that were removed, added or changed
pub fn changed_light_tiles(old: &[MapLight], new: &[MapLight]) -> BTreeSet<GridNo> {
    let missing = |light: &MapLight, lights: &[MapLight]| {
        !lights.iter().any(|other| same_lighting(light, other))
    };
    old.iter()
        .filter(|light| missing(light, new))
        .chain(new.iter().filter(|light| missing(light, old)))
        .flat_map(light_tiles)
        .collect()
}

/// Brings the scene up to date after the lights or the preview light changed: the tiles of the
/// changed lights are lit again, a new light colour or ambient light rebuilds everything,
/// because the shaded palettes and the base light of every tile depend on them
///
/// A change of only the preview waits until the scene is idle, so that moving the cursor
/// quickly only lights the latest position.
pub fn sync_light_scene(
    mut document: ResMut<EditorDocument>,
    map: Res<TacticalMap>,
    preview: Res<PreviewLight>,
    bake: Res<MapBake>,
    mut synced: ResMut<SyncedLights>,
    mut pending: ResMut<PendingSceneUpdate>,
) {
    if preview.is_changed() {
        synced.preview_dirty = true;
    }
    let preview_due = synced.preview_dirty && pending.is_empty() && bake.is_idle();
    if !document.lights_dirty && !preview_due {
        return;
    }
    document.lights_dirty = false;
    synced.sync(&map, &preview, &mut pending);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        document::tests::grid,
        lights::{
            new_light,
            tests::{mode, LightFixture},
            LightMode,
        },
    };

    #[test]
    fn commands_are_read_from_json() {
        let parse = |json: &str| serde_json::from_str::<LightCommand>(json).unwrap();
        assert_eq!(parse(r#"{"kind":"deselect"}"#), LightCommand::Deselect);
        assert_eq!(
            parse(r#"{"kind":"settings","color":[1,2,3],"ambientLevel":7}"#),
            LightCommand::Settings {
                color: [1, 2, 3],
                ambient_level: Some(7)
            }
        );
        assert_eq!(
            parse(r#"{"kind":"settings","color":[1,2,3],"ambientLevel":null}"#),
            LightCommand::Settings {
                color: [1, 2, 3],
                ambient_level: None
            }
        );
    }

    #[test]
    fn the_host_edits_the_selected_light() {
        let mut fixture = LightFixture::new();
        let id = fixture
            .place(&mode(2, LightTime::PrimeTime), grid(30, 31))
            .unwrap();
        let value = LightValue::of(map_light(fixture.map(), id).unwrap());
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json["gridNo"], 31 * 160 + 30);
        assert_eq!(
            (json["x"].clone(), json["y"].clone()),
            (30.into(), 31.into())
        );
        assert_eq!(json["radius"], 2);
        assert_eq!(json["template"], "L-R02.LHT");
        assert_eq!(json["time"], "primeTime");

        let edit: LightEdit =
            serde_json::from_str(&format!(r#"{{"id":{},"radius":3,"time":"night"}}"#, id.0))
                .unwrap();
        fixture
            .edit(|context, templates| update_light(context, templates, &edit))
            .unwrap();
        let light = map_light(fixture.map(), id).unwrap();
        assert_eq!((light.radius(), light.time), (Some(3), LightTime::Night));
        // Missing templates fail, other lights are ignored
        let missing = LightEdit {
            radius: Some(8),
            ..edit
        };
        assert!(fixture
            .edit(|context, templates| update_light(context, templates, &missing))
            .is_err());
        let other = LightEdit { id: 99, ..edit };
        fixture
            .edit(|context, templates| update_light(context, templates, &other))
            .unwrap();
        assert_eq!(fixture.fixture.document.undo.depth(), 2);
    }

    #[test]
    fn events_have_the_lighting_of_the_map() {
        let event = LightEvent {
            map: "maps/a9_b1.dat".to_owned(),
            light: None,
            count: 3,
            color: [10, 20, 30],
            ambient: Some(AmbientValue::of(&AmbientLight {
                basement: true,
                caves: false,
                level: 12,
            })),
            radii: vec![1, 2, 3],
        };
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json["type"], "light");
        assert_eq!(json["light"], serde_json::Value::Null);
        assert_eq!(json["color"], serde_json::json!([10, 20, 30]));
        assert_eq!(
            json["ambient"],
            serde_json::json!({ "basement": true, "caves": false, "level": 4 })
        );
        assert_eq!(json["radii"], serde_json::json!([1, 2, 3]));
    }

    #[test]
    fn changed_lights_relight_their_tiles() {
        let mut fixture = LightFixture::new();
        let a = fixture
            .place(&mode(1, LightTime::Always), grid(30, 30))
            .unwrap();
        fixture
            .place(&mode(1, LightTime::Always), grid(60, 60))
            .unwrap();
        let old = fixture.map().lights.clone();
        assert!(changed_light_tiles(&old, &old).is_empty());

        fixture
            .edit(|context, templates| context.change_light(templates, a, None, LightTime::Night))
            .unwrap();
        let tiles = changed_light_tiles(&old, &fixture.map().lights);
        // The template reaches 2 tiles further than its single node
        assert_eq!(tiles.len(), 25);
        assert!(tiles.contains(&grid(30, 30)) && tiles.contains(&grid(32, 28)));
        assert!(!tiles.contains(&grid(60, 60)));
    }

    /// The tiles that syncing with the map and the preview queues
    fn sync(
        synced: &mut SyncedLights,
        fixture: &LightFixture,
        preview: &PreviewLight,
    ) -> BTreeSet<GridNo> {
        let mut pending = PendingSceneUpdate::default();
        synced.sync(fixture.map(), preview, &mut pending);
        assert!(!pending.full);
        pending.tiles
    }

    /// The light that placing with the mode would add next
    fn preview(fixture: &LightFixture, mode: &LightMode, grid_no: GridNo) -> PreviewLight {
        PreviewLight(Some(
            new_light(
                &fixture.templates,
                mode,
                grid_no,
                fixture.fixture.document.next_light_id(),
            )
            .unwrap(),
        ))
    }

    #[test]
    fn the_preview_light_relights_its_tiles() {
        let mut fixture = LightFixture::new();
        let always = mode(1, LightTime::Always);
        let other = fixture.place(&always, grid(60, 60)).unwrap();
        let none = PreviewLight::default();
        let mut synced = SyncedLights::of(fixture.map(), &none);

        let at_a = preview(&fixture, &always, grid(30, 30));
        let tiles_a = light_tiles(at_a.0.as_ref().unwrap());
        assert_eq!(
            sync(&mut synced, &fixture, &at_a),
            BTreeSet::from_iter(tiles_a.clone())
        );
        // Moving it relights the old and the new tiles
        let at_b = preview(&fixture, &always, grid(40, 30));
        let tiles_b = light_tiles(at_b.0.as_ref().unwrap());
        let moved = sync(&mut synced, &fixture, &at_b);
        assert_eq!(moved, tiles_a.iter().chain(&tiles_b).copied().collect());
        // Clearing it relights the old tiles
        assert_eq!(
            sync(&mut synced, &fixture, &none),
            BTreeSet::from_iter(tiles_b)
        );

        // A preview on a light hides it, other changes relight the tiles of both
        let on_other = preview(&fixture, &mode(2, LightTime::Always), grid(60, 60));
        let tiles = sync(&mut synced, &fixture, &on_other);
        assert_eq!(tiles.len(), 25);
        assert!(tiles.contains(&grid(60, 60)));
        assert!(map_light(fixture.map(), other).is_some());

        // Placing the previewed light changes nothing, nor does the preview of the next light
        let at_c = preview(&fixture, &always, grid(30, 30));
        sync(&mut synced, &fixture, &at_c);
        fixture.place(&always, grid(30, 30)).unwrap();
        assert!(sync(&mut synced, &fixture, &at_c).is_empty());
        let next = preview(&fixture, &always, grid(30, 30));
        assert_ne!(next, at_c);
        assert!(sync(&mut synced, &fixture, &next).is_empty());
    }

    #[test]
    fn preview_changes_wait_for_the_scene() {
        use crate::tileset::TilesetId;
        use bevy::ecs::system::RunSystemOnce;

        let fixture = LightFixture::new();
        let light = preview(&fixture, &mode(1, LightTime::Always), grid(30, 30));
        let mut world = World::new();
        world.init_resource::<EditorDocument>();
        world.init_resource::<MapBake>();
        world.init_resource::<SyncedLights>();
        world.init_resource::<PreviewLight>();
        world.insert_resource(TacticalMap::empty(TilesetId::new(0)));
        world.insert_resource(PendingSceneUpdate {
            full: false,
            tiles: BTreeSet::from([grid(1, 1)]),
        });
        world.run_system_once(reset_synced_lights).unwrap();
        let system = world.register_system(sync_light_scene);
        world.run_system(system).unwrap();

        // The pending update of other edits comes first
        *world.resource_mut::<PreviewLight>() = light.clone();
        world.run_system(system).unwrap();
        assert_eq!(world.resource::<PendingSceneUpdate>().tiles.len(), 1);
        world.resource_mut::<PendingSceneUpdate>().tiles.clear();
        world.run_system(system).unwrap();
        let tiles = &world.resource::<PendingSceneUpdate>().tiles;
        assert_eq!(
            *tiles,
            BTreeSet::from_iter(light_tiles(light.0.as_ref().unwrap()))
        );
    }
}
