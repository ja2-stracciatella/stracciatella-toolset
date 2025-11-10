import z from 'zod';
import { toolsetCloseWindowRequestedEventDefinition } from './toolset';

const EVENTS_CHANNEL_PREFIX = 'events';

export type EventDefinition<
  Category extends string,
  Name extends string,
  Payload = unknown,
> = {
  name: `${Category}/${Name}`;
  payloadSchema: z.ZodSchema<Payload>;
};

export type EventName<D> =
  D extends EventDefinition<infer Category, infer Name, unknown>
    ? `${Category}/${Name}`
    : never;

export type EventPayload<D> =
  D extends EventDefinition<string, string, infer Payload> ? Payload : never;

const ALL_EVENTS = [toolsetCloseWindowRequestedEventDefinition] as const;

export type AnyEvent = (typeof ALL_EVENTS)[number];

export type AnyEventName = AnyEvent['name'];

export type EventFromName<Name extends AnyEventName> = Extract<
  AnyEvent,
  { name: Name }
>;

export function getEventByName<Name extends AnyEventName>(
  name: Name,
): EventFromName<Name> {
  return ALL_EVENTS.find((event) => event.name === name) as EventFromName<Name>;
}

export function getEventsChannelByName<Name extends AnyEventName>(
  name: Name,
): string {
  return `${EVENTS_CHANNEL_PREFIX}/${name}`;
}
