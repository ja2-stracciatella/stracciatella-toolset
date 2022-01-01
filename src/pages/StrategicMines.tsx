import { JsonItemsForm } from "../components/JsonItemsForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMines() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonItemsForm file="strategic-mines.json" name="entranceSector" />
      </Layout>
    </WithOpenMod>
  );
}
