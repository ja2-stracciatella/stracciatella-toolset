import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

function getItemReplacementName(item: any): string {
    return `${item.from} to ${item.to}`;
}

export function TacticalMapItemReplacements() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="tactical-map-item-replacements.json" name={getItemReplacementName} />
      </Layout>
    </WithOpenMod>
  );
}
