//! Schedules of civilians (the schedule mode of the original's Mercs tab, `EditorMercs.cc`)
//!
//! The host edits the 4 actions of the schedule of the selected soldier. An action that needs a
//! tile is not complete until the tiles are picked on the map: a click picks the tile next to the
//! door and then where the soldier goes (`RegisterCurrentScheduleAction`). The rows the host sees
//! are a draft that keeps the incomplete actions, the schedule of the map only has the complete
//! ones, like the game only loads those. Escape cancels a pick, an action whose tiles are missing
//! is removed then (`CancelCurrentScheduleAction`).

use serde::{Deserialize, Serialize};

use crate::map::{
    GridNo, Level, PlacementId, Schedule, ScheduleAction, ScheduleEntry, ScheduleFlags, ScheduleId,
    TacticalMap,
};

use super::{
    document::EditContext,
    soldiers::{is_sittable, placement},
};

/// The actions of a schedule (`SCHEDULE_ACTION_*`)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScheduleActionKind {
    #[default]
    None,
    LockDoor,
    UnlockDoor,
    OpenDoor,
    CloseDoor,
    GridNo,
    LeaveSector,
    EnterSector,
    StayInSector,
    Sleep,
    Wake,
}

/// Which tiles an action needs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionTiles {
    None,
    /// The tile it goes to (A)
    One,
    /// The tile next to the door (A) and where it goes afterwards (B)
    Door,
}

impl ScheduleActionKind {
    pub fn tiles(self) -> ActionTiles {
        match self {
            Self::LockDoor | Self::UnlockDoor | Self::OpenDoor | Self::CloseDoor => {
                ActionTiles::Door
            }
            Self::GridNo | Self::EnterSector | Self::Sleep => ActionTiles::One,
            Self::None | Self::LeaveSector | Self::StayInSector | Self::Wake => ActionTiles::None,
        }
    }

    /// The action of a schedule entry and its tiles
    fn of(action: &ScheduleAction) -> (Self, Option<GridNo>, Option<GridNo>) {
        match *action {
            ScheduleAction::LockDoor { door, destination } => {
                (Self::LockDoor, Some(door), Some(destination))
            }
            ScheduleAction::UnlockDoor { door, destination } => {
                (Self::UnlockDoor, Some(door), Some(destination))
            }
            ScheduleAction::OpenDoor { door, destination } => {
                (Self::OpenDoor, Some(door), Some(destination))
            }
            ScheduleAction::CloseDoor { door, destination } => {
                (Self::CloseDoor, Some(door), Some(destination))
            }
            ScheduleAction::GridNo(grid_no) => (Self::GridNo, Some(grid_no), None),
            ScheduleAction::LeaveSector => (Self::LeaveSector, None, None),
            ScheduleAction::EnterSector(grid_no) => (Self::EnterSector, Some(grid_no), None),
            ScheduleAction::StayInSector => (Self::StayInSector, None, None),
            ScheduleAction::Sleep(grid_no) => (Self::Sleep, Some(grid_no), None),
            ScheduleAction::Wake => (Self::Wake, None, None),
        }
    }

    /// The schedule action, `None` without an action or while tiles are missing
    fn action(self, data1: Option<GridNo>, data2: Option<GridNo>) -> Option<ScheduleAction> {
        let door = || Some((data1?, data2?));
        Some(match self {
            Self::None => return None,
            Self::LockDoor => {
                let (door, destination) = door()?;
                ScheduleAction::LockDoor { door, destination }
            }
            Self::UnlockDoor => {
                let (door, destination) = door()?;
                ScheduleAction::UnlockDoor { door, destination }
            }
            Self::OpenDoor => {
                let (door, destination) = door()?;
                ScheduleAction::OpenDoor { door, destination }
            }
            Self::CloseDoor => {
                let (door, destination) = door()?;
                ScheduleAction::CloseDoor { door, destination }
            }
            Self::GridNo => ScheduleAction::GridNo(data1?),
            Self::LeaveSector => ScheduleAction::LeaveSector,
            Self::EnterSector => ScheduleAction::EnterSector(data1?),
            Self::StayInSector => ScheduleAction::StayInSector,
            Self::Sleep => ScheduleAction::Sleep(data1?),
            Self::Wake => ScheduleAction::Wake,
        })
    }
}

