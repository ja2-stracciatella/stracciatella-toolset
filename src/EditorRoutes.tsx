import { useCallback } from "react";
import { stringReferenceTo } from "./components/form/StringReferenceWidget";
import { JsonForm } from "./components/JsonForm";
import { JsonItemsForm } from "./components/JsonItemsForm";
import { JsonStrategicMapForm } from "./components/StrategicMapForm";
import { Dashboard } from "./components/Dashboard";
import { MercPreview } from "./components/content/MercPreview";
import { ItemPreview } from "./components/content/ItemPreview";

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
    component: function AmmoTypes() {
      return <JsonItemsForm file="ammo-types.json" name="internalName" />;
    },
  },
  {
    id: "army-compositions",
    label: "Army Compositions",
    url: "/army-compositions",
    component: function ArmyCompositions() {
      return <JsonItemsForm file="army-compositions.json" name="name" />;
    },
  },
  {
    id: "army-garrison-groups",
    label: "Army Garrison Groups",
    url: "/army-garrison-groups",
    component: function ArmyGarrisonGroups() {
      const uiSchema = {
        composition: {
          "ui:widget": stringReferenceTo("army-compositions.json", "name"),
        },
      };
      return (
        <JsonStrategicMapForm
          file="army-garrison-groups.json"
          uiSchema={uiSchema}
        />
      );
    },
  },
  {
    id: "army-gun-choice-extended",
    label: "Army Gun Choice Extended",
    url: "/army-gun-choice-extended",
    component: function ArmyGunChoiceExtended() {
      const uiSchema = {
        items: {
          items: {
            "ui:widget": stringReferenceTo("weapons.json", "internalName"),
          },
        },
      };
      return (
        <JsonForm file="army-gun-choice-extended.json" uiSchema={uiSchema} />
      );
    },
  },
  {
    id: "army-gun-choice-normal",
    label: "Army Gun Choice Normal",
    url: "/army-gun-choice-normal",
    component: function ArmyGunChoiceNormal() {
      const uiSchema = {
        items: {
          items: {
            "ui:widget": stringReferenceTo("weapons.json", "internalName"),
          },
        },
      };
      return (
        <JsonForm file="army-gun-choice-normal.json" uiSchema={uiSchema} />
      );
    },
  },
  {
    id: "army-patrol-groups",
    label: "Army Patrol Groups",
    url: "/army-patrol-groups",
    component: function ArmyPatrolGroups() {
      const getPatrolGroupName = useCallback((item: any) => {
        return item.points.join(", ") as string;
      }, []);
      return (
        <JsonItemsForm
          file="army-patrol-groups.json"
          name={getPatrolGroupName}
        />
      );
    },
  },
  {
    id: "calibres",
    label: "Calibres",
    url: "/calibres",
    component: function Calibres() {
      return <JsonItemsForm file="calibres.json" name="internalName" />;
    },
  },
  {
    id: "dealers",
    label: "Dealers",
    url: "/dealers",
    component: function Dealers() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        []
      );
      return (
        <JsonItemsForm file="dealers.json" name="profile" preview={preview} />
      );
    },
  },
  {
    id: "game",
    label: "Game",
    url: "/game",
    component: function Game() {
      return <JsonForm file="game.json" />;
    },
  },
  {
    id: "imp",
    label: "IMP",
    url: "/imp",
    component: function Imp() {
      return <JsonForm file="imp.json" />;
    },
  },
  {
    id: "items",
    label: "Items",
    url: "/items",
    component: function Items() {
      const preview = useCallback(
        (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        []
      );
      return (
        <JsonItemsForm
          file="items.json"
          name="internalName"
          preview={preview}
        />
      );
    },
  },
  {
    id: "loading-screens-mapping",
    label: "Loading Screens Mapping",
    url: "/loading-screens-mapping",
    component: function LoadingScreensMapping() {
      return <JsonStrategicMapForm file="loading-screens-mapping.json" />;
    },
  },
  {
    id: "loading-screens",
    label: "Loading Screens",
    url: "/loading-screens",
    component: function LoadingScreens() {
      return <JsonItemsForm file="loading-screens.json" name="internalName" />;
    },
  },
  {
    id: "magazines",
    label: "Magazines",
    url: "/magazines",
    component: function Magazines() {
      const preview = useCallback(
        (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        []
      );
      return (
        <JsonItemsForm
          file="magazines.json"
          name="internalName"
          preview={preview}
        />
      );
    },
  },
  {
    id: "mercs-MERC-listings",
    label: "M.E.R.C. Listings",
    url: "/mercs-MERC-listings",
    component: function MercsMERCListings() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        []
      );
      return (
        <JsonItemsForm
          file="mercs-MERC-listings.json"
          name="profile"
          preview={preview}
        />
      );
    },
  },
  {
    id: "mercs-profile-info",
    label: "Mercs Profile Info",
    url: "/mercs-profile-info",
    component: function MercsProfileInfo() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.internalName} />,
        []
      );
      return (
        <JsonItemsForm
          file="mercs-profile-info.json"
          name="internalName"
          preview={preview}
        />
      );
    },
  },
  {
    id: "mercs-rpc-small-faces",
    label: "Mercs RPC Small Faces",
    url: "/mercs-rpc-small-faces",
    component: function MercsRpcSmallFaces() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        []
      );
      return (
        <JsonItemsForm
          file="mercs-rpc-small-faces.json"
          name="profile"
          preview={preview}
        />
      );
    },
  },
  {
    id: "music",
    label: "Music",
    url: "/music",
    component: function Music() {
      return <JsonForm file="music.json" />;
    },
  },
  {
    id: "shipping-destinations",
    label: "Shipping Destinations",
    url: "/shipping-destinations",
    component: function ShippingDestinations() {
      return (
        <JsonItemsForm file="shipping-destinations.json" name="locationId" />
      );
    },
  },
  {
    id: "strategic-ai-policy",
    label: "Strategic AI Policy",
    url: "/strategic-ai-policy",
    component: function StrategicAIPolicy() {
      return <JsonForm file="strategic-ai-policy.json" />;
    },
  },
  {
    id: "strategic-bloodcat-placements",
    label: "Strategic Bloodcat Placements",
    url: "/strategic-bloodcat-placements",
    component: function StrategicBloodcatPlacements() {
      return <JsonStrategicMapForm file="strategic-bloodcat-placements.json" />;
    },
  },
  {
    id: "strategic-bloodcat-spawns",
    label: "Strategic Bloodcat Spawns",
    url: "/strategic-bloodcat-spawns",
    component: function StrategicBloodcatSpawns() {
      return <JsonStrategicMapForm file="strategic-bloodcat-spawns.json" />;
    },
  },
  {
    id: "strategic-fact-params",
    label: "Strategic Fact Params",
    url: "/strategic-fact-params",
    component: function StrategicFactParams() {
      return <JsonItemsForm file="strategic-fact-params.json" name="fact" />;
    },
  },
  {
    id: "strategic-map-cache-sectors",
    label: "Strategic Map Cache Sectors",
    url: "/strategic-map-cache-sectors",
    component: function StrategicMapCacheSectors() {
      return <JsonForm file="strategic-map-cache-sectors.json" />;
    },
  },
  {
    id: "strategic-map-creature-lairs",
    label: "Strategic Map Creature Lairs",
    url: "/strategic-map-creature-lairs",
    component: function StrategicMapCreatureLairs() {
      const getCreatureLairName = useCallback((item: any) => {
        return item.entranceSector[0];
      }, []);
      return (
        <JsonItemsForm
          file="strategic-map-creature-lairs.json"
          name={getCreatureLairName}
        />
      );
    },
  },
  {
    id: "strategic-map-movement-costs",
    label: "Strategic Map Movement Costs",
    url: "/strategic-map-movement-costs",
    component: function StrategicMapMovementCosts() {
      return <JsonForm file="strategic-map-movement-costs.json" />;
    },
  },
  {
    id: "strategic-map-npc-placements",
    label: "Strategic Map NPC Placements",
    url: "/strategic-map-npc-placements",
    component: function StrategicMapNpcPlacements() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        []
      );
      return (
        <JsonItemsForm
          file="strategic-map-npc-placements.json"
          name="profileId"
          preview={preview}
        />
      );
    },
  },
  {
    id: "strategic-map-sam-sites-air-control",
    label: "Strategic Map Sam Sites Air Control",
    url: "/strategic-map-sam-sites-air-control",
    component: function StrategicMapSamSitesAirControl() {
      return <JsonForm file="strategic-map-sam-sites-air-control.json" />;
    },
  },
  {
    id: "strategic-map-sam-sites",
    label: "Strategic Map Sam Sites",
    url: "/strategic-map-sam-sites",
    component: function StrategicMapSamSites() {
      return <JsonStrategicMapForm file="strategic-map-sam-sites.json" />;
    },
  },
  {
    id: "strategic-map-secrets",
    label: "Strategic Map Secrets",
    url: "/strategic-map-secrets",
    component: function StrategicMapSecrets() {
      return <JsonStrategicMapForm file="strategic-map-secrets.json" />;
    },
  },
  {
    id: "strategic-map-sectors-descriptions",
    label: "Strategic Map Sector Descriptions",
    url: "/strategic-map-sectors-descriptions",
    component: function StrategicMapSectorsDescriptions() {
      return (
        <JsonStrategicMapForm file="strategic-map-sectors-descriptions.json" />
      );
    },
  },
  {
    id: "strategic-map-towns",
    label: "Strategic Map Towns",
    url: "/strategic-map-towns",
    component: function StrategicMapTowns() {
      return <JsonItemsForm file="strategic-map-towns.json" name="townId" />;
    },
  },
  {
    id: "strategic-map-traversibility-ratings",
    label: "Strategic Map Traversibility Ratings",
    url: "/strategic-map-traversibility-ratings",
    component: function StrategicMapTraversibilityRatings() {
      return <JsonForm file="strategic-map-traversibility-ratings.json" />;
    },
  },
  {
    id: "strategic-map-underground-sectors",
    label: "Strategic Map Underground Sectors",
    url: "/strategic-map-underground-sectors",
    component: function StrategicMapUndergroundSectors() {
      return (
        <JsonStrategicMapForm file="strategic-map-underground-sectors.json" />
      );
    },
  },
  {
    id: "strategic-mines",
    label: "Strategic Mines",
    url: "/strategic-mines",
    component: function StrategicMines() {
      return (
        <JsonStrategicMapForm
          file="strategic-mines.json"
          property="entranceSector"
        />
      );
    },
  },
  {
    id: "tactical-map-item-replacements",
    label: "Tactical Map Item Replacements",
    url: "/tactical-map-item-replacements",
    component: function TacticalMapItemReplacements() {
      const getItemReplacementName = useCallback((item: any) => {
        return `${item.from} to ${item.to}`;
      }, []);
      return (
        <JsonItemsForm
          file="tactical-map-item-replacements.json"
          name={getItemReplacementName}
        />
      );
    },
  },
  {
    id: "tactical-npc-action-params",
    label: "Tactical Npc Action Params",
    url: "/tactical-npc-action-params",
    component: function TacticalNpcActionParams() {
      return (
        <JsonItemsForm
          file="tactical-npc-action-params.json"
          name="actionCode"
        />
      );
    },
  },
  {
    id: "vehicles",
    label: "Vehicles",
    url: "/vehicles",
    component: function Vehicles() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        []
      );
      return (
        <JsonItemsForm file="vehicles.json" name="profile" preview={preview} />
      );
    },
  },
  {
    id: "weapons",
    label: "Weapons",
    url: "/weapons",
    component: function Weapons() {
      const preview = useCallback(
        (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        []
      );
      return (
        <JsonItemsForm
          file="weapons.json"
          name="internalName"
          preview={preview}
        />
      );
    },
  },
];
