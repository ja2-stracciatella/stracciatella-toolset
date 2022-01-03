import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function MercsMERCListings() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="mercs-MERC-listings.json" name="profileID" />
      </EditorLayout>
    </WithOpenMod>
  );
}
