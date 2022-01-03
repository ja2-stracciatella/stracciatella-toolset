import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyGunChoiceExtended() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="army-gun-choice-extended.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
