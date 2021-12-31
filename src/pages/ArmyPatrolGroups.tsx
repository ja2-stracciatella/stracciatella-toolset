import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

function getPatrolGroupName(item: any): string {
    return item.points.join(", ");
}

export function ArmyPatrolGroups() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="army-patrol-groups.json" name={getPatrolGroupName} />
      </Layout>
    </WithOpenMod>
  );
}
