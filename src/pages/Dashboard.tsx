import { Typography } from "antd";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Dashboard() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <Typography.Title level={2}>Dashboard</Typography.Title>
      </EditorLayout>
    </WithOpenMod>
  );
}
