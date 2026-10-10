import { z } from 'zod';
import { invoke } from './invoke';

// The tactical map viewer is a Bevy app compiled to wasm (src-map-editor). winit can only create
// its event loop once per page, so the app is started once and kept alive. Its canvas is moved
// between the page showing the viewer and a hidden holder.

export interface MapViewerStatus {
  state: 'loading' | 'loaded' | 'error';
  map: string;
  message?: string;
}

/** The teams the editor places soldiers of, see `SoldierTeam` in `src-map-editor` */
export const SOLDIER_TEAMS = [
  'enemy',
  'creature',
  'rebel',
  'civilian',
] as const;
export type SoldierTeam = (typeof SOLDIER_TEAMS)[number];

/** Which teams of the soldiers layer are shown */
export type VisibleTeams = Record<SoldierTeam, boolean>;

/** Layers of the map that the viewer can hide */
export interface MapLayers {
  /**
   * What the terrain mode places: ground textures, water, roads, cliffs and their shadows, and
   * the floors of buildings
   */
  terrain: boolean;
  /** What the objects mode places: debris, trees, rocks, junk, furniture and their shadows */
  objects: boolean;
  /**
   * What the buildings mode places but the floors: walls, doors and their shadows, wall decals,
   * switches, roofs and what is on them
   */
  buildings: boolean;
  /** The soldiers of the soldiers mode */
  soldiers: boolean;
  /** Which teams of the soldiers layer are drawn, edited and selected */
  soldierTeams: VisibleTeams;
  /** The items, which are drawn whatever mode placed what they lie on */
  items: boolean;
  /** The room numbers of the tiles */
  roomNumbers: boolean;
}

export type MapLighting = 'day' | 'night';

/**
 * The floor that is shown and edited: the roofs and what is on them are only drawn on the roof,
 * and only soldiers and items on the floor can be edited. The terrain, objects, lights and
 * traversal modes only edit the ground.
 */
export type MapFloor = 'ground' | 'roof';

/** What the viewer shows */
export interface MapRenderOptions {
  layers: MapLayers;
  floor: MapFloor;
  lighting: MapLighting;
}

export const DEFAULT_RENDER_OPTIONS: MapRenderOptions = {
  layers: {
    terrain: true,
    objects: true,
    buildings: true,
    soldiers: true,
    soldierTeams: { enemy: true, creature: true, rebel: true, civilian: true },
    items: true,
    roomNumbers: false,
  },
  floor: 'ground',
  lighting: 'day',
};

/** The terrain modes: ground textures, roads and cliffs, or objects (debris, trees, rocks, junk) */
export type TerrainModeName = 'terrain' | 'objects';

export type EditorBrush =
  /** `size`×`size` tiles around the cursor, or the circle inside them */
  | { kind: 'square' | 'circle'; size: number }
  | { kind: 'line'; width: number }
  /** `fill` paints the connected area of the clicked texture, only for the terrain */
  | { kind: 'area' | 'fill' };

/** A ground texture, by the name of its tile type, and how often it is painted */
export interface TextureWeight {
  typeName: string;
  weight: number;
}

/** A tile of a palette and how often it is picked */
export interface TileSelection {
  tileId: number;
  count: number;
}

/** The tools of the terrain mode, see `TerrainTool` in `src-map-editor` */
export type TerrainTool = 'ground' | 'roads' | 'cliffs';

/** Paints ground textures or places road macros or cliff pieces on the clicked tile */
export interface TerrainMode {
  mode: 'terrain';
  tool: TerrainTool;
  /** Removes the ground textures, roads or cliffs instead */
  erase: boolean;
  /** The brush of the ground tool, roads and cliffs ignore it */
  brush: EditorBrush;
  /** Chance of a placement on each tile of the brush, in percent, ground textures ignore it */
  density: number;
  ground: {
    /**
     * Name of the tile type of the texture painted while no texture has a weight, one of the
     * ground palette
     */
    selected: string | null;
    /** The textures with a weight */
    weights: TextureWeight[];
  };
  /** The road macro of the roads tool, the index in the roads palette */
  road: number;
  /** The sub index of the cliff piece of the cliffs tool, see the cliffs palette */
  cliff: number;
}

/** Places debris, trees, rocks, other junk and furniture */
export interface ObjectsMode {
  mode: 'objects';
  /** Removes debris and structures instead */
  erase: boolean;
  /** Any brush but `fill` */
  brush: EditorBrush;
  /** Chance of a placement on each tile of the brush, in percent */
  density: number;
  /** The objects picked at random, by their counts */
  objects: TileSelection[];
}

/** The tools of the buildings mode, see `BuildingTool` in `src-map-editor` */
export type BuildingTool =
  /**
   * Single tiles of the walls, doors, windows, roofs, broken walls, decals and floors palettes,
   * each in the layer of its piece. Decals only go onto walls and fences. Erasing clears the
   * layers of the selected kinds.
   */
  | 'tiles'
  /**
   * Places what the smart value says: a wall that picks its piece from the walls around it,
   * doors, windows and broken walls that replace the walls of the tile, or a roof style that
   * replaces the roof of the clicked building. Erasing restores doors, windows and broken walls
   * and removes plain walls.
   */
  | 'smart'
  /** A new room of the room palette, or an extension of the building it touches */
  | 'room'
  /**
   * Selects the clicked building. Delete removes it, dragging moves it and Ctrl+dragging (Cmd on
   * macOS) copies it.
   */
  | 'select'
  /** Gives the tiles of the brush the room number */
  | 'roomNumber'
  /** Fills the area with rock on cave maps, the right button carves caves */
  | 'caves'
  /** Edits the lock and trap of the clicked door */
  | 'doorLocks';

/** The tiles of the building palettes that the buildings mode picks from, by their counts */
export interface BuildingSelections {
  /** The walls, floors and roofs of new rooms */
  room: TileSelection[];
  /** The single tiles of the tiles tool, of any of its palettes */
  tiles: TileSelection[];
}

