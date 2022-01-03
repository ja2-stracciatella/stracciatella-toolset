import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Weapons() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="weapons.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
