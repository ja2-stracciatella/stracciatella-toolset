import { BrowserWindow } from 'electron/main';
import { AnyEventName, EventFromName, EventPayload } from 'src/common/events';

export function sendEventToRenderer<Name extends AnyEventName>(
  window: BrowserWindow,
  name: Name,
  payload: EventPayload<EventFromName<Name>>,
) {
  window.webContents.send(`event/${name}`, payload);
}
