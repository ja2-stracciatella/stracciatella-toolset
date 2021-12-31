import { AmmoTypes } from "./pages/AmmoTypes";
import { ArmyCompositions } from "./pages/ArmyCompositions";
import { ArmyGarrisonGroups } from "./pages/ArmyGarrisonGroups";
import { ArmyGunChoiceExtended } from "./pages/ArmyGunChoiceExtended";
import { ArmyGunChoiceNormal } from "./pages/ArmyGunChoiceNormal";
import { ArmyPatrolGroups } from "./pages/ArmyPatrolGroups";
import { Calibres } from "./pages/Calibres";
import { Dashboard } from "./pages/Dashboard";
import { Dealers } from "./pages/Dealers";
import { Game } from "./pages/Game";
import { Imp } from "./pages/Imp";
import { Items } from "./pages/Items";
import { LoadingScreens } from "./pages/LoadingScreens";
import { LoadingScreensMapping } from "./pages/LoadingScreensMapping";
import { Magazines } from "./pages/Magazines";
import { MercsMERCListings } from "./pages/MercsMERCListings";
import { MercsProfileInfo } from "./pages/MercsProfileInfo";
import { MercsRpcSmallFaces } from "./pages/MercsRpcSmallFaces";
import { Music } from "./pages/Music";
import { ShippingDestinations } from "./pages/ShippingDestinations";
import { StrategicAIPolicy } from "./pages/StrategicAIPolicy";
import { StrategicBloodcatPlacements } from "./pages/StrategicBloodcatPlacements";
import { StrategicBloodcatSpawns } from "./pages/StrategicBloodcatSpawns";
import { StrategicFactParams } from "./pages/StrategicFactParams";
import { StrategicMapCacheSectors } from "./pages/StrategicMapCacheSectors";
import { StrategicMapCreatureLairs } from "./pages/StrategicMapCreatureLairs";
import { StrategicMapNpcPlacements } from "./pages/StrategicMapNpcPlacements";
import { TacticalMapItemReplacements } from "./pages/TacticalMapItemReplacements";
import { TacticalNpcActionParams } from "./pages/TacticalNpcActionParams";
import { Vehicles } from "./pages/Vehicles";
import { Weapons } from "./pages/Weapons";

export const ROUTES = [
  {
    id: "dashboard",
    label: "Dashboard",
    url: "/",
    component: Dashboard,
  },
  {
    id: "ammo-types",
    label: "Ammo Types",
    url: "/ammo-types",
    component: AmmoTypes,
  },
  {
    id: "army-compositions",
    label: "Army Compositions",
    url: "/army-compositions",
    component: ArmyCompositions,
  },
  {
    id: "army-garrison-groups",
    label: "Army Garrison Groups",
    url: "/army-garrison-groups",
    component: ArmyGarrisonGroups,
  },
  {
    id: "army-gun-choice-extended",
    label: "Army Gun Choice Extended",
    url: "/army-gun-choice-extended",
    component: ArmyGunChoiceExtended,
  },
  {
    id: "army-gun-choice-normal",
    label: "Army Gun Choice Normal",
    url: "/army-gun-choice-normal",
    component: ArmyGunChoiceNormal,
  },
  {
    id: "army-patrol-groups",
    label: "Army Patrol Groups",
    url: "/army-patrol-groups",
    component: ArmyPatrolGroups,
  },
  {
    id: "calibres",
    label: "Calibres",
    url: "/calibres",
    component: Calibres,
  },
  {
    id: "dealers",
    label: "Dealers",
    url: "/dealers",
    component: Dealers,
  },
  {
    id: "game",
    label: "Game",
    url: "/game",
    component: Game,
  },
  {
    id: "imp",
    label: "IMP",
    url: "/imp",
    component: Imp,
  },
  {
    id: "items",
    label: "Items",
    url: "/items",
    component: Items,
  },
  {
    id: "loading-screens-mapping",
    label: "Loading Screens Mapping",
    url: "/loading-screens-mapping",
    component: LoadingScreensMapping,
  },
  {
    id: "loading-screens",
    label: "Loading Screens",
    url: "/loading-screens",
    component: LoadingScreens,
  },
  {
    id: "magazines",
    label: "Magazines",
    url: "/magazines",
    component: Magazines,
  },
  {
    id: "mercs-MERC-listings",
    label: "M.E.R.C. Listings",
    url: "/mercs-MERC-listings",
    component: MercsMERCListings,
  },
  {
    id: "mercs-profile-info",
    label: "Mercs Profile Info",
    url: "/mercs-profile-info",
    component: MercsProfileInfo,
  },
  {
    id: "mercs-rpc-small-faces",
    label: "Mercs RPC Small Faces",
    url: "/mercs-rpc-small-faces",
    component: MercsRpcSmallFaces,
  },
  {
    id: "music",
    label: "Music",
    url: "/music",
    component: Music,
  },
  {
    id: "shipping-destinations",
    label: "Shipping Destinations",
    url: "/shipping-destinations",
    component: ShippingDestinations,
  },
  {
    id: "strategic-ai-policy",
    label: "Strategic AI Policy",
    url: "/strategic-ai-policy",
    component: StrategicAIPolicy,
  },
  {
    id: "strategic-bloodcat-placements",
    label: "Strategic Bloodcat Placements",
    url: "/strategic-bloodcat-placements",
    component: StrategicBloodcatPlacements,
  },
  {
    id: "strategic-bloodcat-spawns",
    label: "Strategic Bloodcat Spawns",
    url: "/strategic-bloodcat-spawns",
    component: StrategicBloodcatSpawns,
  },
  {
    id: "strategic-fact-params",
    label: "Strategic Fact Params",
    url: "/strategic-fact-params",
    component: StrategicFactParams,
  },
  {
    id: "strategic-map-cache-sectors",
    label: "Strategic Map Cache Sectors",
    url: "/strategic-map-cache-sectors",
    component: StrategicMapCacheSectors,
  },
  {
    id: "strategic-map-creature-lairs",
    label: "Strategic Map Creature Lairs",
    url: "/strategic-map-creature-lairs",
    component: StrategicMapCreatureLairs,
  },
  {
    id: "strategic-map-npc-placements",
    label: "Strategic Map NPC Placements",
    url: "/strategic-map-npc-placements",
    component: StrategicMapNpcPlacements,
  },
  {
    id: "tactical-map-item-replacements",
    label: "Tactical Map Item Replacements",
    url: "/tactical-map-item-replacements",
    component: TacticalMapItemReplacements,
  },
  {
    id: "tactical-npc-action-params",
    label: "Tactical Npc Action Params",
    url: "/tactical-npc-action-params",
    component: TacticalNpcActionParams,
  },
  {
    id: "vehicles",
    label: "Vehicles",
    url: "/vehicles",
    component: Vehicles,
  },
  {
    id: "weapons",
    label: "Weapons",
    url: "/weapons",
    component: Weapons,
  },
];
