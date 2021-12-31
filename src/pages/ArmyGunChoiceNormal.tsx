import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function ArmyGunChoiceNormal() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="army-gun-choice-normal.json" />
      </Layout>
    </WithOpenMod>
  );
}
