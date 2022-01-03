import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicBloodcatPlacements() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-bloodcat-placements.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
