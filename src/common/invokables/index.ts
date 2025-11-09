import z from 'zod';
import { dialogShowOpenDialogInvokableDefinition } from './dialogs';
import {
  toolsetCloseWindowInvokableDefinition,
  toolsetReadConfigInvokableDefinition,
  toolsetUpdateConfigInvokableDefinition,
} from './toolset';
import {
  imageReadMetadataInvokableDefinition,
  imageRenderInvokableDefinition,
} from './images';
import {
  jsonPersistInvokableDefinition,
  jsonReadInvokableDefinition,
} from './jsons';
import {
  modCreateInvokableDefinition,
  modListAvailableInvokableDefinition,
  modListEditableInvokableDefinition,
  modReadSelectedInvokableDefinition,
  modUpdateSelectedInvokableDefinition,
} from './mods';
import { resourcesListInvokableDefinition } from './resources';
import { soundReadInvokableDefinition } from './sounds';

export const INVOKE_CHANNEL = 'invoke';

export type InvokableDefinition<
  Category extends string,
  Name extends string,
  Input = unknown,
  Output = unknown,
> = {
  name: `${Category}/${Name}`;
  inputSchema: z.ZodSchema<Input>;
  outputSchema: z.ZodSchema<Output>;
};

export type InvokableName<D> =
  D extends InvokableDefinition<infer Category, infer Name, unknown, unknown>
    ? `${Category}/${Name}`
    : never;

export type InvokableInput<D> =
  D extends InvokableDefinition<string, string, infer Input, unknown>
    ? Input
    : never;

export type InvokableOutput<D> =
  D extends InvokableDefinition<string, string, unknown, infer Output>
    ? Output
    : never;

const ALL_INVOKABLES = [
  dialogShowOpenDialogInvokableDefinition,

  imageReadMetadataInvokableDefinition,
  imageRenderInvokableDefinition,

  jsonReadInvokableDefinition,
  jsonPersistInvokableDefinition,

  modListAvailableInvokableDefinition,
  modListEditableInvokableDefinition,
  modReadSelectedInvokableDefinition,
  modUpdateSelectedInvokableDefinition,
  modCreateInvokableDefinition,

  resourcesListInvokableDefinition,

  soundReadInvokableDefinition,

  toolsetReadConfigInvokableDefinition,
  toolsetUpdateConfigInvokableDefinition,
  toolsetCloseWindowInvokableDefinition,
];

export type AnyInvokable = (typeof ALL_INVOKABLES)[number];

export type AnyInvokableName = AnyInvokable['name'];

export type InvokableFromName<Name extends AnyInvokableName> = Extract<
  AnyInvokable,
  { name: Name }
>;

export function getInvokableDefinitionByName<Name extends AnyInvokableName>(
  name: Name,
): InvokableFromName<Name> {
  return ALL_INVOKABLES.find(
    (invokable) => invokable.name === name,
  ) as InvokableFromName<Name>;
}
