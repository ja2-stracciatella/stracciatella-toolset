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

export function ArmyGunChoiceExtended() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="army-gun-choice-extended.json" uiSchema={uiSchema} />
      </EditorLayout>
    </WithOpenMod>
  );
}
