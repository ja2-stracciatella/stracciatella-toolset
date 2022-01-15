import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { useCallback } from "react";
import { MercPreview } from "../components/content/MercPreview";

export function Dealers() {
  const preview = useCallback((item: any) => <MercPreview profile={item.profile} />, []);
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="dealers.json" name="profile" preview={preview} />
      </EditorLayout>
    </WithOpenMod>
  );
}
