import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function TacticalNpcActionParams() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="tactical-npc-action-params.json" name="actionCode" />
      </Layout>
    </WithOpenMod>
  );
}