/// An action of a schedule as the host edits it
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleRow {
    pub action: ScheduleActionKind,
    /// Minutes since midnight, `None` without a time
    pub time: Option<u16>,
    /// The time varies by up to 15 minutes
    pub variance: bool,
    /// The tile next to the door or the tile it goes to (A)
    pub data1: Option<u16>,
    /// Where it goes after the door (B)
    pub data2: Option<u16>,
}

/// Minutes of a day
const MINUTES_PER_DAY: u16 = 24 * 60;

impl ScheduleRow {
    /// The row with only the values its action uses (`StartScheduleAction`)
    fn normalized(self) -> Self {
        let tiles = self.action.tiles();
        let grid = |data: Option<u16>| data.filter(|&d| GridNo::new(usize::from(d)).is_some());
        Self {
            action: self.action,
            time: self.time.filter(|&time| time < MINUTES_PER_DAY),
            variance: self.variance,
            data1: grid(self.data1).filter(|_| tiles != ActionTiles::None),
            data2: grid(self.data2).filter(|_| tiles == ActionTiles::Door),
        }
    }

    fn grid(data: Option<u16>) -> Option<GridNo> {
        GridNo::new(usize::from(data?))
    }

    /// The entry of the schedule, `None` without an action or while tiles are missing
    fn entry(&self) -> Option<ScheduleEntry> {
        let action = self
            .action
            .action(Self::grid(self.data1), Self::grid(self.data2))?;
        Some(ScheduleEntry {
            time: self.time,
            action,
        })
    }

    /// Whether the action still needs tiles
    fn is_incomplete(&self) -> bool {
        self.action != ScheduleActionKind::None && self.entry().is_none()
    }
}

/// Which tile a click picks
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SchedulePoint {
    A,
    B,
}

/// The tile of an action that the next click picks
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchedulePick {
    /// The action, 0 to 3
    pub slot: usize,
    pub point: SchedulePoint,
    /// Only this tile is picked, otherwise picking the tile next to a door goes on with where
    /// the soldier goes afterwards (`gfSingleAction`)
    #[serde(default)]
    pub single: bool,
}

/// The variance flag of a slot
fn variance_flag(slot: usize) -> ScheduleFlags {
    [
        ScheduleFlags::VARIANCE1,
        ScheduleFlags::VARIANCE2,
        ScheduleFlags::VARIANCE3,
        ScheduleFlags::VARIANCE4,
    ][slot]
}

/// The rows of the schedule of a soldier, with the actions whose tiles are missing
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleDraft {
    pub placement: PlacementId,
    pub rows: [ScheduleRow; 4],
}

impl ScheduleDraft {
    /// The rows of the schedule of a placement, `None` if it has no detailed placement
    pub fn of(map: &TacticalMap, id: PlacementId) -> Option<Self> {
        let detailed = placement(map, id)?.detailed.as_ref()?;
        let schedule = detailed
            .schedule
            .and_then(|schedule| map.schedules.iter().find(|s| s.id == schedule));
        let rows = std::array::from_fn(|slot| {
            let Some(schedule) = schedule else {
                return ScheduleRow::default();
            };
            let variance = schedule.flags.contains(variance_flag(slot));
            match &schedule.actions[slot] {
                Some(entry) => {
                    let (action, data1, data2) = ScheduleActionKind::of(&entry.action);
                    let raw = |grid: Option<GridNo>| grid.map(|g| g.index() as u16);
                    ScheduleRow {
                        action,
                        time: entry.time,
                        variance,
                        data1: raw(data1),
                        data2: raw(data2),
                    }
                }
                None => ScheduleRow {
                    variance,
                    ..Default::default()
                },
            }
        });
        Some(Self {
            placement: id,
            rows,
        })
    }

