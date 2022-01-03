import { JsonItemsForm } from "../components/JsonItemsForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

function getCreatureLairName(item: any): string {
    return item.entranceSector[0];
}

export function StrategicMapCreatureLairs() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonItemsForm file="strategic-map-creature-lairs.json" name={getCreatureLairName} />
      </EditorLayout>
    </WithOpenMod>
  );
}
