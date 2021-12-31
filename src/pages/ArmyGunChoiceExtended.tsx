import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyGunChoiceExtended() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="army-gun-choice-extended.json" />
      </Layout>
    </WithOpenMod>
  );
}
