import { ReactElement, useCallback, useMemo } from 'react';
import {
  stringReferenceTo,
  stringReferenceToAmmoTypes,
  stringReferenceToArmours,
  stringReferenceToArmyCompositions,
  stringReferenceToCalibres,
  stringReferenceToExplosionAnimations,
  stringReferenceToExplosiveCalibres,
  stringReferenceToItems,
  stringReferenceToLoadingScreens,
  stringReferenceToMagazines,
  stringReferenceToMercProfiles,
  stringReferenceToTowns,
  stringReferenceToWeapons,
} from './components/form/StringReferenceWidget';
import { JsonForm } from './components/JsonForm';
import { JsonItemsForm } from './components/JsonItemsForm';
import { JsonStrategicMapForm } from './components/StrategicMapForm';
import { Dashboard } from './components/Dashboard';
import { MercPreview } from './components/content/MercPreview';
import { ItemPreview } from './components/content/ItemPreview';
import { resourceReference } from './components/form/ResourceReferenceWidget';
import { ResourceType } from './lib/listDir';
import { StiPreview } from './components/content/StiPreview';

const baseItemProps = [
  'itemIndex',
  'internalName',
  'shortName',
  'name',
  'description',
  'inventoryGraphics',
  'tileGraphic',
  'ubPerPocket',
  'ubWeight',
  'usPrice',
  'ubCoolness',
  'bReliability',
  'bRepairable',
  'bRepairEase',
];
const baseItemFlags = [
  'bDamageable',
  'bWaterDamages',

  'bBigGunList',
  'bAttachment',
  'bInseparable',
  'bDefaultUndroppable',
  'bNotBuyable',
  'bNotEditor',
  'bShowStatus',
  'bHiddenAddon',

  'bTwoHanded',
  'bElectronic',
  'bMetal',
  'bUnaerodynamic',
  'bSinks',
];

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
        id: 'compositions',
        label: 'Compositions',
        component: function ArmyCompositions() {
          return (
            <JsonItemsForm
              file="army-compositions.json"
              name="name"
              uiSchema={{
                'ui:order': [
                  'id',
                  'name',
                  'priority',
                  'startPopulation',
                  'desiredPopulation',
                  'adminPercentage',
                  'troopPercentage',
                  'elitePercentage',
                ],
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'garrison-groups',
        label: 'Garrison Groups',
        component: function ArmyGarrisonGroups() {
          return (
            <JsonStrategicMapForm
              file="army-garrison-groups.json"
              uiSchema={{
                'ui:order': ['sector', 'composition'],
                sector: { 'ui:disabled': true },
                composition: {
                  'ui:widget': stringReferenceToArmyCompositions,
                },
              }}
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
          return (
            <JsonForm
              file="army-gun-choice-normal.json"
              uiSchema={{
                items: {
                  items: {
                    'ui:widget': stringReferenceToWeapons,
                  },
                },
              }}
            />
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
      {
        type: 'Item',
        id: 'ai-policy',
        label: 'Strategic AI Policy',
        component: function StrategicAIPolicy() {
          return <JsonForm file="strategic-ai-policy.json" />;
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
        <JsonItemsForm
          file="dealers.json"
          name="profile"
          preview={preview}
          uiSchema={{
            'ui:order': [
              'profile',
              'type',
              'initialCash',
              'buyingPrice',
              'sellingPrice',
              'repairCost',
              'repairSpeed',
              'flags',
            ],
            profile: {
              'ui:widget': stringReferenceToMercProfiles,
            },
          }}
        />
      );
    },
  },
  {
    type: 'Submenu',
    id: 'explosions',
    label: 'Explosions',
    children: [
      {
        type: 'Item',
        id: 'explosion-animations',
        label: 'Explosion Animations',
        component: function ExplosionAnimations() {
          const preview = useCallback(
            (item: any) => (
              <StiPreview file={item.graphics} subimage={item.damageKeyframe} />
            ),
            [],
          );
          return (
            <JsonItemsForm
              file="explosion-animations.json"
              name="name"
              preview={preview}
              uiSchema={{
                'ui:order': [
                  'id',
                  'name',
                  'blastSpeed',
                  'damageKeyframe',
                  'transparentKeyframe',
                  'graphics',
                  'sounds',
                  'waterAnimation',
                ],
                waterAnimation: {
                  'ui:widget': stringReferenceToExplosionAnimations,
                },
                sounds: {
                  items: {
                    'ui:widget': resourceReference(ResourceType.Sound),
                  },
                },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'smoke-effects',
        label: 'Smoke Effects',
        component: function SmokeEffects() {
          const preview = useCallback(
            (item: any) => <StiPreview file={item.staticGraphics} />,
            [],
          );
          return (
            <JsonItemsForm
              file="smoke-effects.json"
              name="name"
              preview={preview}
              uiSchema={{
                'ui:order': [
                  'name',
                  'damage',
                  'breathDamage',
                  'maxVisibility',
                  'maxVisibilityWhenAffected',
                  'lostVisibilityPerTile',
                  'graphics',
                  'staticGraphics',
                  'dissipatingGraphics',
                  'ignoresGasMask',
                  'affectsMonsters',
                  'affectsRobot',
                ],
              }}
            />
          );
        },
      },
    ],
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
      return (
        <JsonForm
          file="imp.json"
          uiSchema={{
            'ui:order': [
              'activation_codes',
              'starting_level',
              'inventory',
              'if_normal_shooter',
              'if_good_shooter',
            ],
            inventory: {
              items: {
                'ui:widget': stringReferenceToItems,
              },
            },
            if_normal_shooter: {
              items: {
                'ui:widget': stringReferenceToItems,
              },
            },
            if_good_shooter: {
              items: {
                'ui:widget': stringReferenceToItems,
              },
            },
          }}
        />
      );
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
          return (
            <JsonItemsForm
              file="ammo-types.json"
              name="internalName"
              uiSchema={{ 'ui:order': ['index', 'internalName'] }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'armours',
        label: 'Armours',
        component: function Armours() {
          const preview = useCallback(
            (item: any) => (
              <ItemPreview inventoryGraphics={item.inventoryGraphics} />
            ),
            [],
          );
          return (
            <JsonItemsForm
              file="armours.json"
              name="internalName"
              preview={preview}
              uiSchema={{
                'ui:order': [
                  ...baseItemProps,

                  'armourClass',
                  'protection',
                  'explosivesProtection',
                  'degradePercentage',

                  'ignoreForMaxProtection',

                  ...baseItemFlags,
                ],
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'calibres',
        label: 'Calibres',
        component: function Calibres() {
          const uiSchema = useMemo(
            () => ({
              'ui:order': [
                'index',
                'internalName',
                'sound',
                'silencedSound',
                'burstSound',
                'silencedBurstSound',
                'monsterWeapon',
                'showInHelpText',
              ],
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
        id: 'explosive-calibres',
        label: 'Explosive Calibres',
        component: function ExplosiveCalibres() {
          return (
            <JsonItemsForm file="explosive-calibres.json" name="internalName" />
          );
        },
      },
      {
        type: 'Item',
        id: 'explosives',
        label: 'Explosives',
        component: function Explosives() {
          const preview = useCallback(
            (item: any) => (
              <ItemPreview inventoryGraphics={item.inventoryGraphics} />
            ),
            [],
          );
          return (
            <JsonItemsForm
              file="explosives.json"
              name="internalName"
              preview={preview}
              uiSchema={{
                'ui:order': [
                  ...baseItemProps,
                  'itemClass',
                  'noise',
                  'volatility',
                  'calibre',
                  'cursor',
                  'animation',
                  'blastEffect',
                  'stunEffect',
                  'smokeEffect',
                  'lightEffect',
                  'isPressureTriggered',
                  ...baseItemFlags,
                ],
                calibre: {
                  'ui:widget': stringReferenceToExplosiveCalibres,
                },
                animation: {
                  'ui:widget': stringReferenceToExplosionAnimations,
                },
              }}
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
              uiSchema={{
                'ui:order': [
                  'usItemClass',
                  ...baseItemProps,
                  'ubCursor',
                  'ubClassIndex',
                  ...baseItemFlags,
                ],
              }}
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
              uiSchema={{
                'ui:order': [
                  ...baseItemProps,
                  'ammoType',
                  'calibre',
                  'capacity',
                  'standardReplacement',
                  'dontUseAsDefaultMagazine',
                  ...baseItemFlags,
                ],
                ammoType: {
                  'ui:widget': stringReferenceToAmmoTypes,
                },
                calibre: {
                  'ui:widget': stringReferenceToCalibres,
                },
                standardReplacement: {
                  'ui:widget': stringReferenceToMagazines,
                },
              }}
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
              uiSchema={{
                from: {
                  oneOf: [
                    {
                      'ui:widget': stringReferenceToItems,
                    },
                    {},
                  ],
                },
              }}
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
              'ui:order': [
                ...baseItemProps,

                'internalType',
                'calibre',
                'rateOfFire',
                'ubAttackVolume',
                'ubBulletSpeed',
                'ubBurstPenalty',
                'ubDeadliness',
                'ubHitVolume',
                'ubMagSize',
                'ubReadyTime',
                'ubShotsPer4Turns',
                'ubShotsPerBurst',
                'usRange',
                'ubImpact',
                'usSmokeEffect',

                'sound',
                'silencedSound',
                'burstSound',
                'silencedBurstSound',

                'standardReplacement',

                'attachment_Bipod',
                'attachment_Duckbill',
                'attachment_GunBarrelExtender',
                'attachment_LaserScope',
                'attachment_Silencer',
                'attachment_SniperScope',
                'attachment_SpringAndBoltUpgrade',
                'attachment_UnderGLauncher',
                ...baseItemFlags,
              ],
              calibre: {
                'ui:widget': stringReferenceToCalibres,
              },
              standardReplacement: {
                'ui:widget': stringReferenceToWeapons,
              },
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
    id: 'loading-screens',
    label: 'Loading Screens',
    component: function LoadingScreens() {
      return (
        <JsonItemsForm
          file="loading-screens.json"
          name="internalName"
          uiSchema={{ 'ui:order': ['internalName', 'filename'] }}
        />
      );
    },
  },
  {
    type: 'Item',
    id: 'loading-screens-mapping',
    label: 'Loading Screens Mapping',
    component: function LoadingScreensMapping() {
      return (
        <JsonStrategicMapForm
          file="loading-screens-mapping.json"
          uiSchema={{
            'ui:order': ['sector', 'sectorLevel', 'day', 'night'],
            sector: { 'ui:disabled': true },
            sectorLevel: { 'ui:disabled': true },
            day: { 'ui:widget': stringReferenceToLoadingScreens },
            night: { 'ui:widget': stringReferenceToLoadingScreens },
          }}
        />
      );
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
              uiSchema={{
                'ui:order': [
                  'profile',
                  'bioIndex',
                  'minDays',
                  'minTotalSpending',
                  'quotes',
                ],
                profile: {
                  'ui:widget': stringReferenceToMercProfiles,
                },
                quotes: {
                  items: {
                    'ui:order': ['type', 'quoteID', 'profile'],
                    profile: {
                      'ui:widget': stringReferenceToMercProfiles,
                    },
                  },
                },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'relations',
        label: 'Opinions',
        component: function MercsRelations() {
          return (
            <JsonItemsForm file="mercs-relations.json" name="internalName" />
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
              uiSchema={{
                'ui:order': [
                  'profileID',
                  'internalName',
                  'type',
                  'nickname',
                  'fullName',
                  'sex',
                  'bodyType',
                  'bodyTypeSubstitution',
                  'face',
                  'skinColor',
                  'hairColor',
                  'vestColor',
                  'pantsColor',
                  'personalityTrait',
                  'skillTrait',
                  'skillTrait2',
                  'stats',
                  'attitude',
                  'toleranceForPlayersDeathRate',
                  'toleranceForPlayersReputation',
                  'sexismMode',
                  'contract',
                  'inventory',
                  'dialogue',
                  'money',
                  'weaponSaleModifier',
                  'civilianGroup',
                  'sector',
                  'town',
                  'townAttachment',
                  'ownedRooms',
                  'isGoodGuy',
                  'isTownIndifferentIfDead',
                ],
                inventory: {
                  items: {
                    'ui:order': [
                      'item',
                      'quantity',
                      'status',
                      'slot',
                      'isUndroppable',
                    ],
                    item: {
                      'ui:widget': stringReferenceToItems,
                    },
                  },
                },
              }}
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
              uiSchema={{
                'ui:order': ['profile', 'eyesXY', 'mouthXY'],
                profile: {
                  'ui:widget': stringReferenceToMercProfiles,
                },
              }}
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
    type: 'Submenu',
    id: 'scripts',
    label: 'Scripts',
    children: [
      {
        type: 'Item',
        id: 'records',
        label: 'NPC Records',
        component: function Records() {
          const preview = useCallback(
            (item: any) => <MercPreview profile={item.profile} />,
            [],
          );

          return (
            <JsonItemsForm
              file="script-records-NPCs.json"
              name="profile"
              preview={preview}
              uiSchema={{
                'ui:order': ['profile', 'meanwhileIndex', 'records'],
                profile: {
                  'ui:widget': stringReferenceToMercProfiles,
                },
                records: {
                  items: {
                    'ui:order': [
                      'index',
                      'quoteNum',
                      'numQuotes',
                      'firstDay',
                      'lastDay',
                      'factMustBeTrue',
                      'factMustBeFalse',
                      'requiredAnyItem',
                      'requiredItem',
                      'requiredAnyRifle',
                      'requiredApproach',
                      'requiredGridNo',
                      'requiredOpinion',
                      'sayOncePerConvo',
                      'userInterface',
                      'startQuest',
                      'endQuest',
                      'quest',
                      'setFactTrue',
                      'triggerClosestMerc',
                      'triggerNPC',
                      'triggerRecord',
                      'triggerSelf',
                      'eraseOnceSaid',
                      'alreadySaid',
                      'giftItem',
                      'goToGridno',
                      'actionData',
                    ],
                    requiredItem: {
                      'ui:widget': stringReferenceToItems,
                    },
                    triggerNPC: {
                      'ui:widget': stringReferenceToMercProfiles,
                    },
                    giftItem: {
                      'ui:widget': stringReferenceToItems,
                    },
                  },
                },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'records-control',
        label: 'Records Control',
        component: function RecordsControl() {
          return (
            <JsonForm
              file="script-records-control.json"
              uiSchema={{
                'ui:order': ['fileNameForScriptControlledPCs', 'meanwhiles'],
                meanwhiles: {
                  items: {
                    'ui:order': ['id', 'internalName', 'chars'],
                    chars: {
                      items: {
                        'ui:order': ['id', 'name', 'fileName'],
                        name: {
                          'ui:widget': stringReferenceToMercProfiles,
                        },
                      },
                    },
                  },
                },
              }}
            />
          );
        },
      },
    ],
  },
  {
    type: 'Item',
    id: 'shipping-destinations',
    label: 'Shipping Destinations',
    component: function ShippingDestinations() {
      return (
        <JsonItemsForm
          file="shipping-destinations.json"
          name="locationId"
          uiSchema={{
            'ui:order': [
              'locationId',
              'deliverySector',
              'deliverySectorZ',
              'deliverySectorGridNo',
              'chargeRateStandard',
              'chargeRate2Days',
              'chargeRateOverNight',
              'flowersNextDayDeliveryCost',
              'flowersWhenItGetsThereCost',
              'emailOffset',
              'emailLength',
              'canDeliver',
              'isPrimary',
            ],
          }}
        />
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
            <JsonStrategicMapForm
              file="strategic-bloodcat-placements.json"
              uiSchema={{
                'ui:order': ['sector', 'bloodCatPlacements'],
                sector: { 'ui:disabled': true },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'bloodcat-spawns',
        label: 'Bloodcat Spawns',
        component: function StrategicBloodcatSpawns() {
          return (
            <JsonStrategicMapForm
              file="strategic-bloodcat-spawns.json"
              uiSchema={{
                'ui:order': [
                  'sector',
                  'bloodCatsSpawnsEasy',
                  'bloodCatsSpawnsMedium',
                  'bloodCatsSpawnsHard',
                  'isArena',
                  'isLair',
                ],
                sector: { 'ui:disabled': true },
              }}
            />
          );
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
              uiSchema={{
                'ui:order': [
                  'lairId',
                  'associatedMineId',
                  'entranceSector',
                  'warpExit',
                  'sectors',
                  'attackSectors',
                ],
              }}
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
              name="profile"
              preview={preview}
              uiSchema={{
                'ui:order': [
                  'profile',
                  'sectors',
                  'placedAtStart',
                  'useAlternateMap',
                  'sciFiOnly',
                ],
                profile: {
                  'ui:widget': stringReferenceToMercProfiles,
                },
              }}
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
          return (
            <JsonStrategicMapForm
              file="strategic-map-sam-sites.json"
              uiSchema={{
                'ui:order': ['sector', 'gridNos'],
                sector: { 'ui:disabled': true },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'secrets',
        label: 'Secrets',
        component: function StrategicMapSecrets() {
          return (
            <JsonStrategicMapForm
              file="strategic-map-secrets.json"
              uiSchema={{
                'ui:order': [
                  'sector',
                  'secretLandType',
                  'foundLandType',
                  'secretMapIcon',
                  'isSAMSite',
                ],
                sector: { 'ui:disabled': true },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'sectors-descriptions',
        label: 'Sector Descriptions',
        component: function StrategicMapSectorsDescriptions() {
          return (
            <JsonStrategicMapForm
              file="strategic-map-sectors-descriptions.json"
              uiSchema={{
                'ui:order': ['sector', 'sectorLevel', 'landType'],
                sector: { 'ui:disabled': true },
                sectorLevel: { 'ui:disabled': true },
              }}
            />
          );
        },
      },
      {
        type: 'Item',
        id: 'towns',
        label: 'Towns',
        component: function StrategicMapTowns() {
          return (
            <JsonItemsForm
              file="strategic-map-towns.json"
              name="townId"
              uiSchema={{
                'ui:order': [
                  'townId',
                  'internalName',
                  'sectors',
                  'townPoint',
                  'isMilitiaTrainingAllowed',
                ],
              }}
            />
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
            <JsonStrategicMapForm
              file="strategic-map-underground-sectors.json"
              uiSchema={{
                'ui:order': [
                  'sector',
                  'sectorLevel',
                  'adjacentSectors',
                  'numTroops',
                  'numTroopsVariance',
                  'numElites',
                  'numElitesVariance',
                  'numCreatures',
                  'numCreaturesVariance',
                ],
                sector: { 'ui:disabled': true },
                sectorLevel: { 'ui:disabled': true },
              }}
            />
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
              uiSchema={{
                'ui:order': [
                  'entranceSector',
                  'associatedTownId',
                  'associatedTown',
                  'mineType',
                  'minimumMineProduction',
                  'noDepletion',
                  'delayDepletion',
                  'headMinerAssigned',
                  'faceDisplayYOffset',
                  'mineSectors',
                ],
                entranceSector: { 'ui:disabled': true },
                associatedTown: { 'ui:widget': stringReferenceToTowns },
              }}
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
        <JsonItemsForm
          file="vehicles.json"
          name="profile"
          preview={preview}
          uiSchema={{
            'ui:order': [
              'profile',
              'movementType',
              'seats',
              'armourType',
              'enterSound',
              'moveSound',
            ],
            profile: { 'ui:widget': stringReferenceToMercProfiles },
            armourType: { 'ui:widget': stringReferenceToArmours },
          }}
        />
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