/**
 * What the smart tool places: a tile of the smart palettes, whose piece tells whether it is a
 * wall, door, window, broken wall or roof (which replaces the roof of the clicked building), or
 * a hole in the wall
 */
export type SmartValue = { kind: 'tile'; tileId: number } | { kind: 'hole' };

export interface BuildingMode {
  mode: 'buildings';
  tool: BuildingTool;
  /** Removes what the tool places, for the single tiles and rooms */
  erase: boolean;
  /**
   * Only used by the tiles when only floors are selected, the rooms and erasing, the others use
   * the clicked tile
   */
  brush: EditorBrush;
  selections: BuildingSelections;
  smart: SmartValue | null;
  /** The room the room number tool paints, 0 is no room */
  roomNumber: number;
}

/** What a soldier does until it is alerted (`bOrders`) */
export const SOLDIER_ORDERS = [
  'stationary',
  'onGuard',
  'closePatrol',
  'farPatrol',
  'pointPatrol',
  'onCall',
  'seekEnemy',
  'randomPointPatrol',
] as const;
export type SoldierOrders = (typeof SOLDIER_ORDERS)[number];

/** How a soldier fights (`bAttitude`) */
export const SOLDIER_ATTITUDES = [
  'defensive',
  'braveSolo',
  'braveAid',
  'cunningSolo',
  'cunningAid',
  'aggressive',
  'attackSlayOnly',
] as const;
export type SoldierAttitude = (typeof SOLDIER_ATTITUDES)[number];

/** The directions of the game, clockwise from north */
export const DIRECTIONS = [
  'north',
  'northEast',
  'east',
  'southEast',
  'south',
  'southWest',
  'west',
  'northWest',
] as const;
export type Direction = (typeof DIRECTIONS)[number];

/** What new soldiers get, the editor keeps the values last set for a soldier */
export interface SoldierDefaults {
  orders: SoldierOrders;
  attitude: SoldierAttitude;
  direction: Direction;
  /** 0 to 4, from bad to great */
  relativeEquipmentLevel: number;
  relativeAttributeLevel: number;
  /** The class of new enemies, one of `enemyClasses` of the soldier palette */
  soldierClass: number;
  /** The civilian group of new civilians */
  civilianGroup: number;
  /** The body type of new creatures */
  creatureBody: number;
}

/** The tools of the soldiers mode, see `SoldierTool` in `src-map-editor` */
export type SoldierTool = 'select' | 'place';

/**
 * Places and edits soldiers: the `place` tool places a soldier of `team` with a left click, or
 * erases the soldiers of the teams shown by `MapLayers.soldierTeams` on the tiles of the brush.
 * The `select` tool selects the clicked soldier of those teams, dragging moves it, Ctrl+dragging
 * (Cmd+dragging on macOS) copies it and Delete removes it.
 */
export interface SoldierMode {
  mode: 'soldiers';
  tool: SoldierTool;
  /** Removes the soldiers of the shown teams on the tiles of the brush instead of placing them */
  erase: boolean;
  /** The brush that erases soldiers */
  brush: EditorBrush;
  team: SoldierTeam;
  defaults: SoldierDefaults;
}

/** What a click in the items mode places, see `ItemChoice` in `src-map-editor` */
export type ItemChoice =
  /** An item of the weapons, ammo, armour, explosives or equipment */
  | { kind: 'item'; item: number }
  /** A slot of the triggers: even slots below 26 are triggers, the others actions */
  | { kind: 'trigger'; slot: number }
  /** The key of a lock, `item` is the key item of the lock */
  | { kind: 'key'; lockId: number; item: number };

/** The tools of the items mode, see `ItemTool` in `src-map-editor` */
export type ItemTool = 'select' | 'place';

/**
 * Places and edits world items: the `place` tool places the chosen item with a left click, or
 * erases the items of the brush. The `select` tool selects the clicked tile with all its items,
 * dragging moves them, Ctrl+dragging (Cmd+dragging on macOS) copies them and Delete removes them.
 */
export interface ItemMode {
  mode: 'items';
  tool: ItemTool;
  /** Removes the items of the brush instead of placing them */
  erase: boolean;
  /** The brush that erases items */
  brush: EditorBrush;
  /** `null` places nothing */
  choice: ItemChoice | null;
  /** Trap level of new action items, 0 to 20 */
  trapLevel: number;
  /** What new action items do, an index of `ACTION_ITEMS` */
  action: number;
}

/** When a light is on, see `LightTime` in `src-map-editor` */
export const LIGHT_TIMES = ['primeTime', 'night', 'always'] as const;
export type LightTime = (typeof LIGHT_TIMES)[number];

/** The radii of the light templates the editor places (`L-R01.LHT` to `L-R08.LHT`) */
export const LIGHT_RADII = [1, 2, 3, 4, 5, 6, 7, 8] as const;

/** The tools of the lights mode, see `LightTool` in `src-map-editor` */
export type LightTool = 'select' | 'place';

/**
 * Places and edits lights: the `place` tool places a light with a left click, replacing the
 * lights of the tile, or erases the lights of the brush. The `select` tool selects the clicked
 * light, dragging moves it, Ctrl+dragging (Cmd+dragging on macOS) copies it and Delete removes it.
 */
export interface LightMode {
  mode: 'lights';
  tool: LightTool;
  /** Removes the lights of the brush instead of placing them */
  erase: boolean;
  /** The brush that erases lights */
  brush: EditorBrush;
  /** Radius of new lights, 1 to 8 */
  radius: number;
  /** When new lights are on */
  time: LightTime;
}

/** The entry points of a map, see `EntryPoint` in `src-map-editor` */
export const ENTRY_POINTS = [
  'north',
  'east',
  'south',
  'west',
  'center',
  'isolated',
] as const;
export type EntryPoint = (typeof ENTRY_POINTS)[number];

/** The sides of a map that have edge points */
export const EDGE_SIDES = ['north', 'east', 'south', 'west'] as const;
export type EdgeSide = (typeof EDGE_SIDES)[number];

