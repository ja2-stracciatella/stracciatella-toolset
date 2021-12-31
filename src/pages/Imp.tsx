import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Imp() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="imp.json" />
      </Layout>
    </WithOpenMod>
  );
}
