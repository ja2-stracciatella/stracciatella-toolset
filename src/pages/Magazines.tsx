import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Magazines() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="magazines.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
