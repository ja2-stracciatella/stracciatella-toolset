import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";
import { useCallback } from "react";
import { MercPreview } from "../components/content/MercPreview";

export function MercsRpcSmallFaces() {
  const preview = useCallback((item: any) => <MercPreview profileId={item.profileID} />, []);
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="mercs-rpc-small-faces.json" name="profileID" preview={preview} />
      </EditorLayout>
    </WithOpenMod>
  );
}
