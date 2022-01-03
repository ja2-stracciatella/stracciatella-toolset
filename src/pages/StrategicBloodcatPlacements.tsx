import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function StrategicBloodcatPlacements() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="strategic-bloodcat-placements.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
