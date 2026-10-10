//! Light sprites: placing, erasing, selecting, moving and deleting them with the lights mode,
//! and the light colour and the ambient light of the map (the Map Info tab of the original
//! editor, `PlaceLight` and `RemoveLight` of `EditScreen.cc`, `EditorMapInfo.cc`)
//!
//! The `place` tool places a light with the radius and the time of the mode with a left click, it
//! replaces the lights of the tile. With `erase` its brush removes the lights of the tiles
//! instead. The `select` tool selects the light of the clicked tile, clicking it again selects
//! the next light of the tile. Dragging the selected light moves it, dragging it while Ctrl (Cmd
//! on macOS) was held on the press copies it, both replace the lights of the target tile. Delete
//! removes the selected light and Esc deselects it. Every change can be undone. The host edits
//! the selected light with `set_light` and the map lighting with `light_command`, see
//! `light_events.rs`.
//!
//! The light colour is the first colour of the lights of the map file, which the game adds to
//! the palettes. The ambient light is only saved for basements and caves, the editor shows it as
//! the "light level" (`EDITOR_LIGHT_MAX` minus the ambient level, so higher is brighter).

use std::sync::Arc;

use bevy::prelude::*;
use serde::Deserialize;

use crate::{
    light::{LightTemplate, LightTemplates, LIGHT_RADII},
    map::{light_template_name, GridNo, Level, LightId, LightTime, MapLight, TacticalMap},
    render::{lighting::PreviewLight, RenderOptions},
    systems::cursor::CursorInfo,
};

use super::{
    brush::{Brush, BrushArea},
    commands::CurrentMode,
    document::{EditContext, EditorDocument},
    mode::EditorMode,
    soldiers::edit_once,
    MapEdit,
};

/// The light level of the original editor is this minus the ambient level (`EDITOR_LIGHT_MAX`)
pub const EDITOR_LIGHT_MAX: u8 = 16;
/// The light levels the editor accepts
pub const LIGHT_LEVELS: std::ops::RangeInclusive<u8> = 1..=15;
/// The ambient level the original editor gives maps above ground, they keep it when they become
/// basements or caves (light level 12)
pub const SURFACE_AMBIENT_LEVEL: u8 = 4;

/// What a click in the lights mode does
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LightTool {
    /// Selects, moves, copies and deletes lights
    #[default]
    Select,
    /// Places lights, or erases them with the brush
    Place,
}

/// The lights mode (the lights of `TASK_MAPINFO` of the original editor)
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct LightMode {
    pub tool: LightTool,
    /// Removes the lights of the brush instead of placing them
    pub erase: bool,
    /// The brush that erases lights
    pub brush: Brush,
    /// Radius of new lights, the template `L-R0N.LHT` (`gsLightRadius`)
    pub radius: u8,
    /// When new lights are on (`gbDefaultLightType`)
    pub time: LightTime,
}

impl Default for LightMode {
    fn default() -> Self {
        Self {
            tool: LightTool::Select,
            erase: false,
            brush: Brush::default(),
            radius: 5,
            time: LightTime::PrimeTime,
        }
    }
}

impl LightMode {
    /// The brush that erases lights, `None` while placing or selecting, which use clicks
    pub fn brush(&self) -> Option<Brush> {
        match (self.tool, self.erase, self.brush) {
            (LightTool::Place, true, Brush::Fill) => Some(Brush::default()),
            (LightTool::Place, true, brush) => Some(brush),
            _ => None,
        }
    }

    /// Erases the lights of the tiles of the brush
    pub fn apply(&self, context: &mut EditContext, area: &BrushArea) {
        if self.brush().is_some() {
            context.erase_lights(area);
        }
    }
}

/// The light level of the editor of an ambient level
pub fn editor_light_level(ambient: u8) -> u8 {
    EDITOR_LIGHT_MAX - ambient.clamp(*LIGHT_LEVELS.start(), *LIGHT_LEVELS.end())
}

