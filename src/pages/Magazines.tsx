import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Magazines() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="magazines.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
