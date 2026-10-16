import type { BuildingPalettes } from '../../../renderer/lib/mapViewer';

/** Building palettes without any tiles, for palette events of tests */
export function emptyBuildingPalettes(): BuildingPalettes {
  return {
    room: [],
    walls: [],
    doors: [],
    windows: [],
    roofs: [],
    brokenWalls: [],
    decals: [],
    floors: [],
    smartWalls: [],
    smartDoors: [],
    smartWindows: [],
    smartBrokenWalls: [],
    smartRoofs: [],
  };
}
