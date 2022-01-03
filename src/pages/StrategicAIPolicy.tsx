import { JsonForm } from "../components/JsonForm";

import { EditorLayout } from "../components/EditorLayout";
import { WithOpenMod } from "../components/WithOpenMod";

export function StrategicAIPolicy() {
  return (
    <WithOpenMod>
      <EditorLayout>
        <JsonForm file="strategic-ai-policy.json" />
      </EditorLayout>
    </WithOpenMod>
  );
}
