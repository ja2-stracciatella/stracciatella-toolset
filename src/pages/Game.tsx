import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Game() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="game.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
