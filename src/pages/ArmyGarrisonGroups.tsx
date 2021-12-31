import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyGarrisonGroups() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="army-garrison-groups.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
