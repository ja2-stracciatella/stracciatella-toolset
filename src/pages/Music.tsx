import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Music() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="music.json" />
      </Layout>
    </WithOpenMod>
  );
}
