import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function LoadingScreensMapping() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="loading-screens-mapping.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
