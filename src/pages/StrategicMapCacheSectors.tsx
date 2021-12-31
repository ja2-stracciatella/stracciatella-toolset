import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapCacheSectors() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="strategic-map-cache-sectors.json" />
      </Layout>
    </WithOpenMod>
  );
}
