import z from 'zod';
import { EventDefinition } from '.';

type Category = 'toolset';

export type ToolsetCloseWindowRequestedEvent = EventDefinition<
  Category,
  'closeWindowRequested',
  null
>;

export const toolsetCloseWindowRequestedEventDefinition: ToolsetCloseWindowRequestedEvent =
  {
    name: 'toolset/closeWindowRequested',
    payloadSchema: z.null(),
  };
