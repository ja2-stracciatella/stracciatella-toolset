import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Dealers() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="dealers.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
