import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Items() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="items.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