/// The index of a light in `TacticalMap::lights`
pub fn light_index(map: &TacticalMap, id: LightId) -> Option<usize> {
    map.lights.iter().position(|light| light.id == id)
}

/// The light with an id
pub fn map_light(map: &TacticalMap, id: LightId) -> Option<&MapLight> {
    map.lights.iter().find(|light| light.id == id)
}

/// The lights of a tile, in the order of the map
fn lights_on(map: &TacticalMap, grid_no: GridNo) -> Vec<LightId> {
    let position = grid_no.position();
    map.lights
        .iter()
        .filter(|light| light.position == position)
        .map(|light| light.id)
        .collect()
}

/// The name and the template of a radius, an error if it is out of range or not loaded
fn radius_template(
    templates: &LightTemplates,
    radius: u8,
) -> Result<(String, Arc<LightTemplate>), String> {
    if !LIGHT_RADII.contains(&radius) {
        return Err(format!(
            "Lights have a radius of {} to {}",
            LIGHT_RADII.start(),
            LIGHT_RADII.end()
        ));
    }
    let name = light_template_name(radius);
    let template = templates
        .get(&name)
        .ok_or_else(|| format!("The light template {} doesn't exist", name))?;
    Ok((name, template.clone()))
}

/// A new light with the radius and the time of the mode
pub fn new_light(
    templates: &LightTemplates,
    mode: &LightMode,
    grid_no: GridNo,
    id: LightId,
) -> Result<MapLight, String> {
    let (template_name, template) = radius_template(templates, mode.radius)?;
    Ok(MapLight {
        id,
        position: grid_no.position(),
        time: mode.time,
        template_name,
        template,
        file_sprite: None,
    })
}

impl EditContext<'_> {
    /// Places a light with the radius and the time of the mode, replacing the lights of the
    /// tile, returns the new light (`RemoveLight`, `PlaceLight`)
    ///
    /// Unlike the original, a missing template is not created.
    pub fn place_light(
        &mut self,
        templates: &LightTemplates,
        mode: &LightMode,
        grid_no: GridNo,
    ) -> Result<LightId, String> {
        let light = new_light(templates, mode, grid_no, self.document.next_light_id())?;
        // The id is only taken once the light can be placed
        let (id, position) = (self.document.new_light_id(), light.position);
        let lights = self.modify_lights();
        lights.retain(|light| light.position != position);
        lights.push(light);
        Ok(id)
    }

    /// Removes the lights of the tiles of the brush (`RemoveLight`), only recorded if there are
    /// any
    pub fn erase_lights(&mut self, area: &BrushArea) {
        let on_area = |light: &MapLight| area.tiles.contains(&GridNo::from(light.position));
        if !self.map.lights.iter().any(on_area) {
            return;
        }
        self.modify_lights().retain(|light| !on_area(light));
    }

    /// Moves a light to a tile, or copies it there with a new id, replacing the lights of the
    /// tile, returns the light on the tile, `None` if the light is gone or already there
    pub fn move_light(&mut self, id: LightId, to: GridNo, copy: bool) -> Option<LightId> {
        let index = light_index(self.map, id)?;
        let position = to.position();
        if self.map.lights[index].position == position {
            return None;
        }
        let mut light = self.map.lights[index].clone();
        light.position = position;
        if copy {
            light.id = self.document.new_light_id();
        }
        let moved = light.id;
        let lights = self.modify_lights();
        lights.retain(|light| light.position != position);
        match lights.iter_mut().find(|light| light.id == moved) {
            Some(existing) => *existing = light,
            None => lights.push(light),
        }
        Some(moved)
    }

    /// Removes a light (`RemoveLight`)
    pub fn delete_light(&mut self, id: LightId) {
        if let Some(index) = light_index(self.map, id) {
            self.modify_lights().remove(index);
        }
    }

    /// Changes the radius and the time of a light, it is recorded for undo if this changes it
    ///
    /// A radius of `None` keeps the template, e.g. of lights that are not of the editor.
    pub fn change_light(
        &mut self,
        templates: &LightTemplates,
        id: LightId,
        radius: Option<u8>,
        time: LightTime,
    ) -> Result<bool, String> {
        let Some(index) = light_index(self.map, id) else {
            return Ok(false);
        };
        let mut changed = self.map.lights[index].clone();
        changed.time = time;
        if let Some(radius) = radius.filter(|&radius| Some(radius) != changed.radius()) {
            let (name, template) = radius_template(templates, radius)?;
            changed.template_name = name;
            changed.template = template;
        }
        if changed == self.map.lights[index] {
            return Ok(false);
        }
        self.modify_lights()[index] = changed;
        Ok(true)
    }

    /// Sets the light colour and the light level of basements and caves (1 to 15, higher is
    /// brighter), only what changes is recorded (`ExtractAndUpdateMapInfo`)
    ///
    /// Maps without an ambient light ignore the level, like the original only saves it for
    /// basements and caves.
    pub fn change_light_settings(&mut self, color: [u8; 3], light_level: Option<u8>) -> bool {
        let ambient = self.map.ambient_light.map(|mut ambient| {
            if let Some(level) = light_level {
                let level = level.clamp(*LIGHT_LEVELS.start(), *LIGHT_LEVELS.end());
                ambient.level = EDITOR_LIGHT_MAX - level;
            }
            ambient
        });
        if color == self.map.light_color && ambient == self.map.ambient_light {
            return false;
        }
        let (light_color, ambient_light) = self.modify_light_settings();
        *light_color = color;
        *ambient_light = ambient;
        true
    }
}

