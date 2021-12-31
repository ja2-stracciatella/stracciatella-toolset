import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapNpcPlacements() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-npc-placements.json" name="profileId" />
      </Layout>
    </WithOpenMod>
  );
}
