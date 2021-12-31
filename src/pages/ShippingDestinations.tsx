import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ShippingDestinations() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="shipping-destinations.json" name="locationID" />
      </Layout>
    </WithOpenMod>
  );
}
