import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapMovementCosts() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="strategic-map-movement-costs.json" />
      </Layout>
    </WithOpenMod>
  );
}