/** A sector: `x` is the column 1 to 16, `y` the row 1 (A) to 16 (P), `z` the level 0 to 3 */
export interface SectorId {
  x: number;
  y: number;
  z: number;
}

/** The tools of the traversal mode, see `TraversalTool` in `src-map-editor` */
export type TraversalTool = 'select' | 'place';

/** What the place tool of the traversal mode places, see `TraversalPlace` in `src-map-editor` */
export type TraversalPlace = 'exit' | 'entry';

/**
 * Exit grids and entry points: the `select` tool selects, moves, copies and deletes them, the
 * `place` tool paints exit grids to the destination with the brush (right click copies the
 * values of an exit grid) or sets the `entry` point with a click
 */
export interface TraversalMode {
  mode: 'traversal';
  tool: TraversalTool;
  place: TraversalPlace;
  /** Removes exit grids */
  erase: boolean;
  /** Square sizes or areas */
  brush: EditorBrush;
  destSector: SectorId;
  destGridNo: number;
  entry: EntryPoint;
}

/** What the mouse does in the viewer, see `EditorMode` in `src-map-editor/src/editor/mode.rs` */
export type EditorMode =
  | { mode: 'view' }
  | TerrainMode
  | ObjectsMode
  | BuildingMode
  | SoldierMode
  | ItemMode
  | LightMode
  | TraversalMode;

const paletteEntrySchema = z.object({
  tileId: z.number().int(),
  /** Name of the tile type, e.g. `TEXTURE1` */
  typeName: z.string(),
  subIndex: z.number().int(),
  /** The image file, relative to the data root */
  file: z.string(),
  subImage: z.number().int(),
});

/** The palettes of the buildings mode, see `BuildingPalette` in `src-map-editor` */
const buildingPalettesSchema = z.object({
  /** The first wall, floor, roof and slant roof of each type, for new rooms */
  room: z.array(paletteEntrySchema),
  walls: z.array(paletteEntrySchema),
  doors: z.array(paletteEntrySchema),
  windows: z.array(paletteEntrySchema),
  /** Flat roofs, slant roofs, A-frames and on roofs */
  roofs: z.array(paletteEntrySchema),
  /** Decorations, broken, cracked and smudged walls */
  brokenWalls: z.array(paletteEntrySchema),
  /** Wall decals and switches */
  decals: z.array(paletteEntrySchema),
  floors: z.array(paletteEntrySchema),
  /** A wall of each wall type */
  smartWalls: z.array(paletteEntrySchema),
  /** The closed and the open door of each door type */
  smartDoors: z.array(paletteEntrySchema),
  /** The 3 window styles */
  smartWindows: z.array(paletteEntrySchema),
  /** The 2 broken walls, the cracked and the smudged wall */
  smartBrokenWalls: z.array(paletteEntrySchema),
  /** A center piece of each flat roof style */
  smartRoofs: z.array(paletteEntrySchema),
});

const paletteEventSchema = z.object({
  type: z.literal('palette'),
  map: z.string(),
  tileset: z.number().int(),
  ground: z.array(paletteEntrySchema),
  roads: z.array(paletteEntrySchema),
  cliffs: z.array(paletteEntrySchema),
  debris: z.array(paletteEntrySchema),
  trees: z.array(paletteEntrySchema),
  rocks: z.array(paletteEntrySchema),
  misc: z.array(paletteEntrySchema),
  /** `ISTRUCT1` to `ISTRUCT4` and `FIRSTCISTRUCT` */
  furniture: z.array(paletteEntrySchema),
  /** `FIFTHISTRUCT` to `EIGHTISTRUCT` */
  genericFurniture: z.array(paletteEntrySchema),
  buildings: buildingPalettesSchema,
});

/**
 * How the editor smooths the map: basements are carved out of rock, caves are rock pieces that
 * follow their neighbours
 */
const mapSmoothingSchema = z.enum(['normal', 'basement', 'caves']);

const stateEventSchema = z.object({
  type: z.literal('state'),
  map: z.string(),
  canUndo: z.boolean(),
  undoDepth: z.number().int(),
  canRedo: z.boolean(),
  redoDepth: z.number().int(),
  /** Name of the tile type of the background texture */
  background: z.string(),
  /** One more than the highest room number of the map, the number of a new room */
  nextRoomNumber: z.number().int(),
  smoothing: mapSmoothingSchema,
  /** Whether the game only scrolls over the middle of the map, like in basements */
  restrictedScroll: z.boolean(),
  /** Counts the changes of the map since it was loaded, see `saveMap` */
  revision: z.number().int(),
});

/** Trap types of doors, see `DoorTrapKind` in `src-map-editor` */
export const DOOR_TRAP_KINDS = [
  'explosion',
  'electric',
  'siren',
  'silentAlarm',
  'brothelSiren',
  'superElectric',
] as const;

/** The lock and the trap of a door */
const doorLockSchema = z.object({
  /** The lock of the lock table, 0 is none */
  lockId: z.number().int(),
  trap: z
    .object({
      kind: z.enum(DOOR_TRAP_KINDS),
      /** How hard the trap is to find, 1 to 20 */
      level: z.number().int(),
    })
    .nullable(),
  locked: z.boolean(),
});

/** Asks to edit a clicked door, `door` is `null` if it has no lock */
const doorEventSchema = z.object({
  type: z.literal('door'),
  map: z.string(),
  gridNo: z.number().int(),
  door: doorLockSchema.nullable(),
});

/** An item stack, see `ItemStackValue` in `src-map-editor` */
const itemStackSchema = z.object({
  item: z.number().int(),
  number: z.number().int(),
  /** Item class specific data, the status of most items is the first byte */
  data: z.array(z.number().int()),
  attachments: z.array(
    z.object({ item: z.number().int(), status: z.number().int() }),
  ),
  /** `OBJECT_UNDROPPABLE` (0x01), `OBJECT_NO_OVERWRITE` (0x80) and others */
  flags: z.number().int(),
  mission: z.number().int(),
  trap: z.number().int(),
  imprintId: z.number().int(),
  weight: z.number().int(),
  used: z.number().int(),
});

