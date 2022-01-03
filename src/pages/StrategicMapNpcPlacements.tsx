import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapNpcPlacements() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-map-npc-placements.json" name="profileId" />
      </EditorLayout>
    </WithOpenMod>
  );
}
