import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { useCallback } from "react";
import { ItemPreview } from "../components/content/ItemPreview";

export function Weapons() {
  const preview = useCallback((item: any) => <ItemPreview graphicType={item.ubGraphicType} graphicIndex={item.ubGraphicNum} />, []);
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="weapons.json" name="internalName" preview={preview}/>
      </EditorLayout>
    </WithOpenMod>
  );
}
