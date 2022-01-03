import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapCacheSectors() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="strategic-map-cache-sectors.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
