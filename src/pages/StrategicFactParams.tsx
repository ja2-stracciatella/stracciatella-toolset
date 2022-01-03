import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicFactParams() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-fact-params.json" name="fact" />
      </EditorLayout>
    </WithOpenMod>
  );
}
