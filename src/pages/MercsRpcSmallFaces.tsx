import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function MercsRpcSmallFaces() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="mercs-rpc-small-faces.json" name="profileID" />
      </EditorLayout>
    </WithOpenMod>
  );
}
