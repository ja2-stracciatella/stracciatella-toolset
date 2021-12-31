import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function LoadingScreensMapping() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="loading-screens-mapping.json" name="sector" />
      </Layout>
    </WithOpenMod>
  );
}