/** Attributes and skills, `null` if the game chooses them */
const soldierStatsSchema = z.object({
  lifeMax: z.number().int().nullable(),
  life: z.number().int().nullable(),
  agility: z.number().int().nullable(),
  dexterity: z.number().int().nullable(),
  expLevel: z.number().int().nullable(),
  marksmanship: z.number().int().nullable(),
  medical: z.number().int().nullable(),
  mechanical: z.number().int().nullable(),
  explosive: z.number().int().nullable(),
  leadership: z.number().int().nullable(),
  strength: z.number().int().nullable(),
  wisdom: z.number().int().nullable(),
  morale: z.number().int().nullable(),
  aiMorale: z.number().int().nullable(),
});

/** The fixed values of a detailed placement */
const detailedPlacementSchema = z.object({
  /** The profile of a named character */
  profile: z.number().int().nullable(),
  copyProfileItemsOver: z.boolean(),
  stats: soldierStatsSchema,
  /** The 19 inventory slots of the game, `null` for empty ones */
  inventory: z.array(itemStackSchema.nullable()),
  /** Whether the soldier keeps the colours */
  visible: z.boolean(),
  headPalette: z.string(),
  pantsPalette: z.string(),
  vestPalette: z.string(),
  skinPalette: z.string(),
});

/** The values of a soldier that `setSoldier` edits */
const soldierFieldsSchema = z.object({
  orders: z.enum(SOLDIER_ORDERS),
  attitude: z.enum(SOLDIER_ATTITUDES),
  direction: z.enum(DIRECTIONS),
  relativeAttributeLevel: z.number().int(),
  relativeEquipmentLevel: z.number().int(),
  soldierClass: z.number().int(),
  civilianGroup: z.number().int(),
  /** The body type, `null` if it is chosen at random */
  body: z.number().int().nullable(),
  priorityExistence: z.boolean(),
  hasKeys: z.boolean(),
  /** `null` without a detailed placement */
  detailed: detailedPlacementSchema.nullable(),
});

const paletteNamesSchema = z.object({
  head: z.string(),
  skin: z.string(),
  vest: z.string(),
  pants: z.string(),
});

const soldierSchema = soldierFieldsSchema.extend({
  id: z.number().int(),
  /** The number of the soldier in the game */
  slot: z.number().int().nullable(),
  /** `null` for the teams of the player */
  team: z.enum(SOLDIER_TEAMS).nullable(),
  gridNo: z.number().int(),
  onRoof: z.boolean(),
  /** The body type it is drawn with, the random one if `body` is `null` */
  drawnBody: z.number().int(),
  /** The colours it is drawn with, `null` if its body has no colours to choose */
  drawnPalette: paletteNamesSchema.nullable(),
  /** The grid numbers of its patrol points */
  patrol: z.array(z.number().int()),
});

/** The actions of schedules (`SCHEDULE_ACTION_*`) */
export const SCHEDULE_ACTIONS = [
  'none',
  'lockDoor',
  'unlockDoor',
  'openDoor',
  'closeDoor',
  'gridNo',
  'leaveSector',
  'enterSector',
  'stayInSector',
  'sleep',
  'wake',
] as const;
export type ScheduleAction = (typeof SCHEDULE_ACTIONS)[number];

/** An action of a schedule, see `ScheduleRow` in `src-map-editor` */
const scheduleRowSchema = z.object({
  action: z.enum(SCHEDULE_ACTIONS),
  /** Minutes since midnight, `null` without a time */
  time: z.number().int().nullable(),
  /** The time varies by up to 15 minutes */
  variance: z.boolean(),
  /** The tile next to the door or the tile it goes to (A) */
  data1: z.number().int().nullable(),
  /** Where it goes after the door (B) */
  data2: z.number().int().nullable(),
});

/** The tile of an action of a schedule that the next click picks */
const schedulePickSchema = z.object({
  slot: z.number().int(),
  point: z.enum(['a', 'b']),
  /** Only this tile, otherwise the tile next to a door goes on with tile B */
  single: z.boolean(),
});

const teamCountsSchema = z.object({
  enemy: z.number().int(),
  creature: z.number().int(),
  rebel: z.number().int(),
  civilian: z.number().int(),
  /** All soldiers of the map, also those of other teams */
  total: z.number().int(),
});

/** The selected soldier, `null` if none is, sent whenever it changes */
const soldierEventSchema = z.object({
  type: z.literal('soldier'),
  map: z.string(),
  soldier: soldierSchema.nullable(),
  counts: teamCountsSchema,
  /** The 4 actions of the schedule of the selected soldier, `null` if it isn't detailed */
  schedule: z.array(scheduleRowSchema).length(4).nullable(),
  /** The tile of the schedule that the next click picks */
  pick: schedulePickSchema.nullable(),
});

/** Whether an item can be seen (`Visibility`) */
export const ITEM_VISIBILITIES = [
  'hiddenItem',
  'buried',
  'hiddenInObject',
  'invisible',
  'unseen',
  'visible',
] as const;
export type ItemVisibility = (typeof ITEM_VISIBILITIES)[number];

/** A world item, see `ItemValue` in `src-map-editor` */
const worldItemSchema = z.object({
  id: z.number().int(),
  gridNo: z.number().int(),
  onRoof: z.boolean(),
  stack: itemStackSchema,
  visibility: z.enum(ITEM_VISIBILITIES),
  /** `WorldItemFlags`, e.g. `REALISTIC_ONLY` (0x100) and `SCIFI_ONLY` (0x80) */
  flags: z.number().int(),
  /** Chance of the item not existing in the game, in percent */
  nonExistChance: z.number().int(),
});

/** How many items of each choice of the palette the map has */
const itemCountsSchema = z.object({
  items: z.array(
    z.object({
      item: z.number().int(),
      placed: z.number().int(),
      /** The items in the stacks */
      quantity: z.number().int(),
    }),
  ),
  /** By trigger slot */
  triggers: z.array(z.number().int()),
  keys: z.array(
    z.object({ lockId: z.number().int(), placed: z.number().int() }),
  ),
});

