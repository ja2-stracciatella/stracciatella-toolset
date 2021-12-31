import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Vehicles() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="vehicles.json" name="profileID" />
      </Layout>
    </WithOpenMod>
  );
}
