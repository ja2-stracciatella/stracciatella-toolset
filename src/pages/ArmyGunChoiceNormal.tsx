import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyGunChoiceNormal() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="army-gun-choice-normal.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
