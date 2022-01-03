import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function StrategicMapSamSites() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="strategic-map-sam-sites.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
