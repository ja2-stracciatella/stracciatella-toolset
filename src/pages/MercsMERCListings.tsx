import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function MercsMERCListings() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="mercs-MERC-listings.json" name="profileID" />
      </Layout>
    </WithOpenMod>
  );
}
