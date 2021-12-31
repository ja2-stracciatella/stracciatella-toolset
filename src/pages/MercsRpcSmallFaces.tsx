import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function MercsRpcSmallFaces() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="mercs-rpc-small-faces.json" name="profileID" />
      </Layout>
    </WithOpenMod>
  );
}
