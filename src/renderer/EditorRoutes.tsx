import { FunctionComponent, ReactElement } from 'react';
import {
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
import {
  JsonStrategicMapForm,
  makeStrategicMapFormPropsForProperties,
  makeStrategicMapFormPropsForProperty,
} from './components/StrategicMapForm';
import { Dashboard } from './components/Dashboard';
import { MercPreview } from './components/content/MercPreview';
import { ItemPreview } from './components/content/ItemPreview';
import {
  makeResourceReference,
  resourceReferenceToGraphics,
  resourceReferenceToSound,
} from './components/form/ResourceReferenceWidget';
import { StiPreview } from './components/content/StiPreview';
import { ResourceType } from './lib/resourceType';
import { makeMultiSectorSelectorWidget } from './components/form/MultiSectorSelectorWidget';
import { mergeDeep } from 'remeda';
import { UiSchema } from '@rjsf/utils';
import { InventoryGraphicsField } from './components/form/InventoryGraphicsField';
import { SamSitesAirControlForm } from './components/SamSitesAirControlForm';
import { MovementCostsForm } from './components/MovementCostsForm';

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
const baseItemUiSchema: UiSchema = {
  inventoryGraphics: {
    small: {
      'ui:field': InventoryGraphicsField,
    },
    big: {
      'ui:field': InventoryGraphicsField,
    },
  },
};

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
  file?: string;
  component: () => ReactElement;
}

export interface Submenu {
  type: 'Submenu';
  id: string;
  label: string;
  children: Array<Item | Submenu>;
}

export type MenuItem = Item | Submenu;