    /// Whether the draft gives the schedule the map has, it is out of date otherwise, e.g. after
    /// undo
    pub fn matches(&self, map: &TacticalMap) -> bool {
        let Some(current) = Self::of(map, self.placement) else {
            return false;
        };
        // Only the complete actions are in the map
        let complete = |rows: &[ScheduleRow; 4]| {
            rows.map(|row| {
                let entry = row.entry();
                (entry, entry.is_some() && row.variance)
            })
        };
        complete(&self.rows) == complete(&current.rows)
    }
}

/// The first schedule id that is not used
fn free_schedule_id(map: &TacticalMap) -> Option<ScheduleId> {
    (1..=u8::MAX)
        .filter_map(ScheduleId::new)
        .find(|id| map.schedules.iter().all(|s| s.id != *id))
}

impl EditContext<'_> {
    /// Gives the soldier of the draft the schedule of its complete rows, or removes its schedule
    /// if no row has an action (`ExtractAndUpdateMercSchedule`)
    pub fn apply_schedule(&mut self, draft: &ScheduleDraft) {
        let Some(soldier) = placement(self.map, draft.placement) else {
            return;
        };
        let Some(detailed) = &soldier.detailed else {
            return;
        };
        let current_id = detailed.schedule;
        let current = current_id.and_then(|id| self.map.schedules.iter().find(|s| s.id == id));
        let has_actions = draft
            .rows
            .iter()
            .any(|row| row.action != ScheduleActionKind::None);
        if !has_actions {
            if current_id.is_none() {
                return;
            }
            let index = self
                .map
                .soldiers
                .iter()
                .position(|p| p.id == draft.placement);
            let (soldiers, schedules) = self.modify_soldiers();
            if let Some(detailed) = index.and_then(|i| soldiers[i].detailed.as_mut()) {
                detailed.schedule = None;
            }
            schedules.retain(|s| Some(s.id) != current_id);
            return;
        }
        let Some(id) = current.map(|s| s.id).or_else(|| free_schedule_id(self.map)) else {
            return;
        };
        let mut flags = current.map_or(ScheduleFlags::empty(), |s| s.flags);
        for (slot, row) in draft.rows.iter().enumerate() {
            flags.set(
                variance_flag(slot),
                row.variance && row.action != ScheduleActionKind::None,
            );
        }
        let schedule = Schedule {
            id,
            soldier_id: current.map_or(0, |s| s.soldier_id),
            flags,
            actions: draft.rows.map(|row| row.entry()),
        };
        if current == Some(&schedule) {
            return;
        }
        let index = self
            .map
            .soldiers
            .iter()
            .position(|p| p.id == draft.placement);
        let (soldiers, schedules) = self.modify_soldiers();
        match schedules.iter_mut().find(|s| s.id == id) {
            Some(existing) => *existing = schedule,
            None => schedules.push(schedule),
        }
        if let Some(detailed) = index.and_then(|i| soldiers[i].detailed.as_mut()) {
            detailed.schedule = Some(id);
        }
    }
}

/// The schedule draft of the selected soldier and the tile a click picks
#[derive(Clone, Debug, Default)]
pub struct ScheduleEditing {
    pub draft: Option<ScheduleDraft>,
    pub pick: Option<SchedulePick>,
}

impl ScheduleEditing {
    /// Follows the selected soldier and the map, e.g. after undo
    pub fn sync(&mut self, map: &TacticalMap, selected: Option<PlacementId>) {
        let current = self
            .draft
            .as_ref()
            .filter(|draft| Some(draft.placement) == selected && draft.matches(map));
        if current.is_none() {
            self.draft = selected.and_then(|id| ScheduleDraft::of(map, id));
            self.pick = None;
        }
    }

