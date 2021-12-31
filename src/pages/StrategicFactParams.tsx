import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicFactParams() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-fact-params.json" name="fact" />
      </Layout>
    </WithOpenMod>
  );
}
