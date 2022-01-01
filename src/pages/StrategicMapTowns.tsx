import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapTowns() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-towns.json" name="townId" />
      </Layout>
    </WithOpenMod>
  );
}
