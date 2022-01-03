import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function Dashboard() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <h1>Dashboard</h1>
      </EditorLayout>
    </WithOpenMod>
  );
}
