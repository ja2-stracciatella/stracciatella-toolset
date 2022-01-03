import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyGarrisonGroups() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="army-garrison-groups.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
