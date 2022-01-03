import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapTraversibilityRatings() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="strategic-map-traversibility-ratings.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
