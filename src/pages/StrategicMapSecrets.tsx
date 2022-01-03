import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function StrategicMapSecrets() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="strategic-map-secrets.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
