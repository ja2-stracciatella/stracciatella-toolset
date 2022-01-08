import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { stringReferenceTo } from "../components/form/StringReferenceWidget";

const uiSchema = {
  items: {
    items: {
      "ui:widget": stringReferenceTo("weapons.json", "internalName"),
    },
  },
};

export function ArmyGunChoiceNormal() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="army-gun-choice-normal.json" uiSchema={uiSchema} />
      </EditorLayout>
    </WithOpenMod>
  );
}
