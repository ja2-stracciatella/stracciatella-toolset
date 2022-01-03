import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Vehicles() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="vehicles.json" name="profileID" />
      </EditorLayout>
    </WithOpenMod>
  );
}
