import { ReactElement, useCallback, useMemo } from 'react';
import { stringReferenceTo } from './components/form/StringReferenceWidget';
import { JsonForm } from './components/JsonForm';
import { JsonItemsForm } from './components/JsonItemsForm';
import { JsonStrategicMapForm } from './components/StrategicMapForm';
import { Dashboard } from './components/Dashboard';
import { MercPreview } from './components/content/MercPreview';
import { ItemPreview } from './components/content/ItemPreview';
import { resourceReference } from './components/form/ResourceReferenceWidget';
import { ResourceType } from './lib/listDir';

export interface Route {
  id: string;
  label: string;
  url: string;
  component: () => ReactElement;
}

export interface Item {
  type: 'Item';
  id: string;
  label: string;
  component: () => ReactElement;
}

export interface Submenu {
  type: 'Submenu';
  id: string;
  label: string;
  children: Array<Item | Submenu>;
}

export type MenuItem = Item | Submenu;

export const MENU: Readonly<Array<Readonly<MenuItem>>> = [
  {
    type: 'Item',
    id: 'dashboard',
    label: 'Dashboard',
    component: Dashboard,
  },
  {
    type: 'Submenu',
    id: 'army',
    label: 'Army',
    children: [
      {
        type: 'Item',
        id: 'ai-policy',
        label: 'Strategic AI Policy',
        component: function StrategicAIPolicy() {
          return <JsonForm file="strategic-ai-policy.json" />;
        },
      },
      {
        type: 'Item',
        id: 'compositions',
        label: 'Compositions',
        component: function ArmyCompositions() {
          return <JsonItemsForm file="army-compositions.json" name="name" />;
        },
      },
      {
        type: 'Item',
        id: 'garrison-groups',
        label: 'Garrison Groups',
        component: function ArmyGarrisonGroups() {
          const uiSchema = useMemo(
            () => ({
              composition: {
                'ui:widget': stringReferenceTo(
                  'army-compositions.json',
                  'name',
                ),
              },
            }),
            [],
          );
          return (
            <JsonStrategicMapForm
              file="army-garrison-groups.json"
              uiSchema={uiSchema}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'gun-choice-extended',
        label: 'Gun Choice Extended',
        component: function ArmyGunChoiceExtended() {
          const uiSchema = useMemo(
            () => ({
              items: {
                items: {
                  'ui:widget': stringReferenceTo(
                    'weapons.json',
                    'internalName',
                  ),
                },
              },
            }),
            [],
          );
          return (
            <JsonForm
              file="army-gun-choice-extended.json"
              uiSchema={uiSchema}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'gun-choice-normal',
        label: 'Gun Choice Normal',
        component: function ArmyGunChoiceNormal() {
          const uiSchema = useMemo(
            () => ({
              items: {
                items: {
                  'ui:widget': stringReferenceTo(
                    'weapons.json',
                    'internalName',
                  ),
                },
              },
            }),
            [],
          );
          return (
            <JsonForm file="army-gun-choice-normal.json" uiSchema={uiSchema} />
          );
        },
      },
      {
        type: 'Item',
        id: 'patrol-groups',
        label: 'Patrol Groups',
        component: function ArmyPatrolGroups() {
          const getPatrolGroupName = useCallback((item: any) => {
            return item.points.join(', ') as string;
          }, []);
          return (
            <JsonItemsForm
              file="army-patrol-groups.json"
              name={getPatrolGroupName}
            />
          );
        },
      },
    ],
  },
  {
    type: 'Item',
    id: 'dealers',
    label: 'Dealers',
    component: function Dealers() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        [],
      );
      return (
        <JsonItemsForm file="dealers.json" name="profile" preview={preview} />
      );
    },
  },
  {
    type: 'Item',
    id: 'game',
    label: 'Game',
    component: function Game() {
      return <JsonForm file="game.json" />;
    },
  },
  {
    type: 'Item',
    id: 'imp',
    label: 'IMP',
    component: function Imp() {
      return <JsonForm file="imp.json" />;
    },
  },
  {
    type: 'Submenu',
    id: 'items',
    label: 'Items',
    children: [
      {
        type: 'Item',
        id: 'ammo-types',
        label: 'Ammo Types',
        component: function AmmoTypes() {
          return <JsonItemsForm file="ammo-types.json" name="internalName" />;
        },
      },
      {
        type: 'Item',
        id: 'calibres',
        label: 'Calibres',
        component: function Calibres() {
          const uiSchema = useMemo(
            () => ({
              sound: {
                'ui:widget': resourceReference(ResourceType.Sound),
              },
              silencedSound: {
                'ui:widget': resourceReference(ResourceType.Sound),
              },
            }),
            [],
          );

          return (
            <JsonItemsForm
              file="calibres.json"
              name="internalName"
              uiSchema={uiSchema}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'items',
        label: 'Items',
        component: function Items() {
          const preview = useCallback(
            (item: any) => (
              <ItemPreview inventoryGraphics={item.inventoryGraphics} />
            ),
            [],
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
        type: 'Item',
        id: 'magazines',
        label: 'Magazines',
        component: function Magazines() {
          const preview = useCallback(
            (item: any) => (
              <ItemPreview inventoryGraphics={item.inventoryGraphics} />
            ),
            [],
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
        type: 'Item',
        id: 'tactical-map-item-replacements',
        label: 'Tactical Map Item Replacements',
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
        type: 'Item',
        id: 'weapons',
        label: 'Weapons',
        component: function Weapons() {
          const preview = useCallback(
            (item: any) => (
              <ItemPreview inventoryGraphics={item.inventoryGraphics} />
            ),
            [],
          );
          const uiSchema = useMemo(
            () => ({
              sound: {
                'ui:widget': resourceReference(ResourceType.Sound),
              },
              silencedSound: {
                'ui:widget': resourceReference(ResourceType.Sound),
              },
            }),
            [],
          );

          return (
            <JsonItemsForm
              file="weapons.json"
              name="internalName"
              preview={preview}
              uiSchema={uiSchema}
            />
          );
        },
      },
    ],
  },
  {
    type: 'Item',
    id: 'loading-screens-mapping',
    label: 'Loading Screens Mapping',
    component: function LoadingScreensMapping() {
      return <JsonStrategicMapForm file="loading-screens-mapping.json" />;
    },
  },
  {
    type: 'Item',
    id: 'loading-screens',
    label: 'Loading Screens',
    component: function LoadingScreens() {
      return <JsonItemsForm file="loading-screens.json" name="internalName" />;
    },
  },
  {
    type: 'Submenu',
    id: 'mercs',
    label: 'Mercs',
    children: [
      {
        type: 'Item',
        id: 'merc-listings',
        label: 'M.E.R.C. Listings',
        component: function MercsMERCListings() {
          const preview = useCallback(
            (item: any) => <MercPreview profile={item.profile} />,
            [],
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
        type: 'Item',
        id: 'profiles',
        label: 'Profiles',
        component: function MercsProfileInfo() {
          const preview = useCallback(
            (item: any) => <MercPreview profile={item.internalName} />,
            [],
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
        type: 'Item',
        id: 'rpc-small-faces',
        label: 'RPC Small Faces',
        component: function MercsRpcSmallFaces() {
          const preview = useCallback(
            (item: any) => <MercPreview profile={item.profile} />,
            [],
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
    ],
  },
  {
    type: 'Item',
    id: 'music',
    label: 'Music',
    component: function Music() {
      const uiSchema = useMemo(
        () =>
          Object.fromEntries(
            [
              'main_menu',
              'laptop',
              'tactical',
              'tactical_enemypresent',
              'tactical_battle',
              'tactical_creature',
              'tactical_creature_enemypresent',
              'tactical_creature_battle',
              'tactical_victory',
              'tactical_defeat',
            ].map((k) => [
              k,
              {
                items: {
                  'ui:widget': resourceReference(ResourceType.Sound),
                },
              },
            ]),
          ),
        [],
      );

      return <JsonForm file="music.json" uiSchema={uiSchema} />;
    },
  },
  {
    type: 'Item',
    id: 'shipping-destinations',
    label: 'Shipping Destinations',
    component: function ShippingDestinations() {
      return (
        <JsonItemsForm file="shipping-destinations.json" name="locationId" />
      );
    },
  },
  {
    type: 'Submenu',
    id: 'strategic-map',
    label: 'Strategic Map',
    children: [
      {
        type: 'Item',
        id: 'bloodcat-placements',
        label: 'Bloodcat Placements',
        component: function StrategicBloodcatPlacements() {
          return (
            <JsonStrategicMapForm file="strategic-bloodcat-placements.json" />
          );
        },
      },
      {
        type: 'Item',
        id: 'bloodcat-spawns',
        label: 'Bloodcat Spawns',
        component: function StrategicBloodcatSpawns() {
          return <JsonStrategicMapForm file="strategic-bloodcat-spawns.json" />;
        },
      },
      {
        type: 'Item',
        id: 'fact-params',
        label: 'Fact Params',
        component: function StrategicFactParams() {
          return (
            <JsonItemsForm file="strategic-fact-params.json" name="fact" />
          );
        },
      },
      {
        type: 'Item',
        id: 'cache-sectors',
        label: 'Weapon Cache Sectors',
        component: function StrategicMapCacheSectors() {
          return <JsonForm file="strategic-map-cache-sectors.json" />;
        },
      },
      {
        type: 'Item',
        id: 'creature-lairs',
        label: 'Creature Lairs',
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
        type: 'Item',
        id: 'movement-costs',
        label: 'Movement Costs',
        component: function StrategicMapMovementCosts() {
          return <JsonForm file="strategic-map-movement-costs.json" />;
        },
      },
      {
        type: 'Item',
        id: 'npc-placements',
        label: 'NPC Placements',
        component: function StrategicMapNpcPlacements() {
          const preview = useCallback(
            (item: any) => <MercPreview profile={item.profile} />,
            [],
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
        type: 'Item',
        id: 'sam-sites-air-control',
        label: 'Sam Sites Air Control',
        component: function StrategicMapSamSitesAirControl() {
          return <JsonForm file="strategic-map-sam-sites-air-control.json" />;
        },
      },
      {
        type: 'Item',
        id: 'sam-sites',
        label: 'Sam Sites',
        component: function StrategicMapSamSites() {
          return <JsonStrategicMapForm file="strategic-map-sam-sites.json" />;
        },
      },
      {
        type: 'Item',
        id: 'secrets',
        label: 'Secrets',
        component: function StrategicMapSecrets() {
          return <JsonStrategicMapForm file="strategic-map-secrets.json" />;
        },
      },
      {
        type: 'Item',
        id: 'sectors-descriptions',
        label: 'Sector Descriptions',
        component: function StrategicMapSectorsDescriptions() {
          return (
            <JsonStrategicMapForm file="strategic-map-sectors-descriptions.json" />
          );
        },
      },
      {
        type: 'Item',
        id: 'towns',
        label: 'Towns',
        component: function StrategicMapTowns() {
          return (
            <JsonItemsForm file="strategic-map-towns.json" name="townId" />
          );
        },
      },
      {
        type: 'Item',
        id: 'traversibility-ratings',
        label: 'Traversibility Ratings',
        component: function StrategicMapTraversibilityRatings() {
          return <JsonForm file="strategic-map-traversibility-ratings.json" />;
        },
      },
      {
        type: 'Item',
        id: 'underground-sectors',
        label: 'Underground Sectors',
        component: function StrategicMapUndergroundSectors() {
          return (
            <JsonStrategicMapForm file="strategic-map-underground-sectors.json" />
          );
        },
      },
      {
        type: 'Item',
        id: 'mines',
        label: 'Mines',
        component: function StrategicMines() {
          return (
            <JsonStrategicMapForm
              file="strategic-mines.json"
              property="entranceSector"
            />
          );
        },
      },
    ],
  },
  {
    type: 'Item',
    id: 'tactical-npc-action-params',
    label: 'Tactical Npc Action Params',
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
    type: 'Item',
    id: 'vehicles',
    label: 'Vehicles',
    component: function Vehicles() {
      const preview = useCallback(
        (item: any) => <MercPreview profile={item.profile} />,
        [],
      );
      return (
        <JsonItemsForm file="vehicles.json" name="profile" preview={preview} />
      );
    },
  },
];

const menuItemToRoutes =
  (parentPath: string) =>
  (item: MenuItem): Array<Route> => {
    if (item.type === 'Item') {
      return [
        {
          id: item.id,
          label: item.label,
          url: `${parentPath}/${item.id}`,
          component: item.component,
        },
      ];
    }
    const mapFn = menuItemToRoutes(`${parentPath}/${item.id}`);
    return item.children.flatMap(mapFn);
  };

export const ROUTES: Readonly<Array<Readonly<Route>>> = MENU.flatMap(
  menuItemToRoutes(''),
);
