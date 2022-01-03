import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMines() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-mines.json" name="entranceSector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
