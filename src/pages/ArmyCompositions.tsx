import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyCompositions() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="army-compositions.json" name="name" />
      </Layout>
    </WithOpenMod>
  );
}