/** The selected tile and its items, sent whenever they or the items of the map change */
const itemEventSchema = z.object({
  type: z.literal('item'),
  map: z.string(),
  /** The selected tile, `null` if none is */
  tile: z.number().int().nullable(),
  /** The items shown on the selected tile, in the order of the map */
  items: z.array(worldItemSchema),
  counts: itemCountsSchema,
});

/** A light of the map, see `LightValue` in `src-map-editor` */
const mapLightSchema = z.object({
  id: z.number().int(),
  gridNo: z.number().int(),
  x: z.number().int(),
  y: z.number().int(),
  /** `null` for templates that are not `L-R0N.LHT` */
  radius: z.number().int().nullable(),
  /** File name of the template */
  template: z.string(),
  time: z.enum(LIGHT_TIMES),
});

/** The ambient light of basements and caves */
const ambientLightSchema = z.object({
  basement: z.boolean(),
  caves: z.boolean(),
  /** The light level of the original editor, 1 to 15, higher is brighter */
  level: z.number().int(),
});

/** The selected light and the lighting of the map, sent whenever they change */
const lightEventSchema = z.object({
  type: z.literal('light'),
  map: z.string(),
  light: mapLightSchema.nullable(),
  /** The number of lights of the map */
  count: z.number().int(),
  /** The light colour as red, green and blue */
  color: z.tuple([z.number().int(), z.number().int(), z.number().int()]),
  /** `null` for maps that are neither basements nor caves */
  ambient: ambientLightSchema.nullable(),
  /** The radii whose templates exist, only these can be placed */
  radii: z.array(z.number().int()),
});

const sectorSchema = z.object({
  x: z.number().int(),
  y: z.number().int(),
  z: z.number().int(),
});

/** An exit grid of the map, see `ExitGridValue` in `src-map-editor` */
const exitGridSchema = z.object({
  gridNo: z.number().int(),
  destSector: sectorSchema,
  destGridNo: z.number().int(),
});

/** The selected exit grid or entry point, see `SelectedTraversalValue` in `src-map-editor` */
const selectedTraversalSchema = z.discriminatedUnion('kind', [
  exitGridSchema.extend({ kind: z.literal('exitGrid') }),
  z.object({
    kind: z.literal('entry'),
    point: z.enum(ENTRY_POINTS),
    gridNo: z.number().int(),
  }),
]);

/** The number of edge points of each side */
const sideCountsSchema = z.object({
  north: z.number().int(),
  east: z.number().int(),
  south: z.number().int(),
  west: z.number().int(),
});

/** The exit grids, the entry points and the edge points, sent whenever they change */
const traversalEventSchema = z.object({
  type: z.literal('traversal'),
  map: z.string(),
  /** The number of exit grids of the map */
  exitGrids: z.number().int(),
  /** The exit grid or entry point the select tool selected */
  selected: selectedTraversalSchema.nullable(),
  /** The exit grid that was right clicked last */
  queried: exitGridSchema.nullable(),
  /** Grid numbers, `null` for the entry points that are not set */
  entryPoints: z.object({
    north: z.number().int().nullable(),
    east: z.number().int().nullable(),
    south: z.number().int().nullable(),
    west: z.number().int().nullable(),
    center: z.number().int().nullable(),
    isolated: z.number().int().nullable(),
  }),
  edgePoints: z.object({
    primary: sideCountsSchema,
    secondary: sideCountsSchema,
    /** The sides with an entry point but without edge points */
    missing: z.array(z.enum(EDGE_SIDES)),
    /** Whether they are being generated again after edits */
    pending: z.boolean(),
  }),
});

/** The floor after Page Up or Page Down switched it in the viewer */
const floorEventSchema = z.object({
  type: z.literal('floor'),
  map: z.string(),
  floor: z.enum(['ground', 'roof']),
});

/** Why an edit did not happen, e.g. a soldier could not be placed */
const noticeEventSchema = z.object({
  type: z.literal('notice'),
  map: z.string(),
  message: z.string(),
});

const bodyOptionSchema = z.object({
  /** `null` for a random body type */
  id: z.number().int().nullable(),
  name: z.string(),
});

const byTeam = <T extends z.ZodTypeAny>(schema: T) =>
  z.object({
    enemy: schema,
    creature: schema,
    rebel: schema,
    civilian: schema,
  });

/** The choices of the soldiers panel, sent once a map is loaded */
const soldierPaletteEventSchema = z.object({
  type: z.literal('soldierPalette'),
  map: z.string(),
  maxSoldiers: z.number().int(),
  maxTeamSoldiers: z.number().int(),
  enemyClasses: z.array(z.object({ id: z.number().int(), name: z.string() })),
  /** The body types the soldiers of each team can have */
  bodies: byTeam(z.array(bodyOptionSchema)),
  civilianGroups: z.array(z.string()),
  /** The palette names of the colour sets */
  colours: z.object({
    head: z.array(z.string()),
    skin: z.array(z.string()),
    vest: z.array(z.string()),
    pants: z.array(z.string()),
  }),
});

const editorEventSchema = z.discriminatedUnion('type', [
  paletteEventSchema,
  stateEventSchema,
  doorEventSchema,
  soldierEventSchema,
  noticeEventSchema,
  soldierPaletteEventSchema,
  itemEventSchema,
  lightEventSchema,
  traversalEventSchema,
  floorEventSchema,
]);

