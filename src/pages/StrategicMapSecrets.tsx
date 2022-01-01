import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSecrets() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-secrets.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
