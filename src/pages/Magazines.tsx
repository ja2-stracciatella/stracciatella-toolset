import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { ItemPreview } from "../components/content/ItemPreview";
import { useCallback } from "react";

export function Magazines() {
  const preview = useCallback((item: any) => <ItemPreview inventoryGraphics={item.inventoryGraphics} />, []);
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="magazines.json" name="internalName" preview={preview} />
      </EditorLayout>
    </WithOpenMod>
  );
}
