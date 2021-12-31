import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Weapons() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="weapons.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
