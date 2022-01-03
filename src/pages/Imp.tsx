import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Imp() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="imp.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
