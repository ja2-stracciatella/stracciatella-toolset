import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSamSites() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-map-sam-sites.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
