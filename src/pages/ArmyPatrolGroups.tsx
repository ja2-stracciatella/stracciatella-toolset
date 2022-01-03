import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

function getPatrolGroupName(item: any): string {
    return item.points.join(", ");
}

export function ArmyPatrolGroups() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="army-patrol-groups.json" name={getPatrolGroupName} />
      </EditorLayout>
    </WithOpenMod>
  );
}
