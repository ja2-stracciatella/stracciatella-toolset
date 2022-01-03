import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Dealers() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="dealers.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
