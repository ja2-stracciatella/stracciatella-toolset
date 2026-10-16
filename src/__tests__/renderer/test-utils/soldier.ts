import type {
  DetailedPlacement,
  EditorSoldierEvent,
  ScheduleRow,
  Soldier,
  SoldierPaletteEvent,
} from '../../../renderer/lib/mapViewer';

/** A schedule without actions */
export function emptySchedule(): ScheduleRow[] {
  return Array.from({ length: 4 }, () => ({
    action: 'none',
    time: null,
    variance: false,
    data1: null,
    data2: null,
  }));
}

/** The choices of the soldiers panel, like the viewer sends them */
export function soldierPaletteEvent(map = 'maps/A9.dat'): SoldierPaletteEvent {
  return {
    type: 'soldierPalette',
    map,
    maxSoldiers: 148,
    maxTeamSoldiers: 32,
    enemyClasses: [
      { id: 3, name: 'Army' },
      { id: 1, name: 'Admin' },
      { id: 2, name: 'Elite' },
    ],
    bodies: {
      enemy: [
        { id: null, name: 'Random' },
        { id: 0, name: 'Reg Male' },
        { id: 3, name: 'Reg Female' },
      ],
      creature: [
        { id: 20, name: 'Bloodcat' },
        { id: 10, name: 'Queen Monster' },
      ],
      rebel: [{ id: null, name: 'Random' }],
      civilian: [
        { id: null, name: 'Random' },
        { id: 12, name: 'M Civilian' },
      ],
    },
    civilianGroups: ['NONE', 'REBEL', 'KINGPIN'],
    colours: {
      head: ['BROWNHEAD', 'BLONDHEAD'],
      skin: ['PINKSKIN', 'DARKSKIN'],
      vest: ['REDVEST', 'BLUEVEST'],
      pants: ['GREENPANTS', 'BLUEPANTS'],
    },
  };
}

/** A detailed placement with nothing fixed */
export function detailedPlacement(): DetailedPlacement {
  return {
    profile: null,
    copyProfileItemsOver: false,
    stats: {
      lifeMax: null,
      life: null,
      agility: null,
      dexterity: null,
      expLevel: null,
      marksmanship: null,
      medical: null,
      mechanical: null,
      explosive: null,
      leadership: null,
      strength: null,
      wisdom: null,
      morale: null,
      aiMorale: null,
    },
    inventory: Array.from({ length: 19 }, () => null),
    visible: false,
    headPalette: '',
    pantsPalette: '',
    vestPalette: '',
    skinPalette: '',
  };
}

/** A basic enemy placement */
export function soldier(overrides: Partial<Soldier> = {}): Soldier {
  return {
    id: 7,
    slot: 20,
    team: 'enemy',
    gridNo: 12345,
    onRoof: false,
    drawnBody: 1,
    drawnPalette: {
      head: 'BROWNHEAD',
      skin: 'PINKSKIN',
      vest: 'REDVEST',
      pants: 'GREENPANTS',
    },
    patrol: [],
    orders: 'stationary',
    attitude: 'defensive',
    direction: 'northWest',
    relativeAttributeLevel: 2,
    relativeEquipmentLevel: 2,
    soldierClass: 3,
    civilianGroup: 0,
    body: null,
    priorityExistence: false,
    hasKeys: false,
    detailed: null,
    ...overrides,
  };
}

/** The selected soldier as the viewer reports it */
export function soldierEvent(
  selected: Soldier | null,
  overrides: Partial<EditorSoldierEvent> = {},
): EditorSoldierEvent {
  return {
    type: 'soldier',
    map: 'maps/A9.dat',
    soldier: selected,
    counts: { enemy: 1, creature: 0, rebel: 0, civilian: 0, total: 1 },
    schedule: selected?.detailed ? emptySchedule() : null,
    pick: null,
    ...overrides,
  };
}
