import { useEffect } from 'react';
import {
  AnyEventName,
  EventFromName,
  EventPayload,
  getEventByName,
} from '../../../common/events';

export function Listen<Name extends AnyEventName>({
  name,
  callback,
}: {
  name: Name;
  callback: (payload: EventPayload<EventFromName<Name>>) => void;
}) {
  useEffect(() => {
    const event = getEventByName(name);

    window.electronAPI.onMainEvent(`event/${name}`, async (data) => {
      const payload = await event.payloadSchema.parseAsync(data);
      callback(payload as EventPayload<EventFromName<Name>>);
    });
  }, [callback, name]);
  return null;
}