/// A drag of the selected light in progress
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LightDrag {
    /// Where the mouse was pressed
    from: GridNo,
    /// Whether Ctrl or Cmd was held on the press, which copies the light instead of moving it
    copy: bool,
    /// Whether the light was selected before the press, releasing on the same tile then selects
    /// the next light of the tile
    was_selected: bool,
}

/// The selected light and what the host is told after a failed edit
#[derive(Resource, Clone, Debug, Default)]
pub struct LightSelection {
    pub selected: Option<LightId>,
    pub notice: Option<String>,
    drag: Option<LightDrag>,
}

impl LightSelection {
    /// Drops the selection when the light is gone, e.g. after undo
    pub fn validate(&mut self, map: &TacticalMap) {
        if self.selected.is_some_and(|id| map_light(map, id).is_none()) {
            self.selected = None;
            self.drag = None;
        }
    }

    /// Drops the selection and the drag
    pub fn clear(&mut self) {
        self.selected = None;
        self.drag = None;
    }

    /// Whether the selected light is being dragged
    pub fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    /// Selects the light of a tile, or keeps the selected one if it is there, and starts
    /// dragging it, or clears the selection if the tile has no lights
    pub fn press(&mut self, map: &TacticalMap, tile: GridNo, copy: bool) {
        let on_tile = lights_on(map, tile);
        let was_selected = self.selected.is_some_and(|id| on_tile.contains(&id));
        if !was_selected {
            let Some(&first) = on_tile.first() else {
                self.clear();
                return;
            };
            self.selected = Some(first);
        }
        self.drag = Some(LightDrag {
            from: tile,
            copy,
            was_selected,
        });
    }

    /// Ends the drag at a tile, moving or copying the light there and selecting it. Releasing
    /// where the selected light already was selects the next light of the tile.
    pub fn release(&mut self, context: &mut EditContext, to: GridNo) {
        let (Some(drag), Some(selected)) = (self.drag.take(), self.selected) else {
            return;
        };
        if to != drag.from {
            if let Some(id) = context.move_light(selected, to, drag.copy) {
                self.selected = Some(id);
            }
            return;
        }
        if drag.was_selected {
            let on_tile = lights_on(context.map, to);
            if let Some(position) = on_tile.iter().position(|&id| id == selected) {
                self.selected = Some(on_tile[(position + 1) % on_tile.len()]);
            }
        }
    }

