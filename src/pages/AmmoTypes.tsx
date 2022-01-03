import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function AmmoTypes() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="ammo-types.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
