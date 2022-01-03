import { EditorLayout } from "../components/EditorLayout";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicBloodcatSpawns() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="strategic-bloodcat-spawns.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
