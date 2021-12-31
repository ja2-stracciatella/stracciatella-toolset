import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function MercsProfileInfo() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="mercs-profile-info.json" name="internalName" />
      </Layout>
    </WithOpenMod>
  );
}
