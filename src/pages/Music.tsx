import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Music() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="music.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
