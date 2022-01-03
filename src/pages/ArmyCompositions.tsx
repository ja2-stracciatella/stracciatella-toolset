import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyCompositions() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="army-compositions.json" name="name" />
      </EditorLayout>
    </WithOpenMod>
  );
}
