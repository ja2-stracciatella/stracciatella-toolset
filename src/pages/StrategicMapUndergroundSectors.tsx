import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function StrategicMapUndergroundSectors() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="strategic-map-underground-sectors.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