    /// Sets the rows of the host, the action of a row decides which values it keeps
    pub fn set_rows(&mut self, context: &mut EditContext, rows: [ScheduleRow; 4]) {
        let Some(draft) = &mut self.draft else {
            return;
        };
        draft.rows = rows.map(ScheduleRow::normalized);
        // A pick of an action that has no such tile any more ends
        if let Some(pick) = self.pick {
            let tiles = draft.rows[pick.slot].action.tiles();
            if tiles == ActionTiles::None
                || (pick.point == SchedulePoint::B && tiles != ActionTiles::Door)
            {
                self.pick = None;
            }
        }
        context.apply_schedule(draft);
    }

    /// Starts picking a tile of an action that needs it (`StartScheduleAction`)
    pub fn start_pick(&mut self, pick: SchedulePick) {
        let Some(draft) = &self.draft else {
            return;
        };
        let Some(row) = draft.rows.get(pick.slot) else {
            return;
        };
        let allowed = matches!(
            (row.action.tiles(), pick.point),
            (ActionTiles::Door, _) | (ActionTiles::One, SchedulePoint::A)
        );
        if allowed {
            self.pick = Some(pick);
        }
    }

    /// Cancels the pick, the action is removed if it misses tiles (`CancelCurrentScheduleAction`)
    pub fn cancel_pick(&mut self, context: &mut EditContext) {
        let (Some(pick), Some(draft)) = (self.pick.take(), &mut self.draft) else {
            return;
        };
        if draft.rows[pick.slot].is_incomplete() {
            draft.rows[pick.slot] = ScheduleRow::default();
            context.apply_schedule(draft);
        }
    }

