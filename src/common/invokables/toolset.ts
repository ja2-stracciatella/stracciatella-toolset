import z from 'zod';
import { InvokableDefinition } from '.';

const PARTIAL_TOOLSET_CONFIG_SCHEMA = z.object({
  stracciatellaHome: z.nullable(z.string()),
  vanillaGameDir: z.nullable(z.string()),
  stracciatellaInstallDir: z.nullable(z.string()),
  lastSelectedMod: z.nullable(z.string()),
});

export type PartialToolsetConfig = z.infer<
  typeof PARTIAL_TOOLSET_CONFIG_SCHEMA
>;

export const FULL_TOOLSET_CONFIG_SCHEMA = z.object({
  stracciatellaHome: z.string().min(1).meta({
    title: 'JA2 Stracciatella Home Directory',
    description:
      'This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your JA2 Stracciatella configuration and mods.',
  }),
  vanillaGameDir: z.string().min(1).meta({
    title: 'Vanilla Game Directory',
    description:
      'This should be auto-populated if you have JA2 Stracciatella installed and started it successfully at least once. It is the location of your vanilla (original) Jagged Alliance 2 installation.',
  }),
  stracciatellaInstallDir: z.string().min(1).meta({
    title: 'JA2 Stracciatella Directory',
    description:
      'This should point at your JA2 Straccciatella install directory. It is used to read the JSON files included with the game.',
  }),
  lastSelectedMod: z.nullable(z.string().min(1)).meta({
    title: 'Last Selected Mod',
    description: 'The last mod that was selected in the mod selection dialog.',
  }),
});

export type FullToolsetConfig = z.infer<typeof FULL_TOOLSET_CONFIG_SCHEMA>;

export const TOOLSET_CONFIG_SCHEMA = z.union([
  z.object({
    partial: z.literal(true),
    config: PARTIAL_TOOLSET_CONFIG_SCHEMA,
  }),
  z.object({
    partial: z.literal(false),
    config: FULL_TOOLSET_CONFIG_SCHEMA,
  }),
]);

export type ToolsetConfig = z.infer<typeof TOOLSET_CONFIG_SCHEMA>;

type Category = 'toolset';

export type ToolsetReadConfigInvokable = InvokableDefinition<
  Category,
  'readConfig',
  null,
  ToolsetConfig
>;

export const toolsetReadConfigInvokableDefinition: ToolsetReadConfigInvokable =
  {
    name: 'toolset/readConfig',
    inputSchema: z.null(),
    outputSchema: TOOLSET_CONFIG_SCHEMA,
  };

export type ToolsetUpdateConfigInvokable = InvokableDefinition<
  Category,
  'updateConfig',
  PartialToolsetConfig,
  ToolsetConfig
>;

export const toolsetUpdateConfigInvokableDefinition: ToolsetUpdateConfigInvokable =
  {
    name: 'toolset/updateConfig',
    inputSchema: PARTIAL_TOOLSET_CONFIG_SCHEMA,
    outputSchema: TOOLSET_CONFIG_SCHEMA,
  };

export type ToolsetCloseWindowInvokable = InvokableDefinition<
  Category,
  'closeWindow',
  null,
  unknown
>;

export const toolsetCloseWindowInvokableDefinition: ToolsetCloseWindowInvokable =
  {
    name: 'toolset/closeWindow',
    inputSchema: z.null(),
    outputSchema: z.unknown(),
  };
