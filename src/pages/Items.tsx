import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { useCallback } from "react";
import { ItemPreview } from "../components/content/ItemPreview";

export function Items() {
  const preview = useCallback((item: any) => <ItemPreview inventoryGraphics={item.inventoryGraphics} />, []);
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="items.json" name="internalName" preview={preview} />
      </EditorLayout>
    </WithOpenMod>
  );
}
