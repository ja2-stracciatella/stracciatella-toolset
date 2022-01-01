import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSectorsDescriptions() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-sectors-descriptions.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
