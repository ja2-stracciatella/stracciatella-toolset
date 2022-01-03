import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function MercsProfileInfo() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="mercs-profile-info.json" name="internalName" />
      </EditorLayout>
    </WithOpenMod>
  );
}
