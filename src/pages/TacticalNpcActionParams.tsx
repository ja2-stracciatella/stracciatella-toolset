import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function TacticalNpcActionParams() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="tactical-npc-action-params.json" name="actionCode" />
      </EditorLayout>
    </WithOpenMod>
  );
}
