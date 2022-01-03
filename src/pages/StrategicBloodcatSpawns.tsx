import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicBloodcatSpawns() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-bloodcat-spawns.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
