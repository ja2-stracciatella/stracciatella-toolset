import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapUndergroundSectors() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-underground-sectors.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
