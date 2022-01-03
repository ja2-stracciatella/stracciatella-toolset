import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function LoadingScreensMapping() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="loading-screens-mapping.json" name="sector" />
      </EditorLayout>
    </WithOpenMod>
  );
}