export type PaletteEntry = z.infer<typeof paletteEntrySchema>;
export type EditorPaletteEvent = z.infer<typeof paletteEventSchema>;
export type BuildingPalettes = z.infer<typeof buildingPalettesSchema>;
export type EditorStateEvent = z.infer<typeof stateEventSchema>;
export type EditorEvent = z.infer<typeof editorEventSchema>;
export type MapSmoothing = z.infer<typeof mapSmoothingSchema>;
/** What a new map looks like, see `createMap` */
export type NewMapOptions = { tileset: number; smoothing: MapSmoothing };
export type DoorLock = z.infer<typeof doorLockSchema>;
export type DoorTrapKind = (typeof DOOR_TRAP_KINDS)[number];
export type EditorDoorEvent = z.infer<typeof doorEventSchema>;
export type ItemStack = z.infer<typeof itemStackSchema>;
export type SoldierStats = z.infer<typeof soldierStatsSchema>;
export type DetailedPlacement = z.infer<typeof detailedPlacementSchema>;
export type SoldierFields = z.infer<typeof soldierFieldsSchema>;
export type Soldier = z.infer<typeof soldierSchema>;
export type TeamCounts = z.infer<typeof teamCountsSchema>;
export type EditorSoldierEvent = z.infer<typeof soldierEventSchema>;
export type EditorNoticeEvent = z.infer<typeof noticeEventSchema>;
export type SoldierPaletteEvent = z.infer<typeof soldierPaletteEventSchema>;
export type BodyOption = z.infer<typeof bodyOptionSchema>;
export type ScheduleRow = z.infer<typeof scheduleRowSchema>;
export type SchedulePick = z.infer<typeof schedulePickSchema>;
export type WorldItem = z.infer<typeof worldItemSchema>;
export type ItemCounts = z.infer<typeof itemCountsSchema>;
export type EditorItemEvent = z.infer<typeof itemEventSchema>;
export type MapLight = z.infer<typeof mapLightSchema>;
export type AmbientLight = z.infer<typeof ambientLightSchema>;
export type EditorLightEvent = z.infer<typeof lightEventSchema>;
export type ExitGrid = z.infer<typeof exitGridSchema>;
export type SelectedTraversal = z.infer<typeof selectedTraversalSchema>;
export type EdgePointCounts = z.infer<typeof sideCountsSchema>;
export type EditorTraversalEvent = z.infer<typeof traversalEventSchema>;
export type EditorFloorEvent = z.infer<typeof floorEventSchema>;

/** Validates an event of the viewer, `null` if it is invalid */
export function parseEditorEvent(event: unknown): EditorEvent | null {
  const result = editorEventSchema.safeParse(event);
  if (!result.success) {
    console.error('Invalid event of the map viewer', result.error);
    return null;
  }
  return result.data;
}

type Viewer = typeof import('ja2-map-editor');
type StatusListener = (status: MapViewerStatus) => void;
type EditorEventListener = (event: EditorEvent) => void;

const CANVAS_ID = 'ja2-map-viewer';

let started: Promise<Viewer> | null = null;
let canvas: HTMLCanvasElement | null = null;
let holder: HTMLDivElement | null = null;
let attachedTo: HTMLElement | null = null;
let removeKeyListeners: (() => void) | null = null;
const listeners = new Set<StatusListener>();
const editorListeners = new Set<EditorEventListener>();
// Sent again after loading a map
let currentMode: EditorMode = { mode: 'view' };

// Resolves to null if the file does not exist
async function readFile(path: string): Promise<Uint8Array | null> {
  const base64 = await invoke('resources/read', { file: path });
  if (base64 === null) {
    return null;
  }
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}

function toBase64(bytes: Uint8Array): string {
  // `String.fromCharCode` takes the bytes as arguments, so large arrays are converted in chunks
  const CHUNK_SIZE = 0x8000;
  let binary = '';
  for (let i = 0; i < bytes.length; i += CHUNK_SIZE) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK_SIZE));
  }
  return btoa(binary);
}

function filesExist(paths: string[]): Promise<boolean[]> {
  return invoke('resources/exists', { files: paths });
}

function onStatus(status: unknown) {
  for (const listener of listeners) {
    listener(status as MapViewerStatus);
  }
}

function onEditorEvent(event: unknown) {
  const parsed = parseEditorEvent(event);
  if (!parsed) {
    return;
  }
  for (const listener of editorListeners) {
    listener(parsed);
  }
}

function ensureStarted(): Promise<Viewer> {
  if (!started) {
    started = (async () => {
      const viewer = await import('ja2-map-editor');
      await viewer.default();

      // Keep the canvas tiny while hidden, so the app has nothing to render
      holder = document.createElement('div');
      holder.style.position = 'fixed';
      holder.style.left = '-10px';
      holder.style.top = '-10px';
      holder.style.width = '1px';
      holder.style.height = '1px';
      holder.style.overflow = 'hidden';
      canvas = document.createElement('canvas');
      canvas.id = CANVAS_ID;
      // The mouse wheel zooms the map, it should not scroll the page
      canvas.addEventListener('wheel', (e) => e.preventDefault(), {
        passive: false,
      });
      // The right button edits with some building tools
      canvas.addEventListener('contextmenu', (e) => e.preventDefault());
      holder.appendChild(canvas);
      document.body.appendChild(holder);

      viewer.start(
        `#${CANVAS_ID}`,
        readFile,
        filesExist,
        onStatus,
        onEditorEvent,
      );
      return viewer;
    })();
    // Allow retrying when starting failed
    started.catch(() => {
      started = null;
    });
  }
  return started;
}

/** Input types the user types text into */
const TEXT_INPUT_TYPES = new Set([
  'text',
  'search',
  'email',
  'password',
  'tel',
  'url',
  'number',
  'date',
  'datetime-local',
  'month',
  'time',
  'week',
]);

/** Whether keyboard input on `target` is meant for it, i.e. typing into a text input */
export function isEditableTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  if (target.isContentEditable) {
    return true;
  }
  if (target instanceof HTMLTextAreaElement) {
    return !target.readOnly;
  }
  // Checkboxes, radios (e.g. Segmented) and sliders keep the focus after clicking them, but
  // don't take typed text. Neither do readonly inputs, such as the one of a non-searchable Select
  if (target instanceof HTMLInputElement) {
    return !target.readOnly && TEXT_INPUT_TYPES.has(target.type);
  }
  return false;
}