    /// A click on a tile while a tile is picked, returns whether it was used
    /// (`RegisterCurrentScheduleAction`)
    pub fn click(&mut self, context: &mut EditContext, grid_no: GridNo) -> bool {
        let (Some(pick), Some(draft)) = (self.pick, &mut self.draft) else {
            return false;
        };
        // Soldiers walk there, others can stand there already
        if !is_sittable(context.map, context.world, grid_no, Level::Ground, false) {
            return true;
        }
        let row = &mut draft.rows[pick.slot];
        let raw = Some(grid_no.index() as u16);
        match pick.point {
            SchedulePoint::A => row.data1 = raw,
            SchedulePoint::B => row.data2 = raw,
        }
        self.pick = (pick.point == SchedulePoint::A
            && row.action.tiles() == ActionTiles::Door
            && !pick.single)
            .then_some(SchedulePick {
                point: SchedulePoint::B,
                ..pick
            });
        context.apply_schedule(draft);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::editor::{
        document::tests::grid,
        soldiers::tests::{mode, SoldierFixture},
    };

    fn row(action: ScheduleActionKind, time: Option<u16>) -> ScheduleRow {
        ScheduleRow {
            action,
            time,
            ..Default::default()
        }
    }

    /// A detailed civilian, selected
    fn civilian(fixture: &mut SoldierFixture) -> (PlacementId, ScheduleEditing) {
        let civilians = mode(r#"{"mode":"soldiers","team":"civilian"}"#);
        let id = fixture
            .edit(|context, sc| context.place_soldier(sc, &civilians, grid(30, 30), Level::Ground))
            .unwrap();
        fixture.edit(|context, sc| context.set_detailed(sc, id, true));
        let mut editing = ScheduleEditing::default();
        editing.sync(fixture.map(), Some(id));
        (id, editing)
    }

    fn schedule(fixture: &SoldierFixture, id: PlacementId) -> Option<&Schedule> {
        let schedule = fixture.soldier(id).detailed.as_ref()?.schedule?;
        fixture.map().schedules.iter().find(|s| s.id == schedule)
    }

    #[test]
    fn rows_are_json_for_the_host() {
        let row: ScheduleRow = serde_json::from_str(
            r#"{"action":"lockDoor","time":480,"variance":true,"data1":100,"data2":null}"#,
        )
        .unwrap();
        assert_eq!(row.action, ScheduleActionKind::LockDoor);
        assert_eq!(row.time, Some(480));
        let pick: SchedulePick = serde_json::from_str(r#"{"slot":2,"point":"b"}"#).unwrap();
        assert_eq!(pick.point, SchedulePoint::B);
        assert!(!pick.single);
    }

    #[test]
    fn complete_actions_make_the_schedule() {
        let mut fixture = SoldierFixture::new();
        let (id, mut editing) = civilian(&mut fixture);
        assert_eq!(
            editing.draft.as_ref().unwrap().rows,
            [ScheduleRow::default(); 4]
        );

        let mut rows = [ScheduleRow::default(); 4];
        rows[0] = row(ScheduleActionKind::LeaveSector, Some(600));
        rows[0].variance = true;
        rows[1] = row(ScheduleActionKind::Sleep, Some(1320));
        // Data that the action doesn't use is dropped
        rows[2] = ScheduleRow {
            data1: Some(5),
            ..row(ScheduleActionKind::StayInSector, None)
        };
        fixture.edit(|context, _| editing.set_rows(context, rows));
        let saved = schedule(&fixture, id).unwrap().clone();
        assert_eq!(saved.id, ScheduleId::new(1).unwrap());
        assert!(saved.flags.contains(ScheduleFlags::VARIANCE1));
        assert_eq!(
            saved.actions[0],
            Some(ScheduleEntry {
                time: Some(600),
                action: ScheduleAction::LeaveSector
            })
        );
        // Sleeping needs a tile
        assert_eq!(saved.actions[1], None);
        assert_eq!(
            saved.actions[2].map(|entry| entry.action),
            Some(ScheduleAction::StayInSector)
        );
        let draft = editing.draft.as_ref().unwrap();
        assert_eq!(draft.rows[1].action, ScheduleActionKind::Sleep);
        assert_eq!(draft.rows[2].data1, None);
        // The draft keeps the incomplete action while it matches the map
        editing.sync(fixture.map(), Some(id));
        assert_eq!(
            editing.draft.as_ref().unwrap().rows[1].action,
            ScheduleActionKind::Sleep
        );

        // Without actions, the schedule is removed
        fixture.edit(|context, _| editing.set_rows(context, [ScheduleRow::default(); 4]));
        assert!(schedule(&fixture, id).is_none());
        assert!(fixture.map().schedules.is_empty());
    }

    #[test]
    fn door_actions_pick_two_tiles() {
        let mut fixture = SoldierFixture::new();
        let (id, mut editing) = civilian(&mut fixture);
        let mut rows = [ScheduleRow::default(); 4];
        rows[1] = row(ScheduleActionKind::OpenDoor, Some(480));
        fixture.edit(|context, _| editing.set_rows(context, rows));
        // Picking B of an action without tiles or A of no action does nothing
        editing.start_pick(SchedulePick {
            slot: 0,
            point: SchedulePoint::A,
            single: false,
        });
        assert_eq!(editing.pick, None);
        editing.start_pick(SchedulePick {
            slot: 1,
            point: SchedulePoint::A,
            single: false,
        });
        fixture.edit(|context, _| assert!(editing.click(context, grid(40, 40))));
        // The door was picked, then where it goes
        assert_eq!(
            editing.pick,
            Some(SchedulePick {
                slot: 1,
                point: SchedulePoint::B,
                single: false
            })
        );
        assert_eq!(schedule(&fixture, id).unwrap().actions[1], None);
        fixture.edit(|context, _| assert!(editing.click(context, grid(41, 41))));
        assert_eq!(editing.pick, None);
        assert_eq!(
            schedule(&fixture, id).unwrap().actions[1],
            Some(ScheduleEntry {
                time: Some(480),
                action: ScheduleAction::OpenDoor {
                    door: grid(40, 40),
                    destination: grid(41, 41)
                }
            })
        );
        // Without a pick, clicks are not used
        fixture.edit(|context, _| assert!(!editing.click(context, grid(42, 42))));

        // Picking only the door again
        editing.start_pick(SchedulePick {
            slot: 1,
            point: SchedulePoint::A,
            single: true,
        });
        fixture.edit(|context, _| editing.click(context, grid(43, 43)));
        assert_eq!(editing.pick, None);
        let draft = editing.draft.as_ref().unwrap();
        assert_eq!(draft.rows[1].data1, Some(grid(43, 43).index() as u16));

        // Undo brings back the door before, the draft follows
        let command = fixture.fixture.document.undo.pop().unwrap();
        let mut context = fixture.fixture.context();
        context.restore(command);
        context.finish();
        editing.sync(fixture.map(), Some(id));
        let draft = editing.draft.as_ref().unwrap();
        assert_eq!(draft.rows[1].data1, Some(grid(40, 40).index() as u16));
    }

    #[test]
    fn cancelling_removes_actions_without_tiles() {
        let mut fixture = SoldierFixture::new();
        let (id, mut editing) = civilian(&mut fixture);
        let mut rows = [ScheduleRow::default(); 4];
        rows[0] = row(ScheduleActionKind::GridNo, Some(60));
        rows[3] = row(ScheduleActionKind::LeaveSector, Some(120));
        fixture.edit(|context, _| editing.set_rows(context, rows));
        editing.start_pick(SchedulePick {
            slot: 0,
            point: SchedulePoint::A,
            single: false,
        });
        fixture.edit(|context, _| editing.cancel_pick(context));
        assert_eq!(editing.pick, None);
        let draft = editing.draft.as_ref().unwrap();
        assert_eq!(draft.rows[0], ScheduleRow::default());
        assert!(schedule(&fixture, id).unwrap().actions[3].is_some());

        // Other soldiers have other drafts, selecting none has none
        editing.sync(fixture.map(), None);
        assert!(editing.draft.is_none());
    }

    #[test]
    fn edited_schedules_are_saved() {
        use crate::file_formats::ja2_map::{Ja2Map, Ja2MapElement};

        let mut fixture = SoldierFixture::new();
        let (id, mut editing) = civilian(&mut fixture);
        let mut rows = [ScheduleRow::default(); 4];
        rows[0] = row(ScheduleActionKind::GridNo, Some(900));
        rows[1] = row(ScheduleActionKind::LeaveSector, Some(60));
        rows[1].variance = true;
        fixture.edit(|context, _| editing.set_rows(context, rows));
        editing.start_pick(SchedulePick {
            slot: 0,
            point: SchedulePoint::A,
            single: false,
        });
        fixture.edit(|context, _| editing.click(context, grid(50, 50)));
        assert!(schedule(&fixture, id).unwrap().actions[0].is_some());

        let mut raw = Ja2Map {
            map_elements: vec![Ja2MapElement::default(); crate::components::map::JA2_MAP_TILES],
            ..Default::default()
        };
        fixture.map().apply_to_ja2(&mut raw).unwrap();
        let saved = &raw.schedules[0];
        // Sorted by time, the variance moves with the action
        assert_eq!(saved.action[..2], [6, 5]);
        assert_eq!(saved.time[..2], [60, 900]);
        assert_eq!(saved.data1[1], grid(50, 50).index() as u16);
        assert_eq!(saved.flags, ScheduleFlags::VARIANCE1.bits());
        assert_eq!(saved.soldier_id, 0);
        let detailed = raw.placements[0].detailed.as_ref().unwrap();
        assert_eq!(detailed.schedule_id, saved.schedule_id);
    }
}
