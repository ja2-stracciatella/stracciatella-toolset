import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSamSitesAirControl() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="strategic-map-sam-sites-air-control.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
