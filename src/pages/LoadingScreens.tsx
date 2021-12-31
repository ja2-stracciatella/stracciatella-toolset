import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function LoadingScreens() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="loading-screens.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
