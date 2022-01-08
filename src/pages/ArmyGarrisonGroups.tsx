import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { JsonStrategicMapForm } from "../components/StrategicMapForm";
import { stringReferenceTo } from "../components/form/StringReferenceWidget";

const uiSchema = {
  "composition": {
    "ui:widget": stringReferenceTo("army-compositions.json", "name"),
  }
}

export function ArmyGarrisonGroups() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonStrategicMapForm file="army-garrison-groups.json" uiSchema={uiSchema} />
      </EditorLayout>
    </WithOpenMod>
  );
}
