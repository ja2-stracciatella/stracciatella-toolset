import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicBloodcatPlacements() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-bloodcat-placements.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
