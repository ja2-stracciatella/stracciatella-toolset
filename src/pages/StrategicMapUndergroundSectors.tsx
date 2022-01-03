import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapUndergroundSectors() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-map-underground-sectors.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
