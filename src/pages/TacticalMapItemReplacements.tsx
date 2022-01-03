import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

function getItemReplacementName(item: any): string {
    return `${item.from} to ${item.to}`;
}

export function TacticalMapItemReplacements() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="tactical-map-item-replacements.json" name={getItemReplacementName} />
      </EditorLayout>
    </WithOpenMod>
  );
}
