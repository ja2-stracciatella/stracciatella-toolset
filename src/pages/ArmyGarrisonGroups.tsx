import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";

export function ArmyGarrisonGroups() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="army-garrison-groups.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
