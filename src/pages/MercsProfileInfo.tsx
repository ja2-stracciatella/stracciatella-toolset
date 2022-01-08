import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { MercPreview } from "../components/content/MercPreview";
import { useCallback } from "react";

export function MercsProfileInfo() {
  const preview = useCallback((item: any) => <MercPreview profileId={item.profileID} />, []);
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="mercs-profile-info.json" name="internalName" preview={preview} />
      </EditorLayout>
    </WithOpenMod>
  );
}
