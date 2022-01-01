import { JsonForm } from "../components/JsonForm";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicMapSamSitesAirControl() {
  return (
    <WithOpenMod>
      <Layout>
        <JsonForm file="strategic-map-sam-sites-air-control.json" />
      </Layout>
    </WithOpenMod>
  );
}
