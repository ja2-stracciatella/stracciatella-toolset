import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicBloodcatSpawns() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-bloodcat-spawns.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
