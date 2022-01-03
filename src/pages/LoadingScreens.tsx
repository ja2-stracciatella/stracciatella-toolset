import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function LoadingScreens() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="loading-screens.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
