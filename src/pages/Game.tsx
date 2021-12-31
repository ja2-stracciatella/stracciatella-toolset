import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Game() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="game.json" />
      </Layout>
    </WithOpenMod>
  );
}
