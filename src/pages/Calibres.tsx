import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Calibres() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="calibres.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