function makeFileItem<Props extends { file: string }>(
  file: string,
  label: string,
  Component: FunctionComponent<Props>,
  extraProps: Omit<Props, 'file'>,
): Item {
  const props = { file, ...extraProps } as Props;
  return {
    type: 'Item',
    id: file.replaceAll('.', '-'),
    label,
    file,
    component: function () {
      return <Component {...props} />;
    },
  };
}

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
      makeFileItem('army-compositions.json', 'Compositions', JsonItemsForm, {
        name: 'name',
        uiSchema: {
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
        },
      }),
      makeFileItem(
        'army-garrison-groups.json',
        'Garrison Groups',
        JsonStrategicMapForm,
        mergeDeep(makeStrategicMapFormPropsForProperty('sector'), {
          uiSchema: {
            'ui:order': ['sector', 'composition'],
            composition: {
              'ui:widget': stringReferenceToArmyCompositions,
            },
          },
        }),
      ),
      makeFileItem(
        'army-gun-choice-normal.json',
        'Gun Choice Normal',
        JsonForm,
        {
          uiSchema: {
            items: {
              items: {
                'ui:widget': stringReferenceToWeapons,
              },
            },
          },
        },
      ),
      makeFileItem(
        'army-gun-choice-extended.json',
        'Gun Choice Extended',
        JsonForm,
        {
          uiSchema: {
            items: {
              items: {
                'ui:widget': stringReferenceToWeapons,
              },
            },
          },
        },
      ),
      makeFileItem('army-patrol-groups.json', 'Patrol Groups', JsonItemsForm, {
        name: function getPatrolGroupName(item: any): string {
          return item.points.join(', ');
        },
      }),
      makeFileItem(
        'strategic-ai-policy.json',
        'Strategic AI Policy',
        JsonForm,
        {},
      ),
    ],
  },
  makeFileItem('dealers.json', 'Dealers', JsonItemsForm, {
    name: 'profile',
    preview: (item: any) => <MercPreview profile={item.profile} />,
    uiSchema: {
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
    },
  }),
  {
    type: 'Submenu',
    id: 'explosions',
    label: 'Explosions',
    children: [
      makeFileItem(
        'explosion-animations.json',
        'Explosion Animations',
        JsonItemsForm,
        {
          name: 'name',
          preview: (item: any) => (
            <StiPreview file={item.graphics} subimage={item.damageKeyframe} />
          ),
          uiSchema: {
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
            graphics: {
              'ui:widget': resourceReferenceToGraphics,
            },
            sounds: {
              items: {
                'ui:widget': resourceReferenceToSound,
              },
            },
          },
        },
      ),
      makeFileItem('smoke-effects.json', 'Smoke Effects', JsonItemsForm, {
        name: 'name',
        preview: (item: any) => <StiPreview file={item.staticGraphics} />,
        canAddNewItem: false,
        uiSchema: {
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
          graphics: {
            'ui:widget': resourceReferenceToGraphics,
          },
          staticGraphics: {
            'ui:widget': resourceReferenceToGraphics,
          },
          dissipatingGraphics: {
            'ui:widget': resourceReferenceToGraphics,
          },
        },
      }),
    ],
  },
  makeFileItem('game.json', 'Game', JsonForm, {}),
  makeFileItem('imp.json', 'IMP', JsonForm, {
    uiSchema: {
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
    },
  }),
  {
    type: 'Submenu',
    id: 'items',
    label: 'Items',
    children: [
      makeFileItem('ammo-types.json', 'Ammo Types', JsonItemsForm, {
        name: 'internalName',
        canAddNewItem: false,
        uiSchema: { 'ui:order': ['index', 'internalName'] },
      }),
      makeFileItem('armours.json', 'Armours', JsonItemsForm, {
        name: 'internalName',
        preview: (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        uiSchema: {
          'ui:order': [
            ...baseItemProps,

            'armourClass',
            'protection',
            'explosivesProtection',
            'degradePercentage',

            'ignoreForMaxProtection',

            ...baseItemFlags,
          ],
          ...baseItemUiSchema,
        },
      }),
      makeFileItem('calibres.json', 'Calibres', JsonItemsForm, {
        name: 'internalName',
        uiSchema: {
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
            'ui:widget': resourceReferenceToSound,
          },
          silencedSound: {
            'ui:widget': resourceReferenceToSound,
          },
        },
      }),
      makeFileItem(
        'explosive-calibres.json',
        'Explosive Calibres',
        JsonItemsForm,
        {
          name: 'internalName',
        },
      ),
      makeFileItem('explosives.json', 'Explosives', JsonItemsForm, {
        name: 'internalName',
        preview: (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        uiSchema: {
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
          ...baseItemUiSchema,
          calibre: {
            'ui:widget': stringReferenceToExplosiveCalibres,
          },
          animation: {
            'ui:widget': stringReferenceToExplosionAnimations,
          },
        },
      }),
      makeFileItem('items.json', 'Items', JsonItemsForm, {
        name: 'internalName',
        preview: (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        uiSchema: {
          'ui:order': [
            'usItemClass',
            ...baseItemProps,
            'ubCursor',
            'ubClassIndex',
            ...baseItemFlags,
          ],
          ...baseItemUiSchema,
        },
      }),
      makeFileItem('magazines.json', 'Magazines', JsonItemsForm, {
        name: 'internalName',
        preview: (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        uiSchema: {
          'ui:order': [
            ...baseItemProps,
            'ammoType',
            'calibre',
            'capacity',
            'standardReplacement',
            'dontUseAsDefaultMagazine',
            ...baseItemFlags,
          ],
          ...baseItemUiSchema,
          ammoType: {
            'ui:widget': stringReferenceToAmmoTypes,
          },
          calibre: {
            'ui:widget': stringReferenceToCalibres,
          },
          standardReplacement: {
            'ui:widget': stringReferenceToMagazines,
          },
        },
      }),
      makeFileItem(
        'tactical-map-item-replacements.json',
        'Tactical Map Item Replacements',
        JsonItemsForm,
        {
          name: (item: any) =>
            `${item.from ?? 'unknown'} to ${item.to ?? 'unknown'}`,
          uiSchema: {
            from: {
              oneOf: [
                {
                  'ui:widget': stringReferenceToItems,
                },
                {},
              ],
            },
          },
        },
      ),
      makeFileItem('weapons.json', 'Weapons', JsonItemsForm, {
        name: 'internalName',
        preview: (item: any) => (
          <ItemPreview inventoryGraphics={item.inventoryGraphics} />
        ),
        uiSchema: {
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
          ...baseItemUiSchema,
          calibre: {
            'ui:widget': stringReferenceToCalibres,
          },
          standardReplacement: {
            'ui:widget': stringReferenceToWeapons,
          },
          sound: {
            'ui:widget': resourceReferenceToSound,
          },
          silencedSound: {
            'ui:widget': resourceReferenceToSound,
          },
        },
      }),
    ],
  },
  makeFileItem('loading-screens.json', 'Loading Screens', JsonItemsForm, {
    name: 'internalName',
    preview: (item) => <StiPreview file={`loadscreens${item.filename}`} />,
    uiSchema: {
      'ui:order': ['internalName', 'filename'],
      filename: {
        'ui:widget': makeResourceReference({
          type: ResourceType.Graphics,
          prefix: ['loadscreens'],
          postProcess: (filename) => `/${filename}`,
        }),
      },
    },
  }),
  makeFileItem(
    'loading-screens-mapping.json',
    'Loading Screens Mapping',
    JsonStrategicMapForm,
    mergeDeep(makeStrategicMapFormPropsForProperties('sector', 'sectorLevel'), {
      uiSchema: {
        'ui:order': ['sector', 'sectorLevel', 'day', 'night'],
        day: { 'ui:widget': stringReferenceToLoadingScreens },
        night: { 'ui:widget': stringReferenceToLoadingScreens },
      },
    }),
  ),
  {
    type: 'Submenu',
    id: 'mercs',
    label: 'Mercs',
    children: [
      makeFileItem(
        'mercs-MERC-listings.json',
        'M.E.R.C. Listings',
        JsonItemsForm,
        {
          name: 'profile',
          preview: (item: any) => <MercPreview profile={item.profile} />,
          uiSchema: {
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
          },
        },
      ),
      makeFileItem('mercs-relations.json', 'Opinions', JsonItemsForm, {
        name: 'profile',
        preview: (item: any) => <MercPreview profile={item.profile} />,
        uiSchema: {
          'ui:order': ['profile', 'relations'],
          profile: {
            'ui:widget': stringReferenceToMercProfiles,
          },
          relations: {
            items: {
              'ui:order': [
                'target',
                'opinion',
                'friend1',
                'friend2',
                'enemy1',
                'enemy2',
                'eventualFriend',
                'resistanceToBefriending',
                'eventualEnemy',
                'resistanceToMakingEnemy',
                'tolerance',
              ],
              target: {
                'ui:widget': stringReferenceToMercProfiles,
              },
            },
          },
        },
      }),
      makeFileItem('mercs-profile-info.json', 'Profiles', JsonItemsForm, {
        name: 'internalName',
        preview: (item: any) => <MercPreview profile={item.internalName} />,
        uiSchema: {
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
        },
      }),
      makeFileItem(
        'mercs-rpc-small-faces.json',
        'RPC Small Faces',
        JsonItemsForm,
        {
          name: 'profile',
          preview: (item: any) => <MercPreview profile={item.profile} />,
          uiSchema: {
            'ui:order': ['profile', 'eyesXY', 'mouthXY'],
            profile: {
              'ui:widget': stringReferenceToMercProfiles,
            },
          },
        },
      ),
    ],
  },
  makeFileItem('music.json', 'Music', JsonForm, {
    uiSchema: Object.fromEntries(
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
            'ui:widget': resourceReferenceToSound,
          },
        },
      ]),
    ),
  }),
  {
    type: 'Submenu',
    id: 'scripts',
    label: 'Scripts',
    children: [
      makeFileItem('script-records-NPCs.json', 'NPC Records', JsonItemsForm, {
        name: 'profile',
        preview: (item: any) => <MercPreview profile={item.profile} />,
        uiSchema: {
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
        },
      }),
      makeFileItem('script-records-control.json', 'Records Control', JsonForm, {
        uiSchema: {
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
        },
      }),
    ],
  },
  makeFileItem(
    'shipping-destinations.json',
    'Shipping Destinations',
    JsonItemsForm,
    {
      name: (item) => item.locationId.toString(),
      uiSchema: {
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
      },
    },
  ),
  {
    type: 'Submenu',
    id: 'strategic-map',
    label: 'Strategic Map',
    children: [
      makeFileItem(
        'strategic-bloodcat-placements.json',
        'Bloodcat Placements',
        JsonStrategicMapForm,
        mergeDeep(makeStrategicMapFormPropsForProperty('sector'), {
          uiSchema: {
            'ui:order': ['sector', 'bloodCatPlacements'],
          },
        }),
      ),
      makeFileItem(
        'strategic-bloodcat-spawns.json',
        'Bloodcat Spawns',
        JsonStrategicMapForm,
        mergeDeep(makeStrategicMapFormPropsForProperty('sector'), {
          uiSchema: {
            'ui:order': [
              'sector',
              'bloodCatsSpawnsEasy',
              'bloodCatsSpawnsMedium',
              'bloodCatsSpawnsHard',
              'isArena',
              'isLair',
            ],
          },
        }),
      ),
      makeFileItem('strategic-fact-params.json', 'Fact Params', JsonItemsForm, {
        name: (item) => item.fact.toString(),
      }),
      makeFileItem(
        'strategic-map-cache-sectors.json',
        'Weapon Cache Sectors',
        JsonForm,
        {
          uiSchema: {
            'ui:order': ['sectors', 'numTroops', 'numTroopsVariance'],
            sectors: {
              'ui:widget': makeMultiSectorSelectorWidget({
                extractSectorFromItem: (value: string) => [value, 0],
                transformSectorToItem: (value) => value[0],
              }),
            },
          },
        },
      ),
      makeFileItem(
        'strategic-map-creature-lairs.json',
        'Creature Lairs',
        JsonItemsForm,
        {
          name: (item: any) => item.entranceSector?.[0] ?? 'unknown',
          uiSchema: {
            'ui:order': [
              'lairId',
              'associatedMineId',
              'entranceSector',
              'warpExit',
              'sectors',
              'attackSectors',
            ],
          },
        },
      ),
      makeFileItem(
        'strategic-map-movement-costs.json',
        'Movement Costs',
        MovementCostsForm,
        {},
      ),
      makeFileItem(
        'strategic-map-npc-placements.json',
        'NPC Placements',
        JsonItemsForm,
        {
          name: 'profile',
          preview: (item: any) => <MercPreview profile={item.profile} />,
          uiSchema: {
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
          },
        },
      ),
      makeFileItem(
        'strategic-map-sam-sites-air-control.json',
        'Sam Sites Air Control',
        SamSitesAirControlForm,
        {},
      ),
      makeFileItem(
        'strategic-map-sam-sites.json',
        'Sam Sites',
        JsonStrategicMapForm,
        mergeDeep(makeStrategicMapFormPropsForProperty('sector'), {
          uiSchema: {
            'ui:order': ['sector', 'gridNos'],
          },
        }),
      ),
      makeFileItem(
        'strategic-map-secrets.json',
        'Secrets',
        JsonStrategicMapForm,
        mergeDeep(makeStrategicMapFormPropsForProperty('sector'), {
          uiSchema: {
            'ui:order': [
              'sector',
              'secretLandType',
              'foundLandType',
              'secretMapIcon',
              'isSAMSite',
            ],
            secretMapIcon: {
              'ui:widget': resourceReferenceToGraphics,
            },
          },
        }),
      ),
      makeFileItem(
        'strategic-map-sectors-descriptions.json',
        'Sector Descriptions',
        JsonStrategicMapForm,
        mergeDeep(
          makeStrategicMapFormPropsForProperties('sector', 'sectorLevel'),
          {
            uiSchema: {
              'ui:order': ['sector', 'sectorLevel', 'landType'],
            },
          },
        ),
      ),
      makeFileItem('strategic-map-towns.json', 'Towns', JsonItemsForm, {
        name: 'internalName',
        uiSchema: {
          'ui:order': [
            'townId',
            'internalName',
            'sectors',
            'townPoint',
            'isMilitiaTrainingAllowed',
          ],
          sectors: {
            'ui:widget': makeMultiSectorSelectorWidget({
              extractSectorFromItem: (sector: string) => [sector, 0],
              transformSectorToItem: (sector) => sector[0],
            }),
          },
        },
      }),
      makeFileItem(
        'strategic-map-traversibility-ratings.json',
        'Traversibility Ratings',
        JsonForm,
        {},
      ),
      makeFileItem(
        'strategic-map-underground-sectors.json',
        'Underground Sectors',
        JsonStrategicMapForm,
        mergeDeep(
          makeStrategicMapFormPropsForProperties('sector', 'sectorLevel'),
          {
            initialLevel: 1,
            uiSchema: {
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
            },
          },
        ),
      ),
      makeFileItem(
        'strategic-mines.json',
        'Mines',
        JsonStrategicMapForm,
        mergeDeep(makeStrategicMapFormPropsForProperty('entranceSector'), {
          uiSchema: {
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
            associatedTown: { 'ui:widget': stringReferenceToTowns },
            mineSectors: {
              'ui:widget': makeMultiSectorSelectorWidget({
                initialLevel: 1,
                canChangeLevel: true,
                extractSectorFromItem: (sector: string) => [sector, 0],
                transformSectorToItem: (sector) => sector[0],
              }),
            },
          },
        }),
      ),
    ],
  },
  makeFileItem(
    'tactical-npc-action-params.json',
    'Tactical Npc Action Params',
    JsonItemsForm,
    {
      name: (item) => item.actionCode.toString(),
    },
  ),
  makeFileItem('vehicles.json', 'Vehicles', JsonItemsForm, {
    name: 'profile',
    uiSchema: {
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
      enterSound: { 'ui:widget': resourceReferenceToSound },
      moveSound: { 'ui:widget': resourceReferenceToSound },
    },
  }),
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