// The canvas only receives keyboard input while it is focused, so keys pressed elsewhere on the
// page are forwarded to the viewer while it is shown
function listenToKeys(viewer: Viewer): () => void {
  const onKey = (e: KeyboardEvent) => {
    // The viewer receives these itself
    if (e.target === canvas) {
      return;
    }
    const pressed = e.type === 'keydown';
    // Releases are always forwarded, so that no key gets stuck. The modifiers themselves are
    // forwarded too (e.g. Ctrl or Cmd+drag copies buildings, soldiers and lights), combinations
    // like Ctrl+Z are not.
    const modifier = ['Control', 'Meta', 'Alt', 'Shift'].includes(e.key);
    if (
      pressed &&
      (isEditableTarget(e.target) ||
        (!modifier && (e.ctrlKey || e.metaKey || e.altKey)))
    ) {
      return;
    }
    viewer.key_event(e.code, e.key, pressed, e.repeat);
  };
  // A closed non-searchable Select keeps the focus after picking an option and opens on almost
  // any key. Instead, the key is handled as if it was pressed while nothing is focused.
  const onSelectKey = (e: KeyboardEvent) => {
    const target = e.target;
    if (
      e.key === 'Tab' ||
      !(target instanceof HTMLInputElement) ||
      !target.readOnly ||
      target.getAttribute('role') !== 'combobox' ||
      target.getAttribute('aria-expanded') === 'true'
    ) {
      return;
    }
    e.stopPropagation();
    e.preventDefault();
    target.blur();
    document.body.dispatchEvent(new KeyboardEvent(e.type, e));
  };
  const onBlur = () => viewer.release_keys();
  window.addEventListener('keydown', onSelectKey, true);
  window.addEventListener('keydown', onKey);
  window.addEventListener('keyup', onKey);
  window.addEventListener('blur', onBlur);
  return () => {
    window.removeEventListener('keydown', onSelectKey, true);
    window.removeEventListener('keydown', onKey);
    window.removeEventListener('keyup', onKey);
    window.removeEventListener('blur', onBlur);
    viewer.release_keys();
  };
}

/** Shows the viewer inside `container`, starting it if necessary */
export async function attach(container: HTMLElement): Promise<void> {
  attachedTo = container;
  const viewer = await ensureStarted();
  // The page might have been left while starting
  if (attachedTo !== container || !canvas) {
    return;
  }
  viewer.set_active(true);
  removeKeyListeners?.();
  removeKeyListeners = listenToKeys(viewer);
  container.appendChild(canvas);
}

/** Hides the viewer again */
export function detach(): void {
  attachedTo = null;
  removeKeyListeners?.();
  removeKeyListeners = null;
  if (canvas && holder && canvas.parentElement !== holder) {
    holder.appendChild(canvas);
  }
  if (started) {
    started
      .then((viewer) => viewer.set_active(false))
      .catch(() => {
        // Errors are reported by `attach`
      });
  }
}

/** Loads the map at `path` (e.g. `maps/A9.dat`) from the selected mod or vanilla data */
export async function loadMap(path: string): Promise<void> {
  const viewer = await ensureStarted();
  viewer.load_map(path);
  viewer.set_editor_mode(JSON.stringify(currentMode));
}

/**
 * Opens a new map that is saved to `path` (e.g. `maps/A9.dat`), like the original editor creates
 * it: random textures, roofed everywhere for basements and rock everywhere for caves. Nothing is
 * written until it is saved.
 */
export async function createMap(
  path: string,
  options: NewMapOptions,
): Promise<void> {
  const viewer = await ensureStarted();
  viewer.new_map(path, JSON.stringify(options));
  viewer.set_editor_mode(JSON.stringify(currentMode));
}

const tilesetListSchema = z.object({
  default: z.number().int(),
  tilesets: z.array(z.object({ id: z.number().int(), name: z.string() })),
});

export type TilesetList = z.infer<typeof tilesetListSchema>;

/** The tilesets of the selected mod or vanilla data, and the one the others fall back to */
export async function listTilesets(): Promise<TilesetList> {
  const viewer = await ensureStarted();
  return tilesetListSchema.parse(await viewer.tilesets());
}

/** Sets the layers and the lighting that the viewer shows, they are kept when loading maps */
export async function setRenderOptions(
  options: MapRenderOptions,
): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_render_options(JSON.stringify(options));
}

/** Subscribes to the loading status of maps, returns a function to unsubscribe */
export function subscribeStatus(listener: StatusListener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Sets what the mouse does in the viewer, it is kept when loading maps */
export async function setEditorMode(mode: EditorMode): Promise<void> {
  currentMode = mode;
  const viewer = await ensureStarted();
  viewer.set_editor_mode(JSON.stringify(mode));
}

/** Undoes the last edit of the loaded map */
export async function undo(): Promise<void> {
  const viewer = await ensureStarted();
  viewer.undo();
}

/** Redoes the last undone edit of the loaded map, until the map is edited again */
export async function redo(): Promise<void> {
  const viewer = await ensureStarted();
  viewer.redo();
}

/**
 * Replaces the ground of the whole loaded map by a texture, the type name of an entry of the
 * ground palette. Can be undone.
 */
export async function setMapBackground(typeName: string): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_map_background(typeName);
}

/** Sets how the editor smooths the loaded map, which decides the building tools. Can be undone. */
export async function setMapSmoothing(smoothing: MapSmoothing): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_map_smoothing(smoothing);
}

/**
 * Sets whether the game only scrolls over the middle of the loaded map, like in basements. Can be
 * undone.
 */
export async function setRestrictedScroll(
  restrictedScroll: boolean,
): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_restricted_scroll(restrictedScroll);
}

/** Sets the lock and trap of the door of a tile, `null` removes them. Can be undone. */
export async function setDoor(
  gridNo: number,
  door: DoorLock | null,
): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_door(JSON.stringify({ gridNo, door }));
}

/**
 * Gives every tile with a room a flat roof of the roofs selection of the buildings mode, the
 * ceilings of basements. Can be undone.
 */
export async function roofAllRooms(): Promise<void> {
  const viewer = await ensureStarted();
  viewer.roof_all_rooms();
}

