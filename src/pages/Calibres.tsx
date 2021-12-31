import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Calibres() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="calibres.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
