import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function AmmoTypes() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="ammo-types.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
