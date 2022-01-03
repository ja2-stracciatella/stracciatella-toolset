import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ShippingDestinations() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="shipping-destinations.json" name="locationId" />
      </EditorLayout>
    </WithOpenMod>
  );
}
