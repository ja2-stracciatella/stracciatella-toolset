import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSamSites() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-sam-sites.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
