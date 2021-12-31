import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

function getCreatureLairName(item: any): string {
    return item.entranceSector[0];
}

export function StrategicMapCreatureLairs() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-map-creature-lairs.json" name={getCreatureLairName} />
      </Layout>
    </WithOpenMod>
  );
}