/** The edited values of the selected soldier */
export type SoldierEdit = { id: number } & SoldierFields;

/** Edits the selected soldier, can be undone */
export async function setSoldier(soldier: SoldierEdit): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_soldier(JSON.stringify(soldier));
}

/** What the soldiers panel asks of the viewer, see `SoldierCommand` in `src-map-editor` */
export type SoldierCommand =
  | { kind: 'select'; id: number | null }
  /** Gives the selected soldier a detailed placement, with the profile or without, or removes it */
  | { kind: 'detailed'; detailed: boolean; profile?: number | null }
  /** Centers the view on a tile */
  | { kind: 'center'; gridNo: number }
  /** Sets the 4 actions of the schedule of the selected soldier */
  | { kind: 'schedule'; rows: ScheduleRow[] }
  | { kind: 'clearSchedule' }
  /** The next left click picks a tile of an action of the schedule */
  | ({ kind: 'pick' } & SchedulePick)
  /** Stops picking, an action whose tiles are missing is removed */
  | { kind: 'cancelPick' };

export async function soldierCommand(command: SoldierCommand): Promise<void> {
  const viewer = await ensureStarted();
  viewer.soldier_command(JSON.stringify(command));
}

/** The edited values of a world item, its item can't be changed */
export type ItemEdit = Pick<
  WorldItem,
  'id' | 'stack' | 'visibility' | 'flags' | 'nonExistChance'
>;

/** Edits a world item, can be undone */
export async function setItem(item: ItemEdit): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_item(JSON.stringify(item));
}

/** What the items panel asks of the viewer, see `ItemCommand` in `src-map-editor` */
export type ItemCommand =
  /** Removes an item of the selected tile, can be undone */
  | { kind: 'delete'; id: number }
  /** Centers the view on the selected tile */
  | { kind: 'center' };

export async function itemCommand(command: ItemCommand): Promise<void> {
  const viewer = await ensureStarted();
  viewer.item_command(JSON.stringify(command));
}

/** The edited radius and time of the selected light, a `radius` of `null` keeps the template */
export type LightEdit = Pick<MapLight, 'id' | 'radius' | 'time'>;

/** Edits the selected light, can be undone */
export async function setLight(light: LightEdit): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_light(JSON.stringify(light));
}

/** What the lights panel asks of the viewer, see `LightCommand` in `src-map-editor` */
export type LightCommand =
  | { kind: 'deselect' }
  /**
   * Sets the light colour and the light level of basements and caves (1 to 15), `null` keeps
   * the level
   */
  | {
      kind: 'settings';
      color: [number, number, number];
      ambientLevel: number | null;
    };

export async function lightCommand(command: LightCommand): Promise<void> {
  const viewer = await ensureStarted();
  viewer.light_command(JSON.stringify(command));
}

/** How the traversal panel edits the selection, see `TraversalEdit` in `src-map-editor` */
export type TraversalEdit =
  /** Moves an entry point, to where the game can show it */
  | { kind: 'entry'; point: EntryPoint; gridNo: number }
  /** Changes the destination of the exit grid of a tile */
  | {
      kind: 'exitGrid';
      gridNo: number;
      destSector: SectorId;
      destGridNo: number;
    };

/** Edits the selected entry point or exit grid, can be undone */
export async function setTraversal(edit: TraversalEdit): Promise<void> {
  const viewer = await ensureStarted();
  viewer.set_traversal(JSON.stringify(edit));
}

const savedMapSchema = z.object({
  map: z.string(),
  revision: z.number().int(),
  data: z.instanceof(Uint8Array),
});

/**
 * Saves the loaded map with the edits so far to `file` in the selected mod, resolves with the
 * `revision` of the state events that was saved
 */
export async function saveMap(file: string): Promise<number> {
  const viewer = await ensureStarted();
  const saved = savedMapSchema.parse(await viewer.save_map());
  if (saved.map !== file) {
    throw new Error(`The viewer saved ${saved.map} instead of ${file}`);
  }
  await invoke('resources/persist', { file, data: toBase64(saved.data) });
  return saved.revision;
}

/** Path of the radar map of a map, e.g. `radarmaps/a9_b1.sti` for `maps/A9_b1.dat` */
export function radarMapFile(mapFile: string): string {
  const name = mapFile.slice(mapFile.lastIndexOf('/') + 1);
  const base = name.includes('.') ? name.slice(0, name.lastIndexOf('.')) : name;
  return `radarmaps/${base.toLowerCase()}.sti`;
}

// Loading the map and rendering it takes seconds, this only catches a viewer that is stuck
const RADAR_MAP_TIMEOUT_MS = 120_000;

/**
 * Renders the radar map of the map `file` (e.g. `maps/A9.dat`) like the game's map utility and
 * writes it to the selected mod, resolves with the path of the radar map
 *
 * The map is loaded into the viewer for this, so the viewer must not be shown.
 */
export async function regenerateRadarMap(file: string): Promise<string> {
  if (attachedTo) {
    throw new Error('The mini map can only be generated while no map is open');
  }
  const viewer = await ensureStarted();
  viewer.load_map(file);
  let timeout: ReturnType<typeof setTimeout> | undefined;
  const timedOut = new Promise<never>((_resolve, reject) => {
    timeout = setTimeout(
      () => reject(new Error(`Rendering the mini map of ${file} timed out`)),
      RADAR_MAP_TIMEOUT_MS,
    );
  });
  try {
    const data = z
      .instanceof(Uint8Array)
      .parse(await Promise.race([viewer.render_radar_map(file), timedOut]));
    const radarFile = radarMapFile(file);
    await invoke('resources/persist', {
      file: radarFile,
      data: toBase64(data),
    });
    return radarFile;
  } finally {
    clearTimeout(timeout);
  }
}

/** Subscribes to the palettes and the state of the editor, returns a function to unsubscribe */
export function subscribeEditorEvents(
  listener: EditorEventListener,
): () => void {
  editorListeners.add(listener);
  return () => {
    editorListeners.delete(listener);
  };
}
