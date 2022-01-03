import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Items() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="items.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