    /// Removes the selected light
    pub fn delete(&mut self, context: &mut EditContext) {
        if let Some(selected) = self.selected.take() {
            context.delete_light(selected);
        }
        self.drag = None;
    }
}

pub fn reset_light_selection(mut selection: ResMut<LightSelection>) {
    *selection = LightSelection::default();
}

/// The lights mode, `None` for the others
pub fn light_mode(mode: &CurrentMode) -> Option<&LightMode> {
    match &mode.0 {
        EditorMode::Lights(mode) => Some(mode),
        _ => None,
    }
}

/// The mouse and the keys of the lights mode, erasing is done by `paint` with the brush
pub fn edit_lights(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    templates: Option<Res<LightTemplates>>,
    mut selection: ResMut<LightSelection>,
    mut edit: MapEdit,
) {
    // Lights are only edited on the ground floor
    let mode = light_mode(&mode).filter(|_| options.floor == Level::Ground);
    let Some(mode) = mode else {
        // Leaving the lights mode or the ground deselects the light
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        return;
    };
    let selection = &mut *selection;
    let tile = cursor.tile.map(GridNo::from);
    if mode.tool != LightTool::Select {
        // Only the select tool has a selection
        if selection.selected.is_some() || selection.drag.is_some() {
            selection.clear();
        }
        let (Some(tile), Some(templates), false) = (tile, templates, mode.erase) else {
            return;
        };
        if mouse.just_pressed(MouseButton::Left) {
            let mut result = Ok(());
            edit_once(&mut edit, |context| {
                result = context.place_light(&templates, mode, tile).map(|_| ())
            });
            if let Err(notice) = result {
                selection.notice = Some(notice);
            }
        }
        return;
    }
    if let Some(tile) = tile.filter(|_| mouse.just_pressed(MouseButton::Left)) {
        // Cmd on macOS, where Ctrl+click is a right click
        let copy = keys.any_pressed([
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::SuperLeft,
            KeyCode::SuperRight,
        ]);
        selection.press(&edit.map, tile, copy);
    }
    if selection.drag.is_some() {
        if cursor.world_position.is_none() {
            // Leaving the viewer cancels the drag, like the area brushes
            selection.drag = None;
        } else if !mouse.pressed(MouseButton::Left) {
            match tile {
                Some(to) => edit_once(&mut edit, |context| selection.release(context, to)),
                None => selection.drag = None,
            }
        }
    }
    if selection.drag.is_none() && keys.any_just_pressed([KeyCode::Delete, KeyCode::Backspace]) {
        edit_once(&mut edit, |context| selection.delete(context));
    }
    if keys.just_pressed(KeyCode::Escape) {
        selection.clear();
    }
    selection.validate(&edit.map);
}

/// The light that a click would place, `None` if a click places none
fn preview_light(
    mode: &CurrentMode,
    floor: Level,
    tile: Option<GridNo>,
    templates: Option<&LightTemplates>,
    id: LightId,
) -> Option<MapLight> {
    let mode = light_mode(mode)
        .filter(|mode| mode.tool == LightTool::Place && !mode.erase && floor == Level::Ground)?;
    new_light(templates?, mode, tile?, id).ok()
}

