import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapMovementCosts() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="strategic-map-movement-costs.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
