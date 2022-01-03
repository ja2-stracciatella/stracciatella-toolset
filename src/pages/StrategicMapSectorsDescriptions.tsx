import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSectorsDescriptions() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-map-sectors-descriptions.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
