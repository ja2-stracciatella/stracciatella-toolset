import { Typography } from "antd";

import { Layout } from "../components/Layout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Dashboard() {
  return (
    <WithOpenMod>
      <Layout>
        <Typography.Title>Dashboard</Typography.Title>
      </Layout>
    </WithOpenMod>
  );
}
