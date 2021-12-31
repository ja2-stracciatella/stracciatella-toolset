import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicAIPolicy() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="strategic-ai-policy.json" />
      </Layout>
    </WithOpenMod>
  );
}
