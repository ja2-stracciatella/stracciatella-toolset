import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function StrategicMapSectorsDescriptions() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="strategic-map-sectors-descriptions.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