/// Lights the scene with the light that a click of the place tool would place, see
/// `sync_light_scene`
pub fn update_light_preview(
    cursor: Res<CursorInfo>,
    mode: Res<CurrentMode>,
    options: Res<RenderOptions>,
    templates: Option<Res<LightTemplates>>,
    document: Res<EditorDocument>,
    mut preview: ResMut<PreviewLight>,
) {
    // The light equals the placed one after a click, so that the scene isn't lit again
    let light = preview_light(
        &mode,
        options.floor,
        cursor.tile.map(GridNo::from),
        templates.as_deref(),
        document.next_light_id(),
    );
    preview.set_if_neq(PreviewLight(light));
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::{
        editor::document::tests::{grid, Fixture},
        light::LightNode,
        map::AmbientLight,
    };

    /// A map to place lights on, with the templates of the radii 1 to 5
    pub(crate) struct LightFixture {
        pub fixture: Fixture,
        pub templates: LightTemplates,
    }

    impl LightFixture {
        pub(crate) fn new() -> Self {
            let templates = (1..=5)
                .map(|radius| {
                    let template = LightTemplate {
                        lights: vec![LightNode {
                            dx: 0,
                            dy: 0,
                            light: radius,
                        }],
                        rays: vec![],
                    };
                    (
                        light_template_name(radius).to_lowercase(),
                        Arc::new(template),
                    )
                })
                .collect();
            Self {
                fixture: Fixture::new(),
                templates: LightTemplates(templates),
            }
        }

        /// Edits the map as one undo command
        pub(crate) fn edit<R>(
            &mut self,
            edit: impl FnOnce(&mut EditContext, &LightTemplates) -> R,
        ) -> R {
            self.fixture.document.undo.begin();
            let mut context = self.fixture.context();
            let result = edit(&mut context, &self.templates);
            context.finish();
            self.fixture.document.undo.end();
            result
        }

        pub(crate) fn place(
            &mut self,
            mode: &LightMode,
            grid_no: GridNo,
        ) -> Result<LightId, String> {
            self.edit(|context, templates| context.place_light(templates, mode, grid_no))
        }

        pub(crate) fn map(&self) -> &TacticalMap {
            &self.fixture.map
        }

        pub(crate) fn undo(&mut self) {
            let command = self.fixture.document.undo.pop().unwrap();
            let mut context = self.fixture.context();
            context.restore(command);
            context.finish();
        }
    }

    pub(crate) fn mode(radius: u8, time: LightTime) -> LightMode {
        LightMode {
            radius,
            time,
            ..LightMode::default()
        }
    }

    #[test]
    fn only_the_place_tool_previews_its_light() {
        let fixture = LightFixture::new();
        let tile = Some(grid(30, 30));
        let place = LightMode {
            tool: LightTool::Place,
            ..mode(3, LightTime::Night)
        };
        let preview = |mode: &LightMode, floor, tile| {
            let mode = CurrentMode(EditorMode::Lights(mode.clone()));
            preview_light(&mode, floor, tile, Some(&fixture.templates), LightId(7))
        };
        let light = preview(&place, Level::Ground, tile).unwrap();
        assert_eq!(light.id, LightId(7));
        assert_eq!(GridNo::from(light.position), grid(30, 30));
        assert_eq!((light.radius(), light.time), (Some(3), LightTime::Night));

        assert!(preview(&place, Level::Roof, tile).is_none());
        assert!(preview(&place, Level::Ground, None).is_none());
        let erase = LightMode {
            erase: true,
            ..place.clone()
        };
        assert!(preview(&erase, Level::Ground, tile).is_none());
        let select = LightMode {
            tool: LightTool::Select,
            ..place.clone()
        };
        assert!(preview(&select, Level::Ground, tile).is_none());
        let missing = LightMode {
            radius: 8,
            ..place.clone()
        };
        assert!(preview(&missing, Level::Ground, tile).is_none());
        let other = CurrentMode::default();
        assert!(preview_light(
            &other,
            Level::Ground,
            tile,
            Some(&fixture.templates),
            LightId(7)
        )
        .is_none());
        let mode = CurrentMode(EditorMode::Lights(place));
        assert!(preview_light(&mode, Level::Ground, tile, None, LightId(7)).is_none());
    }

    #[test]
    fn modes_are_read_from_json() {
        let read = |json: &str| match serde_json::from_str(json).unwrap() {
            EditorMode::Lights(mode) => mode,
            _ => panic!("not the lights mode"),
        };
        assert_eq!(
            read(r#"{"mode":"lights","radius":3,"time":"night"}"#),
            mode(3, LightTime::Night)
        );
        // The defaults of the original
        assert_eq!(read(r#"{"mode":"lights"}"#), mode(5, LightTime::PrimeTime));
        assert_eq!(
            read(r#"{"mode":"lights","time":"always"}"#).time,
            LightTime::Always
        );
        let erasing = read(
            r#"{"mode":"lights","tool":"place","erase":true,
                "brush":{"kind":"circle","size":5}}"#,
        );
        assert_eq!(erasing.brush(), Some(Brush::Circle { size: 5 }));
        let defaults = read(r#"{"mode":"lights"}"#);
        assert_eq!(
            (defaults.tool, defaults.erase, defaults.brush),
            (LightTool::Select, false, Brush::default())
        );
        // Only erasing uses the brush
        assert_eq!(defaults.brush(), None);
        let select = read(r#"{"mode":"lights","tool":"select","erase":true}"#);
        assert_eq!(select.tool, LightTool::Select);
        assert_eq!(select.brush(), None);
        let fill = read(r#"{"mode":"lights","tool":"place","erase":true,"brush":{"kind":"fill"}}"#);
        assert_eq!(fill.brush(), Some(Brush::default()));
    }

    #[test]
    fn erasing_removes_the_lights_of_the_brush() {
        let mut fixture = LightFixture::new();
        let always = mode(1, LightTime::Always);
        let inside = fixture.place(&always, grid(50, 50)).unwrap();
        fixture.place(&always, grid(51, 49)).unwrap();
        let outside = fixture.place(&always, grid(60, 50)).unwrap();
        let erase = LightMode {
            tool: LightTool::Place,
            erase: true,
            brush: Brush::Square { size: 3 },
            ..always.clone()
        };
        let area = erase.brush().unwrap().region(None, grid(50, 50).position());
        fixture.edit(|context, _| erase.apply(context, &area));
        let ids: Vec<_> = fixture.map().lights.iter().map(|l| l.id).collect();
        assert_eq!(ids, [outside]);
        // Nothing to erase is not recorded
        let depth = fixture.fixture.document.undo.depth();
        fixture.edit(|context, _| erase.apply(context, &area));
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        fixture.undo();
        assert_eq!(fixture.map().lights.len(), 3);
        assert!(map_light(fixture.map(), inside).is_some());
    }

    #[test]
    fn placing_replaces_the_lights_of_the_tile() {
        let mut fixture = LightFixture::new();
        let (a, b) = (grid(50, 50), grid(60, 50));
        let first = fixture.place(&mode(3, LightTime::Night), a).unwrap();
        let other = fixture.place(&mode(1, LightTime::Always), b).unwrap();
        let light = map_light(fixture.map(), first).unwrap();
        assert_eq!(light.template_name, "L-R03.LHT");
        assert_eq!(light.radius(), Some(3));
        assert_eq!(light.time, LightTime::Night);
        assert!(light.file_sprite.is_none());
        assert_eq!(light.template.lights[0].light, 3);

        let replaced = fixture.place(&mode(2, LightTime::PrimeTime), a).unwrap();
        let ids: Vec<_> = fixture.map().lights.iter().map(|l| l.id).collect();
        assert_eq!(ids, [other, replaced]);
        assert!(fixture.fixture.document.lights_dirty);

        // Missing templates and radii out of range are not placed
        for radius in [6, 0, 9] {
            assert!(fixture.place(&mode(radius, LightTime::Always), a).is_err());
        }
        assert_eq!(fixture.map().lights.len(), 2);
        assert_eq!(fixture.fixture.document.undo.depth(), 3);

        fixture.undo();
        let ids: Vec<_> = fixture.map().lights.iter().map(|l| l.id).collect();
        assert_eq!(ids, [first, other]);
    }

    /// Releases the drag of the selection as one undo command
    fn release(fixture: &mut LightFixture, selection: &mut LightSelection, to: GridNo) {
        fixture.edit(|context, _| selection.release(context, to));
    }

    #[test]
    fn pressing_selects_the_lights() {
        let mut fixture = LightFixture::new();
        let (a, b) = (grid(50, 50), grid(40, 60));
        let first = fixture.place(&mode(4, LightTime::Always), a).unwrap();
        let second = fixture.place(&mode(4, LightTime::Always), b).unwrap();
        let mut selection = LightSelection::default();
        selection.press(fixture.map(), b, false);
        assert_eq!(selection.selected, Some(second));
        assert!(selection.dragging());
        release(&mut fixture, &mut selection, b);
        assert_eq!(selection.selected, Some(second));
        assert!(!selection.dragging());
        selection.press(fixture.map(), a, false);
        assert_eq!(selection.selected, Some(first));
        // Tiles without lights deselect
        selection.press(fixture.map(), grid(10, 10), false);
        assert_eq!(selection.selected, None);
        assert!(!selection.dragging());
        // Nothing was changed
        assert_eq!(fixture.fixture.document.undo.depth(), 2);

        selection.press(fixture.map(), a, false);
        release(&mut fixture, &mut selection, a);
        fixture.edit(|context, _| selection.delete(context));
        assert_eq!(selection.selected, None);
        assert_eq!(fixture.map().lights.len(), 1);
        fixture.undo();
        assert_eq!(fixture.map().lights.len(), 2);
        selection.selected = Some(second);
        fixture.undo();
        selection.validate(fixture.map());
        assert_eq!(selection.selected, None);
    }

    #[test]
    fn releasing_on_the_selected_light_selects_the_next_one_of_the_tile() {
        let mut fixture = LightFixture::new();
        let a = grid(50, 50);
        let first = fixture.place(&mode(4, LightTime::Always), a).unwrap();
        // Old maps can have more than one light on a tile
        let mut stacked = fixture.map().lights[0].clone();
        stacked.id = fixture.fixture.document.new_light_id();
        let second = stacked.id;
        fixture.fixture.map.lights.push(stacked);

        let mut selection = LightSelection::default();
        // The first press only selects
        selection.press(fixture.map(), a, false);
        release(&mut fixture, &mut selection, a);
        assert_eq!(selection.selected, Some(first));
        selection.press(fixture.map(), a, false);
        release(&mut fixture, &mut selection, a);
        assert_eq!(selection.selected, Some(second));
        selection.press(fixture.map(), a, false);
        release(&mut fixture, &mut selection, a);
        assert_eq!(selection.selected, Some(first));
        assert_eq!(fixture.map().lights.len(), 2);
    }

    #[test]
    fn dragging_moves_or_copies_the_light() {
        let mut fixture = LightFixture::new();
        let (a, b, c) = (grid(50, 50), grid(55, 50), grid(60, 50));
        let id = fixture.place(&mode(3, LightTime::Night), a).unwrap();
        let replaced = fixture.place(&mode(1, LightTime::Always), b).unwrap();
        let mut selection = LightSelection::default();

        selection.press(fixture.map(), a, false);
        release(&mut fixture, &mut selection, b);
        assert_eq!(selection.selected, Some(id));
        assert_eq!(fixture.map().lights.len(), 1);
        let light = map_light(fixture.map(), id).unwrap();
        assert_eq!(GridNo::from(light.position), b);
        assert_eq!((light.radius(), light.time), (Some(3), LightTime::Night));
        assert!(map_light(fixture.map(), replaced).is_none());

        selection.press(fixture.map(), b, true);
        release(&mut fixture, &mut selection, c);
        let copy = selection.selected.unwrap();
        assert_ne!(copy, id);
        let copied = map_light(fixture.map(), copy).unwrap();
        assert_eq!(GridNo::from(copied.position), c);
        assert_eq!(copied.template_name, "L-R03.LHT");
        assert_eq!(
            GridNo::from(map_light(fixture.map(), id).unwrap().position),
            b
        );

        // The move is undone as a whole
        fixture.undo();
        fixture.undo();
        selection.validate(fixture.map());
        assert_eq!(selection.selected, None);
        let light = map_light(fixture.map(), id).unwrap();
        assert_eq!(GridNo::from(light.position), a);
        assert!(map_light(fixture.map(), replaced).is_some());
    }

    #[test]
    fn the_selected_light_can_be_edited() {
        let mut fixture = LightFixture::new();
        let id = fixture
            .place(&mode(2, LightTime::Always), grid(50, 50))
            .unwrap();
        let changed = fixture.edit(|context, templates| {
            context.change_light(templates, id, Some(4), LightTime::Night)
        });
        assert_eq!(changed, Ok(true));
        let light = map_light(fixture.map(), id).unwrap();
        assert_eq!((light.radius(), light.time), (Some(4), LightTime::Night));
        assert_eq!(light.template.lights[0].light, 4);
        let depth = fixture.fixture.document.undo.depth();

        // The same values change nothing, missing templates fail
        let same = fixture.edit(|context, templates| {
            context.change_light(templates, id, Some(4), LightTime::Night)
        });
        assert_eq!(same, Ok(false));
        let missing = fixture.edit(|context, templates| {
            context.change_light(templates, id, Some(8), LightTime::Always)
        });
        assert!(missing.is_err());
        assert_eq!(fixture.fixture.document.undo.depth(), depth);

        // Lights with other templates keep them
        fixture.fixture.map.lights[0].template_name = "LAMP.LHT".to_owned();
        fixture
            .edit(|context, templates| context.change_light(templates, id, None, LightTime::Always))
            .unwrap();
        let light = map_light(fixture.map(), id).unwrap();
        assert_eq!(light.template_name, "LAMP.LHT");
        assert_eq!(light.time, LightTime::Always);

        fixture.undo();
        assert_eq!(map_light(fixture.map(), id).unwrap().time, LightTime::Night);
    }

    #[test]
    fn the_colour_and_the_light_level_can_be_changed() {
        let mut fixture = LightFixture::new();
        // Maps without an ambient light ignore the level
        assert!(fixture.edit(|context, _| context.change_light_settings([1, 2, 3], Some(4))));
        assert_eq!(fixture.map().light_color, [1, 2, 3]);
        assert_eq!(fixture.map().ambient_light, None);
        assert!(!fixture.edit(|context, _| context.change_light_settings([1, 2, 3], Some(4))));

        fixture.fixture.map.ambient_light = Some(AmbientLight {
            basement: true,
            caves: false,
            level: 10,
        });
        assert_eq!(editor_light_level(10), 6);
        assert!(fixture.edit(|context, _| context.change_light_settings([1, 2, 3], Some(12))));
        assert_eq!(fixture.map().ambient_light.unwrap().level, 4);
        // Clamped like the original
        fixture.edit(|context, _| context.change_light_settings([1, 2, 3], Some(40)));
        assert_eq!(fixture.map().ambient_light.unwrap().level, 1);
        fixture.edit(|context, _| context.change_light_settings([1, 2, 3], Some(0)));
        assert_eq!(fixture.map().ambient_light.unwrap().level, 15);
        // `None` keeps the level
        fixture.edit(|context, _| context.change_light_settings([9, 9, 9], None));
        assert_eq!(fixture.map().ambient_light.unwrap().level, 15);

        for _ in 0..4 {
            fixture.undo();
        }
        assert_eq!(fixture.map().ambient_light.unwrap().level, 10);
        assert_eq!(fixture.map().light_color, [1, 2, 3]);
        // The first edit had no ambient light
        fixture.undo();
        assert_eq!(fixture.map().light_color, [0, 0, 0]);
        assert_eq!(fixture.map().ambient_light, None);
    }
}
